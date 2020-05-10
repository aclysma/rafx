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
use crate::pipeline::gltf::MaterialAsset;
use crate::resource_managers::material_resource_manager::MaterialResourceUpdate;

struct PendingMaterialUpdate {
    asset_uuid: AssetUuid,
    image_uuid: Option<AssetUuid>
}

// This is registered with the asset storage which lets us hook when assets are updated
pub struct MaterialLoadHandler {
    material_update_tx: Sender<MaterialResourceUpdate>,
    pending_updates: FnvHashMap<LoadHandle, FnvHashMap<u32, PendingMaterialUpdate>>,
}

impl MaterialLoadHandler {
    pub fn new(
        material_update_tx: Sender<MaterialResourceUpdate>,
    ) -> Self {
        MaterialLoadHandler {
            material_update_tx,
            pending_updates: Default::default(),
        }
    }
}

// This sends the texture to the upload queue. The upload queue will batch uploads together when update()
// is called on it. When complete, the upload queue will send the material handle back via a channel
impl ResourceLoadHandler<MaterialAsset> for MaterialLoadHandler {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<MaterialAsset>,
        version: u32,
        asset: &MaterialAsset,
    ) {
        log::info!(
            "MaterialLoadHandler update_asset {} {:?} {:?}",
            version,
            load_handle,
            resource_handle
        );

        let pending_update = PendingMaterialUpdate {
            asset_uuid: *asset_uuid,
            image_uuid: asset.base_color_texture
        };

        self.pending_updates
            .entry(load_handle)
            .or_default()
            .insert(version, pending_update);

        load_op.complete();
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<MaterialAsset>,
        version: u32,
    ) {
        log::info!(
            "MaterialLoadHandler commit_asset_version {} {:?} {:?}",
            version,
            load_handle,
            resource_handle
        );
        if let Some(versions) = self.pending_updates.get_mut(&load_handle) {
            if let Some(pending_update) = versions.remove(&version) {
                self.material_update_tx.send(MaterialResourceUpdate {
                    asset_uuid: pending_update.asset_uuid,
                    resource_handle: resource_handle,

                    image_uuid: pending_update.image_uuid,
                });
            } else {
                log::error!(
                    "Could not find awaiter for asset version {:?} {}",
                    load_handle,
                    version
                );
            }
        } else {
            log::error!("Could not find awaiter for {:?} {}", load_handle, version);
        }
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<MaterialAsset>,
    ) {
        log::info!(
            "MaterialLoadHandler free {:?} {:?}",
            load_handle,
            resource_handle
        );
        //TODO: We are not unloading images
        self.pending_updates.remove(&load_handle);
    }
}
