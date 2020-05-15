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
use crate::pipeline::shader::ShaderAsset;
use ash::vk;
use crate::resource_managers::shader_resource_manager::ShaderResourceUpdate;
use ash::version::DeviceV1_0;

// This is registered with the asset storage, letting us hook into the asset
// update/commit/free lifecycle
pub struct ShaderLoadHandler {
    // We fire update messages into the shader resource manager here
    device_context: VkDeviceContext,
    shader_update_tx: Sender<ShaderResourceUpdate>,
}

impl ShaderLoadHandler {
    pub fn new(
        device_context: &VkDeviceContext,
        shader_update_tx: Sender<ShaderResourceUpdate>,
    ) -> Self {
        ShaderLoadHandler {
            device_context: device_context.clone(),
            shader_update_tx,
        }
    }
}

// This sends the texture to the upload queue. The upload queue will batch uploads together when update()
// is called on it. When complete, the upload queue will send the material handle back via a channel
impl ResourceLoadHandler<ShaderAsset> for ShaderLoadHandler {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<ShaderAsset>,
        version: u32,
        asset: &ShaderAsset,
        load_op: AssetLoadOp,
    ) {
        log::info!(
            "ShaderLoadHandler update_asset {} {:?} {:?}",
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
        resource_handle: ResourceHandle<ShaderAsset>,
        version: u32,
        asset: &ShaderAsset,
    ) {
        log::info!(
            "MaterialLoadHandler commit_asset_version {} {:?} {:?}",
            version,
            load_handle,
            resource_handle
        );

        let shader = vk::ShaderModuleCreateInfo::builder()
            .code(&asset.data);
        let shader_module = unsafe {
            self.device_context.device().create_shader_module(&shader, None)
        };

        match shader_module {
            Ok(shader_module) => {
                self.shader_update_tx.send(ShaderResourceUpdate {
                    asset_uuid: *asset_uuid,
                    resource_handle,
                    shader_module
                });
            },
            Err(err) => log::error!("Error loading shader module: {:?}", err)
        }
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<ShaderAsset>,
        version: u32,
    ) {
        log::info!(
            "MaterialLoadHandler free {:?} {:?}",
            load_handle,
            resource_handle
        );

        //TODO: We are not unloading shaders
    }
}
