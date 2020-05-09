use renderer_shell_vulkan::{VkTransferUploadState, VkDevice, VkDeviceContext, VkTransferUpload, VkImage, VkBuffer};
use crossbeam_channel::{Sender, Receiver};
use ash::prelude::VkResult;
use std::time::Duration;
use crate::image_utils::{enqueue_load_images, DecodedTexture};
use crate::renderpass::sprite::ImageUpdate;
use std::mem::ManuallyDrop;
use crate::asset_storage::{ResourceHandle, StorageUploader};
use crate::image_importer::ImageAsset;
use std::error::Error;
use atelier_assets::core::AssetUuid;
use atelier_assets::loader::{LoadHandle, AssetLoadOp};
use fnv::FnvHashMap;
use std::sync::Arc;
use image::load;

use crate::upload::PendingImageUpload;
use crate::upload::ImageUploadOpResult;
use crate::upload::ImageUploadOpAwaiter;

struct PendingImageUpdate {
    awaiter: ImageUploadOpAwaiter
}

// This is registered with the asset storage which lets us hook when assets are updated
pub struct ImageUploader {
    upload_tx: Sender<PendingImageUpload>,
    image_update_tx: Sender<ImageUpdate>,
    pending_updates: FnvHashMap<LoadHandle, FnvHashMap<u32, PendingImageUpdate>>
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
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<ImageAsset>,
        version: u32,
        asset: &ImageAsset,
    ) {
        log::info!("ImageUploader update_asset {} {:?} {:?}", version, load_handle, resource_handle);
        let texture = DecodedTexture {
            width: asset.width,
            height: asset.height,
            data: asset.data.clone(),
        };

        let (upload_op, awaiter) = crate::upload::create_upload_op();

        let pending_update = PendingImageUpdate {
            awaiter
        };

        self.pending_updates.entry(load_handle).or_default().insert(version,pending_update);

        self.upload_tx
            .send(PendingImageUpload {
                load_op,
                upload_op,
                texture,
            })
            .unwrap(); //TODO: Better error handling
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<ImageAsset>,
        version: u32
    ) {
        log::info!("ImageUploader commit_asset_version {} {:?} {:?}", version, load_handle, resource_handle);
        if let Some(versions) = self.pending_updates.get_mut(&load_handle) {
            if let Some(pending_update) = versions.remove(&version) {
                let awaiter = pending_update.awaiter;

                // We assume that if commit_asset_version is being called the awaiter is signaled
                // and has a valid result
                let value = awaiter.receiver().recv_timeout(Duration::from_secs(0)).unwrap();
                match value {
                    ImageUploadOpResult::UploadComplete(image) => {
                        log::info!("Commit asset {:?} {:?}", load_handle, version);
                        self.image_update_tx.send(ImageUpdate {
                            images: vec![image],
                            resource_handles: vec![resource_handle]
                        });
                    },
                    ImageUploadOpResult::UploadError => unreachable!(),
                    ImageUploadOpResult::UploadDrop => unreachable!(),
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
        log::info!("ImageUploader free {:?} {:?}", load_handle, resource_handle);
        //TODO: We are not unloading images
        self.pending_updates.remove(&load_handle);
    }
}

