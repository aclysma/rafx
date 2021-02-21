use super::load_queue::LoadRequest;
use super::BufferAssetData;
use super::ImageAssetData;
use super::{BufferAsset, ImageAsset};
use crate::assets::image::ImageAssetDataFormat;
use crate::image_upload::{ImageUploadParams, IMAGE_UPLOAD_REQUIRED_SUBRESOURCE_ALIGNMENT};
use crate::{
    buffer_upload, image_upload, GpuImageData, GpuImageDataColorSpace, GpuImageDataLayer,
    GpuImageDataMipLevel,
};
use basis_universal::{TranscodeParameters, TranscoderTextureFormat};
use crossbeam_channel::{Receiver, Sender};
use distill::loader::{storage::AssetLoadOp, LoadHandle};
use rafx_api::{
    extra::upload::*, RafxBuffer, RafxDeviceContext, RafxError, RafxQueue, RafxResourceType,
    RafxResult, RafxTexture,
};

//
// Ghetto futures - UploadOp is used to signal completion and UploadOpAwaiter is used to check the result
//
pub enum UploadOpResult<ResourceT, AssetT> {
    UploadError(LoadHandle),
    UploadComplete(AssetLoadOp, Sender<AssetT>, ResourceT),
    UploadDrop(LoadHandle),
}

pub struct UploadOp<ResourceT, AssetT> {
    load_handle: LoadHandle,
    asset_sender: Option<Sender<AssetT>>, // This sends back to the asset storage, we just pass it along
    sender: Option<Sender<UploadOpResult<ResourceT, AssetT>>>, // This sends back to the resource manager to finalize the load
}

impl<ResourceT, AssetT> UploadOp<ResourceT, AssetT> {
    pub fn new(
        load_handle: LoadHandle,
        asset_sender: Sender<AssetT>,
        sender: Sender<UploadOpResult<ResourceT, AssetT>>,
    ) -> Self {
        Self {
            load_handle,
            asset_sender: Some(asset_sender),
            sender: Some(sender),
        }
    }

    pub fn complete(
        mut self,
        image: ResourceT,
        load_op: AssetLoadOp,
    ) {
        let _ = self
            .sender
            .as_ref()
            .unwrap()
            .send(UploadOpResult::UploadComplete(
                load_op,
                self.asset_sender.take().unwrap(),
                image,
            ));
        self.sender = None;
    }

    pub fn error(mut self) {
        let _ = self
            .sender
            .as_ref()
            .unwrap()
            .send(UploadOpResult::UploadError(self.load_handle));
        self.sender = None;
    }
}

impl<ResourceT, AssetT> Drop for UploadOp<ResourceT, AssetT> {
    fn drop(&mut self) {
        if let Some(ref sender) = self.sender {
            let _ = sender.send(UploadOpResult::UploadDrop(self.load_handle));
        }
    }
}

pub type ImageUploadOpResult = UploadOpResult<RafxTexture, ImageAsset>;
pub type ImageUploadOp = UploadOp<RafxTexture, ImageAsset>;

pub type BufferUploadOpResult = UploadOpResult<RafxBuffer, BufferAsset>;
pub type BufferUploadOp = UploadOp<RafxBuffer, BufferAsset>;

//
// Represents a single request inserted into the upload queue that hasn't started yet
//
//TODO: Make a helper object that carries an Arc<Receiver> that can be called
pub struct PendingImageUpload {
    pub load_op: AssetLoadOp,
    pub upload_op: ImageUploadOp,
    pub image_data: GpuImageData,
    pub resource_type: RafxResourceType,
    pub generate_mips: bool,
}

pub struct PendingBufferUpload {
    pub load_op: AssetLoadOp,
    pub upload_op: BufferUploadOp,
    pub data: Vec<u8>,
}

//
// Represents a single request that the upload queue has started
//
struct InFlightImageUpload {
    load_op: AssetLoadOp,
    upload_op: ImageUploadOp,
    texture: RafxTexture,
}

pub struct InFlightBufferUpload {
    load_op: AssetLoadOp,
    upload_op: BufferUploadOp,
    buffer: RafxBuffer,
}

//
// Represents a batch of requests that has been started, contains multiple InFlightImageUpload and
// InFlightBufferUploads
//

