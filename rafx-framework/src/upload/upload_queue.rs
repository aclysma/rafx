use crate::upload::image_upload::ImageUploadParams;
use crate::upload::GpuImageData;
use crate::upload::{buffer_upload, image_upload};
use crate::{BufferResource, ResourceArc};
use crossbeam_channel::{Receiver, Sender};
use rafx_api::{
    extra::upload::*, RafxBuffer, RafxDeviceContext, RafxError, RafxQueue, RafxResourceType,
    RafxResult, RafxTexture,
};

pub trait UploadOp<ResourceT>: Send + Sync {
    fn complete(
        self: Box<Self>,
        resource: ResourceT,
    );

    fn error(
        self: Box<Self>,
        error: RafxError,
    );
}

type ImageUploadOp = Box<dyn UploadOp<RafxTexture>>;
type BufferUploadOp = Box<dyn UploadOp<RafxBuffer>>;
type ExistingResourceUploadOp = Box<dyn UploadOp<()>>;

//
// Represents a single request inserted into the upload queue that hasn't started yet
//
//TODO: Make a helper object that carries an Arc<Receiver> that can be called
struct PendingImageUpload {
    upload_op: ImageUploadOp,
    image_data: GpuImageData,
    resource_type: RafxResourceType,
    generate_mips: bool,
}

struct PendingBufferUpload {
    upload_op: BufferUploadOp,
    resource_type: RafxResourceType,
    data: Vec<u8>,
}

struct PendingExistingBufferUpload {
    upload_op: ExistingResourceUploadOp,
    resource_type: RafxResourceType,
    data: Vec<u8>,
    dst_buffer: ResourceArc<BufferResource>,
    dst_byte_offset: u64,
}

enum PendingUpload {
    Image(PendingImageUpload),
    Buffer(PendingBufferUpload),
    ExistingBuffer(PendingExistingBufferUpload),
}

impl PendingUpload {
    // Ok(None) = upload enqueue
    // Ok(Some) = upload not enqueued because there was not enough room
    // Err = Vulkan error
    fn try_enqueue_image_upload(
        device_context: &RafxDeviceContext,
        upload: &mut RafxTransferUpload,
        pending_image: PendingImageUpload,
        in_flight_uploads: &mut Vec<InFlightUpload>,
    ) -> RafxResult<Option<PendingImageUpload>> {
        let result = image_upload::enqueue_load_image(
            device_context,
            upload,
            // self.transfer_queue.queue_family_index(),
            // self.graphics_queue.queue_family_index(),
            &pending_image.image_data,
            ImageUploadParams {
                resource_type: pending_image.resource_type,
                generate_mips: pending_image.generate_mips,
                ..Default::default()
            },
        );

        match result {
            Ok(texture) => {
                in_flight_uploads.push(InFlightUpload::Image(InFlightImageUpload {
                    texture,
                    upload_op: pending_image.upload_op,
                }));
                Ok(None)
            }
            Err(RafxUploadError::Other(e)) => Err(e),
            Err(RafxUploadError::BufferFull) => Ok(Some(pending_image)),
        }
    }

    // Ok(None) = upload enqueue
    // Ok(Some) = upload not enqueued because there was not enough room
    // Err = Vulkan error
    fn try_enqueue_buffer_upload(
        device_context: &RafxDeviceContext,
        upload: &mut RafxTransferUpload,
        pending_buffer: PendingBufferUpload,
        in_flight_uploads: &mut Vec<InFlightUpload>,
    ) -> RafxResult<Option<PendingBufferUpload>> {
        let result = buffer_upload::enqueue_load_buffer(
            device_context,
            upload,
            pending_buffer.resource_type,
            // self.transfer_queue.queue_family_index(),
            // self.graphics_queue.queue_family_index(),
            &pending_buffer.data,
            None,
            0,
        );

        match result {
            Ok(buffer) => {
                // We created a buffer, so this should always be Some
                let buffer = buffer.unwrap();
                in_flight_uploads.push(InFlightUpload::Buffer(InFlightBufferUpload {
                    buffer,
                    upload_op: pending_buffer.upload_op,
                }));
                Ok(None)
            }
            Err(RafxUploadError::Other(e)) => Err(e),
            Err(RafxUploadError::BufferFull) => Ok(Some(pending_buffer)),
        }
    }

