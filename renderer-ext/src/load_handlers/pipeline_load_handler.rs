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
use crate::pipeline::pipeline::PipelineAsset;

// This is registered with the asset storage which lets us hook when assets are updated
pub struct PipelineLoadHandler {
    //pipeline_update_tx: Sender<PipelineResourceUpdate>,
}

impl PipelineLoadHandler {
    pub fn new(
        //pipeline_update_tx: Sender<PipelineResourceUpdate>,
    ) -> Self {
        PipelineLoadHandler {
            //pipeline_update_tx,
        }
    }
}

// This sends the texture to the upload queue. The upload queue will batch uploads together when update()
// is called on it. When complete, the upload queue will send the pipeline handle back via a channel
impl ResourceLoadHandler<PipelineAsset> for PipelineLoadHandler {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<PipelineAsset>,
        version: u32,
        asset: &PipelineAsset,
        load_op: AssetLoadOp,
    ) {
        log::info!(
            "PipelineLoadHandler update_asset {} {:?} {:?}",
            version,
            load_handle,
            resource_handle
        );

        println!("LOAD PIPELINE\n{:#?}", asset);

        load_op.complete();
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<PipelineAsset>,
        version: u32,
        asset: &PipelineAsset,
    ) {

        //TODO: Need to find a way to create the pipeline
        // - Don't want to call complete() until it's created
        // - But need to access shader modules to create it
        // Options:
        // - Pipeline build queue that is given a ref to shader module asset storage on update
        // (although there is no vk::ShaderModule on the asset)
        // - Some way like an Arc<Mutex<HashMap>> for us to look up shader modules here (or DashMap)

        //TODO: I'm using AssetUuid in some places but I should be using handles instead

        //Problems
        // - Load handle is not unique to asset version
        // - Not clear how I can reference an asset from another loader
        // - Does a dependent asset reloading trigger an update/commit downstream?


        log::info!(
            "PipelineLoadHandler commit_asset_version {} {:?} {:?}",
            version,
            load_handle,
            resource_handle
        );

        //asset.pipeline_shader_stages[0].shader_module.asset_with_version()

        //asset.pipeline_shader_stages[0].shader_module.asset()

        // self.pipeline_update_tx.send(PipelineResourceUpdate {
        //     asset_uuid: *asset_uuid,
        //     resource_handle: resource_handle,
        //     images: asset.images.clone(),
        // });
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<PipelineAsset>,
        version: u32,
    ) {
        log::info!(
            "PipelineLoadHandler free {:?} {:?}",
            load_handle,
            resource_handle
        );

        //TODO: We are not unloading pipelines
    }
}
