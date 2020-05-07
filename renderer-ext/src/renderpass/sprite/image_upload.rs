use renderer_shell_vulkan::{
    VkTransferUploadState, VkDevice, VkDeviceContext, VkTransferUpload, VkImage,
};
use crossbeam_channel::{Sender, Receiver};
use ash::prelude::VkResult;
use std::time::Duration;
use crate::image_utils::{enqueue_load_images, DecodedTexture};
use crate::renderpass::sprite::ImageUpdate;
use std::mem::ManuallyDrop;
use crate::asset_storage::{ResourceHandle, StorageUploader};
use crate::image_importer::ImageAsset;
use std::error::Error;
use atelier_assets::loader::{LoadHandle, AssetLoadOp};
use fnv::FnvHashMap;
use std::sync::Arc;
use image::load;
use std::hint::unreachable_unchecked;


//
// Ghetto futures
//

enum UploadOpResult {
    UploadError,
    UploadComplete(ManuallyDrop<VkImage>),
    UploadDrop,
}

struct UploadOp {
    sender: Option<Sender<UploadOpResult>>,
}

impl UploadOp {
    pub(crate) fn new(sender: Sender<UploadOpResult>) -> Self {
        Self {
            sender: Some(sender),
        }
    }

    pub fn complete(mut self, image: ManuallyDrop<VkImage>) {
        let _ = self
            .sender
            .as_ref()
            .unwrap()
            .send(UploadOpResult::UploadComplete(image));
        self.sender = None;
    }

    pub fn error(mut self) {
        let _ = self
            .sender
            .as_ref()
            .unwrap()
            .send(UploadOpResult::UploadError);
        self.sender = None;
    }
}

impl Drop for UploadOp {
    fn drop(&mut self) {
        if let Some(ref sender) = self.sender {
            let _ = sender.send(UploadOpResult::UploadDrop);
        }
    }
}

struct UploadOpAwaiter {
    receiver: Receiver<UploadOpResult>
}

fn create_upload_op() -> (UploadOp, UploadOpAwaiter) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let op = UploadOp::new(tx);
    let awaiter = UploadOpAwaiter {
        receiver: rx
    };

    (op, awaiter)
}








//
// A storage uploader for ImageAsset
//



// This is registered with the asset storage which lets us hook when assets are updated
pub struct ImageUploader {
    upload_tx: Sender<PendingImageUpload>,
    image_update_tx: Sender<ImageUpdate>,
    pending_updates: FnvHashMap<LoadHandle, FnvHashMap<u32, UploadOpAwaiter>>
}

impl ImageUploader {
    pub fn new(
        upload_tx: Sender<PendingImageUpload>,
        image_update_tx: Sender<ImageUpdate>
    ) -> Self {
        ImageUploader {
            upload_tx,
            image_update_tx,
            pending_updates: Default::default()
        }
    }
}

// This sends the texture to the uploader. The uploader will batch uploads together when update()
// is called on it. When complete, the uploader will send the image handle back via a channel
impl StorageUploader<ImageAsset> for ImageUploader {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        resource_handle: ResourceHandle<ImageAsset>,
        version: u32,
        asset: &ImageAsset,
    ) {
        let texture = DecodedTexture {
            width: asset.width,
            height: asset.height,
            data: asset.data.clone(),
        };

        let (upload_op, awaiter) = create_upload_op();

        self.pending_updates.entry(load_handle).or_default().insert(version,awaiter);

        self.upload_tx
            .send(PendingImageUpload {
                load_op,
                upload_op,
                texture,
                resource_handle,
            })
            .unwrap(); //TODO: Better error handling
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<ImageAsset>,
        version: u32
    ) {
        if let Some(versions) = self.pending_updates.get_mut(&load_handle) {
            if let Some(awaiter) = versions.remove(&version) {

                // We assume that if commit_asset_version is being called the awaiter is signaled
                // and has a valid result
                let value = awaiter.receiver.recv_timeout(Duration::from_secs(0)).unwrap();
                match value {
                    UploadOpResult::UploadComplete(image) => {
                        log::info!("Commit asset {:?} {:?}", load_handle, version);
                        self.image_update_tx.send(ImageUpdate {
                            images: vec![image],
                            resource_handles: vec![resource_handle]
                        });
                    },
                    UploadOpResult::UploadError => unreachable!(),
                    UploadOpResult::UploadDrop => unreachable!(),
                }
            } else {
                log::error!("Could not find awaiter for asset version {:?} {}", load_handle, version);
            }
        } else {
            log::error!("Could not find awaiter for {:?} {}", load_handle, version);
        }
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<ImageAsset>,
    ) {
        //TODO: We are not unloading images
        self.pending_updates.remove(&load_handle);
    }
}









//
// Something to upload resources
//



pub struct PendingImageUpload {
    load_op: AssetLoadOp,
    upload_op: UploadOp,
    texture: DecodedTexture,
    resource_handle: ResourceHandle<ImageAsset>,
}

pub struct PendingBufferUpload {
    load_op: AssetLoadOp,
    upload_op: UploadOp,
    data: Vec<u8>,
    resource_handle: ResourceHandle<ImageAsset>,
}

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
    image_load_ops: Vec<AssetLoadOp>,
    image_upload_ops: Vec<UploadOp>,
    images: Vec<ManuallyDrop<VkImage>>,

    // buffer_load_ops: Vec<AssetLoadOp>,
    // buffer_upload_ops: Vec<UploadOp>,
    // buffers: Vec<ManuallyDrop<VkImage>>,

    upload: VkTransferUpload,
}

// A single upload which may contain multiple images
struct InProgressUpload {
    inner: Option<InProgressUploadInner>,
}