    // Ok(None) = upload enqueue
    // Ok(Some) = upload not enqueued because there was not enough room
    // Err = Vulkan error
    fn try_enqueue_existing_buffer_upload(
        device_context: &RafxDeviceContext,
        upload: &mut RafxTransferUpload,
        pending_buffer: PendingExistingBufferUpload,
        in_flight_uploads: &mut Vec<InFlightUpload>,
    ) -> RafxResult<Option<PendingExistingBufferUpload>> {
        let result = buffer_upload::enqueue_load_buffer(
            device_context,
            upload,
            pending_buffer.resource_type,
            // self.transfer_queue.queue_family_index(),
            // self.graphics_queue.queue_family_index(),
            &pending_buffer.data,
            Some(&*pending_buffer.dst_buffer.get_raw().buffer),
            pending_buffer.dst_byte_offset,
        );

        match result {
            Ok(buffer) => {
                //It should be none, we used an existing buffer
                assert!(buffer.is_none());
                in_flight_uploads.push(InFlightUpload::ExistingResource(
                    InFlightExistingResourceUpload {
                        upload_op: pending_buffer.upload_op,
                    },
                ));
                Ok(None)
            }
            Err(RafxUploadError::Other(e)) => Err(e),
            Err(RafxUploadError::BufferFull) => Ok(Some(pending_buffer)),
        }
    }

    fn try_enqueue_upload(
        self,
        device_context: &RafxDeviceContext,
        upload: &mut RafxTransferUpload,
        in_flight_uploads: &mut Vec<InFlightUpload>,
    ) -> RafxResult<Option<Self>> {
        Ok(match self {
            PendingUpload::Image(pending_upload) => Self::try_enqueue_image_upload(
                device_context,
                upload,
                pending_upload,
                in_flight_uploads,
            )?
            .map(|x| PendingUpload::Image(x)),
            PendingUpload::Buffer(pending_upload) => Self::try_enqueue_buffer_upload(
                device_context,
                upload,
                pending_upload,
                in_flight_uploads,
            )?
            .map(|x| PendingUpload::Buffer(x)),
            PendingUpload::ExistingBuffer(pending_upload) => {
                Self::try_enqueue_existing_buffer_upload(
                    device_context,
                    upload,
                    pending_upload,
                    in_flight_uploads,
                )?
                .map(|x| PendingUpload::ExistingBuffer(x))
            }
        })
    }

    fn required_bytes(
        &self,
        device_context: &RafxDeviceContext,
    ) -> usize {
        match self {
            PendingUpload::Image(image) => {
                let device_info = device_context.device_info();
                image.image_data.total_size(
                    device_info.upload_texture_alignment,
                    device_info.upload_texture_row_alignment,
                ) as usize
            }
            PendingUpload::Buffer(buffer) => buffer.data.len(),
            PendingUpload::ExistingBuffer(buffer) => buffer.data.len(),
        }
    }
}

//
// Represents a single request that the upload queue has started
//
struct InFlightImageUpload {
    upload_op: ImageUploadOp,
    texture: RafxTexture,
}

struct InFlightBufferUpload {
    upload_op: BufferUploadOp,
    buffer: RafxBuffer,
}

struct InFlightExistingResourceUpload {
    upload_op: ExistingResourceUploadOp,
}

enum InFlightUpload {
    Image(InFlightImageUpload),
    Buffer(InFlightBufferUpload),
    ExistingResource(InFlightExistingResourceUpload),
}

impl InFlightUpload {
    fn complete(self) {
        match self {
            InFlightUpload::Image(image) => {
                image.upload_op.complete(image.texture);
            }
            InFlightUpload::Buffer(buffer) => {
                buffer.upload_op.complete(buffer.buffer);
            }
            InFlightUpload::ExistingResource(existing) => {
                existing.upload_op.complete(());
            }
        }
    }

    fn error(
        self,
        error: RafxError,
    ) {
        match self {
            InFlightUpload::Image(image) => {
                image.upload_op.error(error);
                // image.texture is dropped here
            }
            InFlightUpload::Buffer(buffer) => {
                buffer.upload_op.error(error);
                // image.buffer is dropped here
            }
            InFlightUpload::ExistingResource(existing) => {
                existing.upload_op.error(error);
            }
        }
    }
}