// The result from polling a single upload (which may contain multiple images in it)
pub enum InProgressUploadPollResult {
    Pending,
    Complete,
    Error,
    Destroyed,
}

// This is an inner of InProgressImageUpload - it is wrapped in a Option to avoid borrowing issues
// when polling by allowing us to temporarily take ownership of it and then put it back
struct InProgressUploadInner {
    image_uploads: Vec<InFlightImageUpload>,
    buffer_uploads: Vec<InFlightBufferUpload>,
    upload: RafxTransferUpload,
}

struct InProgressUploadDebugInfo {
    upload_id: usize,
    start_time: std::time::Instant,
    size: u64,
    image_count: usize,
    buffer_count: usize,
}

// A single upload which may contain multiple images
struct InProgressUpload {
    // Only valid if the upload is actually in progress
    inner: Option<InProgressUploadInner>,
    debug_info: InProgressUploadDebugInfo,
}

impl InProgressUpload {
    pub fn new(
        image_uploads: Vec<InFlightImageUpload>,
        buffer_uploads: Vec<InFlightBufferUpload>,
        upload: RafxTransferUpload,
        debug_info: InProgressUploadDebugInfo,
    ) -> Self {
        let inner = InProgressUploadInner {
            image_uploads,
            buffer_uploads,
            upload,
        };

        InProgressUpload {
            inner: Some(inner),
            debug_info,
        }
    }

    // The main state machine for an upload:
    // - Submits on the transfer queue and waits
    // - Submits on the graphics queue and waits
    //
    // Calls load_op.complete() or load_op.error() as appropriate
    pub fn poll_load(&mut self) -> InProgressUploadPollResult {
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
                            break InProgressUploadPollResult::Pending;
                        }
                        RafxTransferUploadState::PendingSubmitDstQueue => {
                            //log::trace!("RafxTransferUploadState::PendingSubmitDstQueue");
                            inner.upload.submit_dst().unwrap();
                            self.inner = Some(inner);
                        }
                        RafxTransferUploadState::SentToDstQueue => {
                            //log::trace!("RafxTransferUploadState::SentToDstQueue");
                            self.inner = Some(inner);
                            break InProgressUploadPollResult::Pending;
                        }
                        RafxTransferUploadState::Complete => {
                            //log::trace!("RafxTransferUploadState::Complete");
                            for upload in inner.image_uploads {
                                let texture = upload.texture;
                                upload.upload_op.complete(texture, upload.load_op);
                            }

                            for upload in inner.buffer_uploads {
                                let buffer = upload.buffer;
                                upload.upload_op.complete(buffer, upload.load_op);
                            }

                            break InProgressUploadPollResult::Complete;
                        }
                    },
                    Err(err) => {
                        for upload in inner.image_uploads {
                            upload.load_op.error(err.clone());
                            upload.upload_op.error();
                            // Image is dropped here
                        }

                        for upload in inner.buffer_uploads {
                            upload.load_op.error(err.clone());
                            upload.upload_op.error();
                            // Buffer is dropped here
                        }

                        break InProgressUploadPollResult::Error;
                    }
                }
            } else {
                break InProgressUploadPollResult::Destroyed;
            }
        }
    }

    // Allows taking ownership of the inner object
    fn take_inner(&mut self) -> Option<InProgressUploadInner> {
        let mut inner = None;
        std::mem::swap(&mut self.inner, &mut inner);
        inner
    }
}

impl Drop for InProgressUpload {
    fn drop(&mut self) {
        if let Some(mut inner) = self.take_inner() {
            // I don't think order of destruction matters but just in case
            inner.image_uploads.clear();
            inner.buffer_uploads.clear();
        }
    }
}

pub struct UploadQueueConfig {
    pub max_bytes_per_upload: usize,
    pub max_concurrent_uploads: usize,
    pub max_new_uploads_in_single_frame: usize,
}

//
// Receives sets of images/buffers that need to be uploaded and kicks off the upload. Responsible
// for batching image updates together into uploads
//
pub struct UploadQueue {
    device_context: RafxDeviceContext,
    config: UploadQueueConfig,