impl InProgressUpload {
    pub fn new(
        image_load_ops: Vec<AssetLoadOp>,
        image_upload_ops: Vec<UploadOp>,
        images: Vec<ManuallyDrop<VkImage>>,
        upload: VkTransferUpload,
    ) -> Self {
        let inner = InProgressUploadInner {
            image_load_ops,
            image_upload_ops,
            images,
            upload,
        };

        InProgressUpload { inner: Some(inner) }
    }

    // The main state machine for an upload:
    // - Submits on the transfer queue and waits
    // - Submits on the graphics queue and waits
    //
    // Calls load_op.complete() or load_op.error() as appropriate
    pub fn poll_load(
        &mut self,
        device: &VkDevice,
    ) -> InProgressUploadPollResult {
        loop {
            if let Some(mut inner) = self.take_inner() {
                match inner.upload.state() {
                    Ok(state) => match state {
                        VkTransferUploadState::Writable => {
                            println!("VkTransferUploadState::Writable");
                            inner.upload.submit_transfer(device.queues.transfer_queue);
                            self.inner = Some(inner);
                        }
                        VkTransferUploadState::SentToTransferQueue => {
                            println!("VkTransferUploadState::SentToTransferQueue");
                            self.inner = Some(inner);
                            break InProgressUploadPollResult::Pending;
                        }
                        VkTransferUploadState::PendingSubmitDstQueue => {
                            println!("VkTransferUploadState::PendingSubmitDstQueue");
                            inner.upload.submit_dst(device.queues.graphics_queue);
                            self.inner = Some(inner);
                        }
                        VkTransferUploadState::SentToDstQueue => {
                            println!("VkTransferUploadState::SentToDstQueue");
                            self.inner = Some(inner);
                            break InProgressUploadPollResult::Pending;
                        }
                        VkTransferUploadState::Complete => {
                            println!("VkTransferUploadState::Complete");
                            let mut image_upload_ops = inner.image_upload_ops;
                            let mut images = inner.images;
                            for i in 0..image_upload_ops.len() {
                                //TODO: This is gross
                                let upload_op = image_upload_ops.pop().unwrap();
                                let image = images.pop().unwrap();
                                //let upload_op = image_upload_ops[i];
                                upload_op.complete(image);
                            }

                            for load_op in inner.image_load_ops {
                                load_op.complete();
                            }
                            break InProgressUploadPollResult::Complete;
                        }
                    },
                    Err(err) => {
                        for load_op in inner.image_load_ops {
                            load_op.error(err);
                        }
                        for upload_op in inner.image_upload_ops {
                            upload_op.error();
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

// Receives sets of images that need to be uploaded and kicks off the upload. Responsible for
// batching image updates together into uploads
pub struct UploadQueue {
    device_context: VkDeviceContext,

    // The ImageUploader associated with the asset storage passes messages via these channels.
    tx: Sender<PendingImageUpload>,
    rx: Receiver<PendingImageUpload>,

    // These are uploads that are currently in progress
    uploads_in_progress: Vec<InProgressUpload>,

    // This channel forwards completed uploads to the sprite resource manager
    //sprite_update_tx: Sender<ImageUpdate>,
}

impl UploadQueue {
    pub fn new(
        device_context: &VkDeviceContext,
    ) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();

        UploadQueue {
            device_context: device_context.clone(),
            tx,
            rx,
            uploads_in_progress: Default::default(),
        }
    }

    pub fn tx(&self) -> &Sender<PendingImageUpload> {
        &self.tx
    }

    fn start_new_uploads(&mut self) -> VkResult<()> {
        let mut image_load_ops = vec![];
        let mut image_upload_ops = vec![];
        let mut decoded_textures = vec![];
        let mut resource_handles = vec![];

        while let Ok(pending_upload) = self.rx.recv_timeout(Duration::from_secs(0)) {
            image_load_ops.push(pending_upload.load_op);
            image_upload_ops.push(pending_upload.upload_op);
            decoded_textures.push(pending_upload.texture);
            resource_handles.push(pending_upload.resource_handle);

            //TODO: Handle budgeting how much we can upload at once
        }

        if decoded_textures.is_empty() {
            return Ok(());
        }

        let mut upload = VkTransferUpload::new(
            &self.device_context,
            self.device_context
                .queue_family_indices()
                .transfer_queue_family_index,
            self.device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            1024 * 1024 * 16,
        )?;

        let images = enqueue_load_images(
            &self.device_context,
            &mut upload,
            self.device_context
                .queue_family_indices()
                .transfer_queue_family_index,
            self.device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            &decoded_textures,
        )?;

        upload.submit_transfer(self.device_context.queues().transfer_queue)?;
        self.uploads_in_progress.push(InProgressUpload::new(
            image_load_ops,
            image_upload_ops,
            images,
            upload,
        ));

        Ok(())
    }

    fn update_existing_uploads(
        &mut self,
        device: &VkDevice,
    ) {
        // iterate backwards so we can use swap_remove
        for i in (0..self.uploads_in_progress.len()).rev() {
            let result = self.uploads_in_progress[i].poll_load(device);
            match result {
                InProgressUploadPollResult::Pending => {
                    // do nothing
                }
                InProgressUploadPollResult::Complete => {
                    //load_op.complete() is called by poll_load
                    self.uploads_in_progress.swap_remove(i);
                }
                InProgressUploadPollResult::Error => {
                    //load_op.error() is called by poll_load
                    self.uploads_in_progress.swap_remove(i);
                }
                InProgressUploadPollResult::Destroyed => {
                    // not expected - this only occurs if polling the upload when it is already in a complete or error state
                    unreachable!();
                }
            }
        }
    }

    pub fn update(
        &mut self,
        device: &VkDevice,
    ) -> VkResult<()> {
        self.start_new_uploads()?;
        self.update_existing_uploads(device);
        Ok(())
    }
}