//
// Represents a batch of requests that has been started, contains multiple InFlightImageUpload and
// InFlightBufferUploads
//

// The result from polling a single upload (which may contain multiple images in it)
enum InProgressUploadBatchPollResult {
    Pending,
    Complete,
    Error,
    Destroyed,
}

// This is an inner of InProgressImageUpload - it is wrapped in a Option to avoid borrowing issues
// when polling by allowing us to temporarily take ownership of it and then put it back
struct InProgressUploadBatchInner {
    in_flight_uploads: Vec<InFlightUpload>,
    upload: RafxTransferUpload,
}

struct InProgressUploadBatchDebugInfo {
    upload_id: usize,
    start_time: rafx_base::Instant,
    size: u64,
    resource_count: usize,
}

// A single upload which may contain multiple images
struct InProgressUploadBatch {
    // Only valid if the upload is actually in progress
    inner: Option<InProgressUploadBatchInner>,
    debug_info: InProgressUploadBatchDebugInfo,
}

impl InProgressUploadBatch {
    pub fn new(
        in_flight_uploads: Vec<InFlightUpload>,
        upload: RafxTransferUpload,
        debug_info: InProgressUploadBatchDebugInfo,
    ) -> Self {
        let inner = InProgressUploadBatchInner {
            in_flight_uploads,
            upload,
        };

        InProgressUploadBatch {
            inner: Some(inner),
            debug_info,
        }
    }

    // The main state machine for an upload:
    // - Submits on the transfer queue and waits
    // - Submits on the graphics queue and waits
    //
    // Calls upload_op.complete() or upload_op.error() as appropriate
    pub fn poll_load(&mut self) -> InProgressUploadBatchPollResult {
        loop {
            if let Some(mut inner) = self.take_inner() {
                match inner.upload.state() {
                    Ok(state) => match state {
                        RafxTransferUploadState::Writable => {
                            //log::trace!("RafxTransferUploadState::Writable");
                            inner.upload.submit_transfer().unwrap();
                            self.inner = Some(inner);
                        }
                        RafxTransferUploadState::SentToTransferQueue => {
                            //log::trace!("RafxTransferUploadState::SentToTransferQueue");
                            self.inner = Some(inner);
                            break InProgressUploadBatchPollResult::Pending;
                        }
                        RafxTransferUploadState::PendingSubmitDstQueue => {
                            //log::trace!("RafxTransferUploadState::PendingSubmitDstQueue");
                            inner.upload.submit_dst().unwrap();
                            self.inner = Some(inner);
                        }
                        RafxTransferUploadState::SentToDstQueue => {
                            //log::trace!("RafxTransferUploadState::SentToDstQueue");
                            self.inner = Some(inner);
                            break InProgressUploadBatchPollResult::Pending;
                        }
                        RafxTransferUploadState::Complete => {
                            //log::trace!("RafxTransferUploadState::Complete");
                            for in_flight_upload in inner.in_flight_uploads {
                                in_flight_upload.complete();
                            }

                            break InProgressUploadBatchPollResult::Complete;
                        }
                    },
                    Err(err) => {
                        for in_flight_upload in inner.in_flight_uploads {
                            in_flight_upload.error(err.clone());
                        }

                        break InProgressUploadBatchPollResult::Error;
                    }
                }
            } else {
                break InProgressUploadBatchPollResult::Destroyed;
            }
        }
    }

    // Allows taking ownership of the inner object
    fn take_inner(&mut self) -> Option<InProgressUploadBatchInner> {
        let mut inner = None;
        std::mem::swap(&mut self.inner, &mut inner);
        inner
    }
}

impl Drop for InProgressUploadBatch {
    fn drop(&mut self) {
        if let Some(mut inner) = self.take_inner() {
            // I don't think order of destruction matters but just in case
            inner.in_flight_uploads.clear();
        }
    }
}

pub struct UploadQueueConfig {
    pub max_bytes_per_upload: usize,
    pub max_concurrent_uploads: usize,
    pub max_new_uploads_in_single_frame: usize,
}