    // For enqueueing images to upload
    pending_image_tx: Sender<PendingImageUpload>,
    pending_image_rx: Receiver<PendingImageUpload>,

    // If we fail to upload due to size limitation, keep the failed upload here to retry later
    next_image_upload: Option<PendingImageUpload>,

    // For enqueueing buffers to upload
    pending_buffer_tx: Sender<PendingBufferUpload>,
    pending_buffer_rx: Receiver<PendingBufferUpload>,

    // If we fail to upload due to size limitation, keep the failed upload here to retry later
    next_buffer_upload: Option<PendingBufferUpload>,

    // These are uploads that are currently in progress
    uploads_in_progress: Vec<InProgressUpload>,

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
    ) -> Self {
        let (pending_image_tx, pending_image_rx) = crossbeam_channel::unbounded();
        let (pending_buffer_tx, pending_buffer_rx) = crossbeam_channel::unbounded();

        UploadQueue {
            device_context: device_context.clone(),
            config,
            pending_image_tx,
            pending_image_rx,
            next_image_upload: None,
            pending_buffer_tx,
            pending_buffer_rx,
            next_buffer_upload: None,
            uploads_in_progress: Default::default(),
            next_upload_id: 1,
            graphics_queue,
            transfer_queue,
        }
    }

    pub fn pending_image_tx(&self) -> &Sender<PendingImageUpload> {
        &self.pending_image_tx
    }

    pub fn pending_buffer_tx(&self) -> &Sender<PendingBufferUpload> {
        &self.pending_buffer_tx
    }

    // Ok(None) = upload enqueue
    // Ok(Some) = upload not enqueued because there was not enough room
    // Err = Vulkan error
    fn try_enqueue_image_upload(
        &mut self,
        upload: &mut RafxTransferUpload,
        pending_image: PendingImageUpload,
        in_flight_uploads: &mut Vec<InFlightImageUpload>,
    ) -> RafxResult<Option<PendingImageUpload>> {
        let result = image_upload::enqueue_load_image(
            &self.device_context,
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
                in_flight_uploads.push(InFlightImageUpload {
                    texture,
                    load_op: pending_image.load_op,
                    upload_op: pending_image.upload_op,
                });
                Ok(None)
            }
            Err(RafxUploadError::Other(e)) => Err(e),
            Err(RafxUploadError::BufferFull) => Ok(Some(pending_image)),
        }
    }

    fn start_new_image_uploads(
        &mut self,
        upload: &mut RafxTransferUpload,
    ) -> RafxResult<Vec<InFlightImageUpload>> {
        let mut in_flight_uploads = vec![];

        // If we had a pending image upload from before, try to upload it now
        self.next_image_upload = if let Some(next_image_upload) = self.next_image_upload.take() {
            self.try_enqueue_image_upload(upload, next_image_upload, &mut in_flight_uploads)?
        } else {
            None
        };

        // The first image we tried to upload failed. Log an error since we aren't making forward progress
        if let Some(next_image_upload) = &self.next_image_upload {
            log::error!(
                "Image of {} bytes has repeatedly exceeded the available room in the upload buffer. ({} of {} bytes free)",
                next_image_upload.image_data.total_size(IMAGE_UPLOAD_REQUIRED_SUBRESOURCE_ALIGNMENT as u64),
                upload.bytes_free(),
                upload.buffer_size()
            );
            return Ok(vec![]);
        }

        let rx = self.pending_image_rx.clone();
        for pending_upload in rx.try_iter() {
            self.next_image_upload =
                self.try_enqueue_image_upload(upload, pending_upload, &mut in_flight_uploads)?;

            if let Some(next_image_upload) = &self.next_image_upload {
                log::debug!(
                    "Image of {} bytes exceeds the available room in the upload buffer. ({} of {} bytes free)",
                    next_image_upload.image_data.total_size(IMAGE_UPLOAD_REQUIRED_SUBRESOURCE_ALIGNMENT as u64),
                    upload.bytes_free(),
                    upload.buffer_size(),
                );
                break;
            }
        }

        Ok(in_flight_uploads)
    }

    // Ok(None) = upload enqueue
    // Ok(Some) = upload not enqueued because there was not enough room
    // Err = Vulkan error
    fn try_enqueue_buffer_upload(
        &mut self,
        upload: &mut RafxTransferUpload,
        pending_buffer: PendingBufferUpload,
        in_flight_uploads: &mut Vec<InFlightBufferUpload>,
    ) -> RafxResult<Option<PendingBufferUpload>> {
        let result = buffer_upload::enqueue_load_buffer(
            &self.device_context,
            upload,
            // self.transfer_queue.queue_family_index(),
            // self.graphics_queue.queue_family_index(),
            &pending_buffer.data,
        );

        match result {
            Ok(buffer) => {
                in_flight_uploads.push(InFlightBufferUpload {
                    buffer,
                    load_op: pending_buffer.load_op,
                    upload_op: pending_buffer.upload_op,
                });
                Ok(None)
            }
            Err(RafxUploadError::Other(e)) => Err(e),
            Err(RafxUploadError::BufferFull) => Ok(Some(pending_buffer)),
        }
    }

    fn start_new_buffer_uploads(
        &mut self,
        upload: &mut RafxTransferUpload,
    ) -> RafxResult<Vec<InFlightBufferUpload>> {
        let mut in_flight_uploads = vec![];

        // If we had a pending image upload from before, try to upload it now
        self.next_buffer_upload = if let Some(next_buffer_upload) = self.next_buffer_upload.take() {
            self.try_enqueue_buffer_upload(upload, next_buffer_upload, &mut in_flight_uploads)?
        } else {
            None
        };

        // The first buffer we tried to upload failed. Log an error since we aren't making forward progress
        if let Some(next_buffer_upload) = &self.next_buffer_upload {
            log::error!(
                "Buffer of {} bytes has repeatedly exceeded the available room in the upload buffer. ({} of {} bytes free)",
                next_buffer_upload.data.len(),
                upload.bytes_free(),
                upload.buffer_size()
            );
            return Ok(vec![]);
        }

        let rx = self.pending_buffer_rx.clone();
        for pending_upload in rx.try_iter() {
            self.next_buffer_upload =
                self.try_enqueue_buffer_upload(upload, pending_upload, &mut in_flight_uploads)?;

            if let Some(next_buffer_upload) = &self.next_buffer_upload {
                log::debug!(
                    "Buffer of {} bytes exceeds the available room in the upload buffer. ({} of {} bytes free)",
                    next_buffer_upload.data.len(),
                    upload.bytes_free(),
                    upload.buffer_size(),
                );
                break;
            }
        }

        Ok(in_flight_uploads)
    }

    fn start_new_uploads(&mut self) -> RafxResult<()> {
        for _ in 0..self.config.max_new_uploads_in_single_frame {
            if self.pending_image_rx.is_empty()
                && self.next_image_upload.is_none()
                && self.pending_buffer_rx.is_empty()
                && self.next_buffer_upload.is_none()
            {
                return Ok(());
            }

            if self.uploads_in_progress.len() >= self.config.max_concurrent_uploads {
                log::trace!(
                    "Max number of uploads already in progress. Waiting to start a new one"
                );
                return Ok(());
            }

            if !self.start_new_upload()? {
                return Ok(());
            }
        }

        Ok(())
    }

    fn start_new_upload(&mut self) -> RafxResult<bool> {
        let mut upload = RafxTransferUpload::new(
            &self.device_context,
            &self.transfer_queue,
            &self.graphics_queue,
            self.config.max_bytes_per_upload as u64,
        )?;

        let in_flight_image_uploads = self.start_new_image_uploads(&mut upload)?;
        let in_flight_buffer_uploads = self.start_new_buffer_uploads(&mut upload)?;

        if !in_flight_image_uploads.is_empty() || !in_flight_buffer_uploads.is_empty() {
            let upload_id = self.next_upload_id;
            self.next_upload_id += 1;

            log::debug!(
                "Submitting {} byte upload with {} images and {} buffers, UploadId = {}",
                upload.bytes_written(),
                in_flight_image_uploads.len(),
                in_flight_buffer_uploads.len(),
                upload_id
            );

            upload.submit_transfer()?;

            let debug_info = InProgressUploadDebugInfo {
                upload_id,
                buffer_count: in_flight_buffer_uploads.len(),
                image_count: in_flight_image_uploads.len(),
                size: upload.bytes_written(),
                start_time: std::time::Instant::now(),
            };

            self.uploads_in_progress.push(InProgressUpload::new(
                in_flight_image_uploads,
                in_flight_buffer_uploads,
                upload,
                debug_info,
            ));

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn update_existing_uploads(&mut self) {
        // iterate backwards so we can use swap_remove
        for i in (0..self.uploads_in_progress.len()).rev() {
            let result = self.uploads_in_progress[i].poll_load();
            match result {
                InProgressUploadPollResult::Pending => {
                    // do nothing
                }
                InProgressUploadPollResult::Complete => {
                    //load_op.complete() is called by poll_load

                    let debug_info = &self.uploads_in_progress[i].debug_info;
                    log::debug!(
                        "Completed {} byte upload with {} images and {} buffers in {} ms, UploadId = {}",
                        debug_info.size,
                        debug_info.image_count,
                        debug_info.buffer_count,
                        (std::time::Instant::now() - debug_info.start_time).as_secs_f32(),
                        debug_info.upload_id
                    );

                    self.uploads_in_progress.swap_remove(i);
                }
                InProgressUploadPollResult::Error => {
                    //load_op.error() is called by poll_load

                    let debug_info = &self.uploads_in_progress[i].debug_info;
                    log::error!(
                        "Failed {} byte upload with {} images and {} buffers in {} ms, UploadId = {}",
                        debug_info.size,
                        debug_info.image_count,
                        debug_info.buffer_count,
                        (std::time::Instant::now() - debug_info.start_time).as_secs_f32(),
                        debug_info.upload_id
                    );

                    self.uploads_in_progress.swap_remove(i);
                }
                InProgressUploadPollResult::Destroyed => {
                    // not expected - this only occurs if polling the upload when it is already in a complete or error state
                    unreachable!();
                }
            }
        }
    }

    pub fn update(&mut self) -> RafxResult<()> {
        self.start_new_uploads()?;
        self.update_existing_uploads();
        Ok(())
    }
}

pub struct UploadManager {
    upload_queue: UploadQueue,

    pub image_upload_result_tx: Sender<ImageUploadOpResult>,
    pub image_upload_result_rx: Receiver<ImageUploadOpResult>,

    pub buffer_upload_result_tx: Sender<BufferUploadOpResult>,
    pub buffer_upload_result_rx: Receiver<BufferUploadOpResult>,

    pub astc4x4_supported: bool,
    pub bc7_supported: bool,
}

impl UploadManager {
    pub fn new(
        device_context: &RafxDeviceContext,
        upload_queue_config: UploadQueueConfig,
        graphics_queue: RafxQueue,
        transfer_queue: RafxQueue,
    ) -> Self {
        let (image_upload_result_tx, image_upload_result_rx) = crossbeam_channel::unbounded();
        let (buffer_upload_result_tx, buffer_upload_result_rx) = crossbeam_channel::unbounded();

        UploadManager {
            upload_queue: UploadQueue::new(
                device_context,
                upload_queue_config,
                graphics_queue,
                transfer_queue,
            ),
            image_upload_result_rx,
            image_upload_result_tx,
            buffer_upload_result_rx,
            buffer_upload_result_tx,
            astc4x4_supported: false,
            bc7_supported: true,
        }
    }

    pub fn update(&mut self) -> RafxResult<()> {
        self.upload_queue.update()
    }

    pub fn upload_image(
        &self,
        request: LoadRequest<ImageAssetData, ImageAsset>,
    ) -> RafxResult<()> {
        let color_space: GpuImageDataColorSpace = request.asset.color_space.into();

        let generate_mips = request.asset.generate_mips_at_runtime;

        let t0 = std::time::Instant::now();
        let image_data = match request.asset.format {
            ImageAssetDataFormat::RawRGBA32 => GpuImageData::new_simple(
                request.asset.width,
                request.asset.height,
                color_space.rgba8(),
                request.asset.data,
            ),
            ImageAssetDataFormat::BasisCompressed => {
                let data = request.asset.data;
                let mut transcoder = basis_universal::Transcoder::new();
                transcoder.prepare_transcoding(&data).unwrap();

                let (rafx_format, transcode_format) = if generate_mips {
                    // We can't do runtime mip generation with compresed formats, fall back to uncompressed data
                    (color_space.rgba8(), TranscoderTextureFormat::RGBA32)
                } else if self.astc4x4_supported {
                    (
                        color_space.astc4x4(),
                        TranscoderTextureFormat::ASTC_4x4_RGBA,
                    )
                } else if self.bc7_supported {
                    (color_space.bc7(), TranscoderTextureFormat::BC7_RGBA)
                } else {
                    (color_space.rgba8(), TranscoderTextureFormat::RGBA32)
                };

                let layer_count = transcoder.image_count(&data);
                if layer_count == 0 {
                    Err("BasisCompressed image asset has no images")?;
                }

                let level_count = transcoder.image_level_count(&data, 0);
                if level_count == 0 {
                    Err("BasisCompressed image asset has image with no mip levels")?;
                }

                if level_count > 1 && generate_mips {
                    Err("BasisCompressed image asset configured to generate mips at runtime but has more than one mip layer stored")?;
                }

                log::trace!(
                    "Decompressing basis format: {:?} transcode format: {:?} layers: {} levels {}",
                    rafx_format,
                    transcode_format,
                    layer_count,
                    level_count
                );

                let mut layers = Vec::with_capacity(layer_count as usize);
                for layer_index in 0..layer_count {
                    let image_level_count = transcoder.image_level_count(&data, layer_index);
                    if image_level_count != level_count {
                        Err(format!("Two images in a BasisCompressed image asset has different mip level counts ({} and {})", level_count, image_level_count))?;
                    }

                    let mut levels = Vec::with_capacity(level_count as usize);
                    for level_index in 0..level_count {
                        let level_description = transcoder
                            .image_level_description(&data, layer_index, level_index)
                            .unwrap();

                        log::trace!(
                            "transcoding layer {} level {} size: {}x{}",
                            layer_index,
                            level_index,
                            level_description.original_width,
                            level_description.original_height
                        );

                        let level_data = transcoder
                            .transcode_image_level(
                                &data,
                                transcode_format,
                                TranscodeParameters {
                                    image_index: layer_index,
                                    level_index,
                                    ..Default::default()
                                },
                            )
                            .unwrap();

                        levels.push(GpuImageDataMipLevel {
                            width: level_description.original_width,
                            height: level_description.original_height,
                            data: level_data,
                        });
                    }

                    layers.push(GpuImageDataLayer::new(levels));
                }

                GpuImageData::new(layers, rafx_format)
            }
        };
        let t1 = std::time::Instant::now();

        #[cfg(debug_assertions)]
        image_data.verify_state();

        log::info!(
            "GpuImageData {}x{} format {:?} total bytes {} prepared in {}ms",
            image_data.width,
            image_data.height,
            image_data.format,
            image_data.total_size(IMAGE_UPLOAD_REQUIRED_SUBRESOURCE_ALIGNMENT),
            (t1 - t0).as_secs_f64() * 1000.0
        );

        self.upload_queue
            .pending_image_tx()
            .send(PendingImageUpload {
                load_op: request.load_op,
                upload_op: UploadOp::new(
                    request.load_handle,
                    request.result_tx,
                    self.image_upload_result_tx.clone(),
                ),
                image_data,
                resource_type: request.asset.resource_type,
                generate_mips,
            })
            .map_err(|_err| {
                let error = format!("Could not enqueue image upload");
                log::error!("{}", error);
                RafxError::StringError(error)
            })
    }

    pub fn upload_buffer(
        &self,
        request: LoadRequest<BufferAssetData, BufferAsset>,
    ) -> RafxResult<()> {
        self.upload_queue
            .pending_buffer_tx()
            .send(PendingBufferUpload {
                load_op: request.load_op,
                upload_op: UploadOp::new(
                    request.load_handle,
                    request.result_tx,
                    self.buffer_upload_result_tx.clone(),
                ),
                data: request.asset.data,
            })
            .map_err(|_err| {
                let error = format!("Could not enqueue buffer upload");
                log::error!("{}", error);
                RafxError::StringError(error)
            })
    }
}
