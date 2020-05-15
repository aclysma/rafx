use renderer_shell_vulkan::{
    VkTransferUploadState, VkDevice, VkDeviceContext, VkTransferUpload, VkImage, VkBuffer,
};
use crossbeam_channel::{Sender, Receiver};
use ash::prelude::VkResult;
use std::time::Duration;
use crate::image_utils::{enqueue_load_images, DecodedTexture};
use std::mem::ManuallyDrop;
use crate::asset_storage::{ResourceHandle, ResourceLoadHandler};
use std::error::Error;
use atelier_assets::core::AssetUuid;
use atelier_assets::loader::{LoadHandle, AssetLoadOp};
use fnv::FnvHashMap;
use std::sync::Arc;
use image::load;

use crate::upload::PendingImageUpload;
use crate::upload::ImageUploadOpResult;
use crate::upload::ImageUploadOpAwaiter;
use crate::resource_managers::sprite_resource_manager::SpriteResourceUpdate;
use crate::pipeline::image::ImageAsset;
use crate::resource_managers::image_resource_manager::ImageResourceUpdate;
use crate::pipeline::sprite::SpriteAsset;

// This is registered with the asset storage which lets us hook when assets are updated
pub struct SpriteLoadHandler {
    sprite_update_tx: Sender<SpriteResourceUpdate>,
}

impl SpriteLoadHandler {
    pub fn new(
        sprite_update_tx: Sender<SpriteResourceUpdate>,
    ) -> Self {
        SpriteLoadHandler {
            sprite_update_tx,
        }
    }
}

// This sends the texture to the upload queue. The upload queue will batch uploads together when update()
// is called on it. When complete, the upload queue will send the sprite handle back via a channel
impl ResourceLoadHandler<SpriteAsset> for SpriteLoadHandler {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<SpriteAsset>,
        version: u32,
        asset: &SpriteAsset,
        load_op: AssetLoadOp,
    ) {
        log::info!(
            "SpriteLoadHandler update_asset {} {:?} {:?}",
            version,
            load_handle,
            resource_handle
        );

        load_op.complete();
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<SpriteAsset>,
        version: u32,
        asset: &SpriteAsset,
    ) {
        log::info!(
            "SpriteLoadHandler commit_asset_version {} {:?} {:?}",
            version,
            load_handle,
            resource_handle
        );

        self.sprite_update_tx.send(SpriteResourceUpdate {
            asset_uuid: *asset_uuid,
            resource_handle: resource_handle,
            images: asset.images.clone(),
        });
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<SpriteAsset>,
        version: u32,
    ) {
        log::info!(
            "SpriteLoadHandler free {:?} {:?}",
            load_handle,
            resource_handle
        );

        //TODO: We are not unloading sprites
    }
}