//
// Receives sets of images/buffers that need to be uploaded and kicks off the uploads in batches
//
pub struct UploadQueue {
    device_context: RafxDeviceContext,
    config: UploadQueueConfig,

    pending_upload_tx: Sender<PendingUpload>,
    pending_upload_rx: Receiver<PendingUpload>,
    // If we fail to upload due to size limitation, keep the failed upload here to retry later
    next_upload: Option<PendingUpload>,

    // These are uploads that are currently in progress
    uploads_in_progress: Vec<InProgressUploadBatch>,

    upload_buffer_pool: RafxUploadBufferPool,

    graphics_queue: RafxQueue,
    transfer_queue: RafxQueue,

    next_upload_id: usize,
}

impl UploadQueue {
    pub fn new(
        device_context: &RafxDeviceContext,
        config: UploadQueueConfig,
        graphics_queue: RafxQueue,
        transfer_queue: RafxQueue,
    ) -> RafxResult<Self> {
        let (pending_upload_tx, pending_upload_rx) = crossbeam_channel::unbounded();
        let upload_buffer_pool = RafxUploadBufferPool::new(
            device_context,
            config.max_concurrent_uploads as u32,
            config.max_bytes_per_upload as u64,
        )?;

        Ok(UploadQueue {
            device_context: device_context.clone(),
            config,
            upload_buffer_pool,
            pending_upload_tx,
            pending_upload_rx,
            next_upload: None,
            uploads_in_progress: Default::default(),
            next_upload_id: 1,
            graphics_queue,
            transfer_queue,
        })
    }

    pub fn upload_queue_context(&self) -> UploadQueueContext {
        UploadQueueContext {
            pending_upload_tx: self.pending_upload_tx.clone(),
        }
    }

    pub fn update(&mut self) -> RafxResult<()> {
        self.start_new_upload_batches()?;
        self.update_existing_upload_batches();
        Ok(())
    }

    fn gather_pending_uploads_for_single_upload_batch(
        &mut self,
        upload: &mut RafxTransferUpload,
    ) -> RafxResult<Vec<InFlightUpload>> {
        let mut in_flight_uploads = vec![];

        // If we had a pending image upload from before, try to upload it now
        self.next_upload = if let Some(next_upload) = self.next_upload.take() {
            next_upload.try_enqueue_upload(&self.device_context, upload, &mut in_flight_uploads)?
        } else {
            None
        };

        // The first image we tried to upload failed. Log an error since we aren't making forward progress
        if let Some(next_upload) = &self.next_upload {
            log::error!(
                "Resource of {} bytes has repeatedly exceeded the available room in the upload buffer. ({} of {} bytes free)",
                next_upload.required_bytes(&upload.dst_queue().device_context()),
                upload.bytes_free(),
                upload.buffer_size()
            );
            return Ok(vec![]);
        }

        let rx = self.pending_upload_rx.clone();
        for pending_upload in rx.try_iter() {
            self.next_upload = pending_upload.try_enqueue_upload(
                &self.device_context,
                upload,
                &mut in_flight_uploads,
            )?;

            if let Some(next_upload) = &self.next_upload {
                log::debug!(
                    "Resource of {} bytes exceeds the available room in the upload buffer. ({} of {} bytes free)",
                    next_upload.required_bytes(&upload.dst_queue().device_context()),
                    upload.bytes_free(),
                    upload.buffer_size(),
                );
                break;
            }
        }

        Ok(in_flight_uploads)
    }

