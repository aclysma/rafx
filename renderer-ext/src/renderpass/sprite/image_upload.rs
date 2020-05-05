use renderer_shell_vulkan::{
    VkTransferUploadState, VkDevice, VkDeviceContext, VkTransferUpload, VkImage,
};
use std::sync::mpsc::{Sender, Receiver};
use ash::prelude::VkResult;
use std::time::Duration;
use crate::image_utils::{enqueue_load_images, DecodedTexture};
use crate::renderpass::sprite::ImageUpdate;
use std::mem::ManuallyDrop;
use crate::asset_storage::{ResourceHandle, StorageUploader};
use crate::image_importer::ImageAsset;
use std::error::Error;
use atelier_assets::loader::AssetLoadOp;

// This is registered with the asset storage which lets us hook when assets are updated
pub struct ImageUploader {
    device_context: VkDeviceContext,
    tx: Sender<PendingImageUpload>,
}

impl ImageUploader {
    pub fn new(
        device_context: VkDeviceContext,
        tx: Sender<PendingImageUpload>,
    ) -> Self {
        ImageUploader { device_context, tx }
    }
}

impl StorageUploader<ImageAsset> for ImageUploader {
    fn upload(
        &self,
        asset: &ImageAsset,
        load_op: AssetLoadOp,
        resource_handle: ResourceHandle<ImageAsset>,
    ) {
        let texture = DecodedTexture {
            width: asset.width,
            height: asset.height,
            data: asset.data.clone(),
        };

        //TODO: This is not respecting commit() - it just updates sprites as soon as it can
        self.tx
            .send(PendingImageUpload {
                load_op,
                texture,
                resource_handle,
            })
            .unwrap(); //TODO: Better error handling
    }

    fn free(
        &self,
        resource_handle: ResourceHandle<ImageAsset>,
    ) {
        //TODO: We are not unloading images
    }
}

// A message sent to ImageUploadQueue
pub struct PendingImageUpload {
    load_op: AssetLoadOp,
    texture: DecodedTexture,
    resource_handle: ResourceHandle<ImageAsset>,
}

// The result from polling a single upload (which may contain multiple images in it)
pub enum InProgressImageUploadPollResult {
    Pending,
    Complete(Vec<ManuallyDrop<VkImage>>, Vec<ResourceHandle<ImageAsset>>),
    Error(Box<Error + 'static + Send>),
    Destroyed,
}

// This is an inner of InProgressImageUpload - it is wrapped in a Option to avoid borrowing issues
// when polling by allowing us to temporarily take ownership of it and then put it back
struct InProgressImageUploadInner {
    load_ops: Vec<AssetLoadOp>,
    images: Vec<ManuallyDrop<VkImage>>,
    resource_handles: Vec<ResourceHandle<ImageAsset>>,
    upload: VkTransferUpload,
}

// A single upload which may contain multiple images
struct InProgressImageUpload {
    inner: Option<InProgressImageUploadInner>,
}

impl InProgressImageUpload {
    pub fn new(
        load_ops: Vec<AssetLoadOp>,
        images: Vec<ManuallyDrop<VkImage>>,
        resource_handles: Vec<ResourceHandle<ImageAsset>>,
        upload: VkTransferUpload,
    ) -> Self {
        let inner = InProgressImageUploadInner {
            load_ops,
            images,
            resource_handles,
            upload,
        };

        InProgressImageUpload { inner: Some(inner) }
    }

    // The main state machine for an upload:
    // - Submits on the transfer queue and waits
    // - Submits on the graphics queue and waits
    //
    // Calls load_op.complete() or load_op.error() as appropriate
    pub fn poll_load(
        &mut self,
        device: &VkDevice,
    ) -> InProgressImageUploadPollResult {
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
                            break InProgressImageUploadPollResult::Pending;
                        }
                        VkTransferUploadState::PendingSubmitDstQueue => {
                            println!("VkTransferUploadState::PendingSubmitDstQueue");
                            inner.upload.submit_dst(device.queues.graphics_queue);
                            self.inner = Some(inner);
                        }
                        VkTransferUploadState::SentToDstQueue => {
                            println!("VkTransferUploadState::SentToDstQueue");
                            self.inner = Some(inner);
                            break InProgressImageUploadPollResult::Pending;
                        }
                        VkTransferUploadState::Complete => {
                            println!("VkTransferUploadState::Complete");
                            for load_op in inner.load_ops {
                                load_op.complete();
                            }
                            break InProgressImageUploadPollResult::Complete(
                                inner.images,
                                inner.resource_handles,
                            );
                        }
                    },
                    Err(err) => {
                        for load_op in inner.load_ops {
                            load_op.error(err);
                        }
                        break InProgressImageUploadPollResult::Error(Box::new(err));
                    }
                }
            } else {
                break InProgressImageUploadPollResult::Destroyed;
            }
        }
    }

    // Allows taking ownership of the inner object
    fn take_inner(&mut self) -> Option<InProgressImageUploadInner> {
        let mut inner = None;
        std::mem::swap(&mut self.inner, &mut inner);
        inner
    }
}

// Receives sets of images that need to be uploaded and kicks off the upload. Responsible for
// batching image updates together into uploads
pub struct ImageUploadQueue {
    device_context: VkDeviceContext,

    // The ImageUploader associated with the asset storage passes messages via these channels.
    tx: Sender<PendingImageUpload>,
    rx: Receiver<PendingImageUpload>,

    // These are uploads that are currently in progress
    uploads_in_progress: Vec<InProgressImageUpload>,

    // This channel forwards completed uploads to the sprite resource manager
    sprite_update_tx: Sender<ImageUpdate>,
}

impl ImageUploadQueue {
    pub fn new(
        device_context: &VkDeviceContext,
        sprite_update_tx: Sender<ImageUpdate>,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        ImageUploadQueue {
            device_context: device_context.clone(),
            tx,
            rx,
            uploads_in_progress: Default::default(),
            sprite_update_tx,
        }
    }

    pub fn tx(&self) -> &Sender<PendingImageUpload> {
        &self.tx
    }

    fn start_new_uploads(&mut self) -> VkResult<()> {
        let mut load_ops = vec![];
        let mut decoded_textures = vec![];
        let mut resource_handles = vec![];

        while let Ok(pending_upload) = self.rx.recv_timeout(Duration::from_secs(0)) {
            load_ops.push(pending_upload.load_op);
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
        self.uploads_in_progress.push(InProgressImageUpload::new(
            load_ops,
            images,
            resource_handles,
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
                InProgressImageUploadPollResult::Pending => {
                    // do nothing
                }
                InProgressImageUploadPollResult::Complete(images, resource_handles) => {
                    //load_op.complete() is called by poll_load
                    let upload = self.uploads_in_progress.swap_remove(i);
                    self.sprite_update_tx.send(ImageUpdate {
                        images,
                        resource_handles,
                    });
                }
                InProgressImageUploadPollResult::Error(e) => {
                    //load_op.error() is called by poll_load
                    self.uploads_in_progress.swap_remove(i);
                }
                InProgressImageUploadPollResult::Destroyed => {
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