    fn try_start_single_upload_batch(&mut self) -> RafxResult<bool> {
        let mut upload = RafxTransferUpload::new(
            &self.device_context,
            &self.transfer_queue,
            &self.graphics_queue,
            self.config.max_bytes_per_upload as u64,
            Some(&mut self.upload_buffer_pool),
        )?;

        let in_flight_uploads = self.gather_pending_uploads_for_single_upload_batch(&mut upload)?;

        if !in_flight_uploads.is_empty() {
            let upload_id = self.next_upload_id;
            self.next_upload_id += 1;

            log::debug!(
                "Submitting {} byte upload with {} uploads, UploadId = {}",
                upload.bytes_written(),
                in_flight_uploads.len(),
                upload_id
            );

            upload.submit_transfer()?;

            let debug_info = InProgressUploadBatchDebugInfo {
                upload_id,
                resource_count: in_flight_uploads.len(),
                size: upload.bytes_written(),
                start_time: rafx_base::Instant::now(),
            };

            self.uploads_in_progress.push(InProgressUploadBatch::new(
                in_flight_uploads,
                upload,
                debug_info,
            ));

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn start_new_upload_batches(&mut self) -> RafxResult<()> {
        for _ in 0..self.config.max_new_uploads_in_single_frame {
            if self.pending_upload_rx.is_empty() && self.next_upload.is_none() {
                return Ok(());
            }

            if self.uploads_in_progress.len() >= self.config.max_concurrent_uploads {
                log::trace!(
                    "Max number of uploads already in progress. Waiting to start a new one"
                );
                return Ok(());
            }

            if !self.try_start_single_upload_batch()? {
                return Ok(());
            }
        }

        Ok(())
    }

    fn update_existing_upload_batches(&mut self) {
        // iterate backwards so we can use swap_remove
        for i in (0..self.uploads_in_progress.len()).rev() {
            let result = self.uploads_in_progress[i].poll_load();
            match result {
                InProgressUploadBatchPollResult::Pending => {
                    // do nothing
                }
                InProgressUploadBatchPollResult::Complete => {
                    //load_op.complete() is called by poll_load

                    let debug_info = &self.uploads_in_progress[i].debug_info;
                    log::debug!(
                        "Completed {} byte upload with {} resources in {} ms, UploadId = {}",
                        debug_info.size,
                        debug_info.resource_count,
                        debug_info.start_time.elapsed().as_secs_f32(),
                        debug_info.upload_id
                    );

                    self.uploads_in_progress.swap_remove(i);
                }
                InProgressUploadBatchPollResult::Error => {
                    //load_op.error() is called by poll_load

                    let debug_info = &self.uploads_in_progress[i].debug_info;
                    log::error!(
                        "Failed {} byte upload with {} resources in {} ms, UploadId = {}",
                        debug_info.size,
                        debug_info.resource_count,
                        debug_info.start_time.elapsed().as_secs_f32(),
                        debug_info.upload_id
                    );

                    self.uploads_in_progress.swap_remove(i);
                }
                InProgressUploadBatchPollResult::Destroyed => {
                    // not expected - this only occurs if polling the upload when it is already in a complete or error state
                    unreachable!();
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct UploadQueueContext {
    pending_upload_tx: Sender<PendingUpload>,
}

impl UploadQueueContext {
    pub fn upload_new_image(
        &self,
        upload_op: ImageUploadOp,
        image_data: GpuImageData,
        resource_type: RafxResourceType,
        generate_mips: bool,
    ) -> RafxResult<()> {
        self.pending_upload_tx
            .send(PendingUpload::Image(PendingImageUpload {
                upload_op,
                image_data,
                resource_type,
                generate_mips,
            }))
            .map_err(|_err| {
                let error = format!("Could not enqueue image upload");
                log::error!("{}", error);
                RafxError::StringError(error)
            })
    }

    pub fn upload_new_buffer(
        &self,
        upload_op: BufferUploadOp,
        resource_type: RafxResourceType,
        data: Vec<u8>,
    ) -> RafxResult<()> {
        self.pending_upload_tx
            .send(PendingUpload::Buffer(PendingBufferUpload {
                upload_op,
                resource_type,
                data,
            }))
            .map_err(|_err| {
                let error = format!("Could not enqueue buffer upload");
                log::error!("{}", error);
                RafxError::StringError(error)
            })
    }

    pub fn upload_to_existing_buffer(
        &self,
        upload_op: ExistingResourceUploadOp,
        resource_type: RafxResourceType,
        data: Vec<u8>,
        dst_buffer: ResourceArc<BufferResource>,
        dst_byte_offset: u64,
    ) -> RafxResult<()> {
        self.pending_upload_tx
            .send(PendingUpload::ExistingBuffer(PendingExistingBufferUpload {
                upload_op,
                resource_type,
                data,
                dst_buffer,
                dst_byte_offset,
            }))
            .map_err(|_err| {
                let error = format!("Could not enqueue buffer upload");
                log::error!("{}", error);
                RafxError::StringError(error)
            })
    }
}
