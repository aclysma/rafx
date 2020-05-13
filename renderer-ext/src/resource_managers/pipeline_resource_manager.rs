use std::mem;
use ash::{vk, Device};
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{
    VkDevice, VkUpload, VkTransferUpload, VkTransferUploadState, VkResourceDropSink,
    VkDescriptorPoolAllocator, VkDeviceContext,
};
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;
use atelier_assets::core::AssetUuid;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use std::process::exit;
use image::{GenericImageView, ImageFormat, load};
use ash::vk::{ShaderStageFlags, DescriptorPoolResetFlags};
use crate::image_utils::DecodedTexture;
use std::collections::VecDeque;
use imgui::FontSource::DefaultFontData;
use crossbeam_channel::{Receiver, Sender};
use std::time::Duration;
use std::error::Error;
use std::num::Wrapping;
use itertools::max;
use renderer_shell_vulkan::cleanup::CombinedDropSink;
use crate::asset_storage::ResourceHandle;
use fnv::FnvHashMap;


/// Represents an image that will replace another image
pub struct PipelineResourceUpdate {
    pub asset_uuid: AssetUuid,
    pub resource_handle: ResourceHandle<PipelineAsset>,

    // data goes here
}

/// Represents the current state of the sprite and the GPU resources associated with it
pub struct PipelineResource {
    // data goes here
}

/// Keeps track of sprites/images and manages descriptor sets that allow shaders to bind to images
/// and use them
pub struct PipelineResourceManager {
    device_context: VkDeviceContext,

    // The raw texture resources
    pipelines_by_uuid: FnvHashMap<AssetUuid, ResourceHandle<PipelineAsset>>,
    pipelines: Vec<Option<PipelineResource>>,
    drop_sink: CombinedDropSink,

    // For sending image updates in a thread-safe manner
    pipeline_update_tx: Sender<PipelineResourceUpdate>,
    pipeline_update_rx: Receiver<PipelineResourceUpdate>,
}

impl PipelineResourceManager {
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> VkResult<Self> {
        let pipelines_by_uuid = Default::default();
        let pipelines = Vec::new();

        let (pipeline_update_tx, pipeline_update_rx) = crossbeam_channel::unbounded();

        let drop_sink = CombinedDropSink::new(max_frames_in_flight + 1);

        Ok(PipelineResourceManager {
            device_context: device_context.clone(),
            pipelines_by_uuid,
            pipelines,
            drop_sink,
            pipeline_update_tx,
            pipeline_update_rx,
        })
    }

    pub fn pipeline_by_handle(
        &self,
        resource_handle: ResourceHandle<ImageAsset>,
    ) -> Option<&PipelineResource> {
        //TODO: Stale handle detection?
        self.pipelines[resource_handle.index() as usize].as_ref()
    }

    pub fn pipeline_handle_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<ResourceHandle<PipelineAsset>> {
        self.pipelines_by_uuid.get(asset_uuid).map(|x| *x)
    }

    pub fn pipeline_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<&PipelineResource> {
        self.pipelines_by_uuid
            .get(asset_uuid)
            .and_then(|handle| self.pipelines[handle.index() as usize].as_ref())
    }

    pub fn pipeline_update_tx(&self) -> &Sender<PipelineResourceUpdate> {
        &self.pipeline_update_tx
    }

    pub fn update(
        &mut self,
        image_resource_manager: &ImageResourceManager
    ) {
        // This will handle any resources that need to be dropped
        self.drop_sink
            .on_frame_complete(self.device_context.device());

        // Check if we have any image updates to process
        self.apply_pipeline_updates(image_resource_manager);
    }

    /// Checks if there are pending image updates, and if there are, regenerates the descriptor sets
    fn apply_pipeline_updates(
        &mut self,
        image_resource_manager: &ImageResourceManager
    ) {
        let mut updates = Vec::with_capacity(self.pipeline_update_rx.len());
        while let Ok(update) = self.pipeline_update_rx.recv_timeout(Duration::from_secs(0)) {
            updates.push(update);
        }

        if !updates.is_empty() {
            self.do_apply_pipeline_updates(updates, image_resource_manager);
        }
    }

    /// Runs through the incoming image updates and applies them to the list of sprites
    fn do_apply_pipeline_updates(
        &mut self,
        updates: Vec<PipelineResourceUpdate>,
        image_resource_manager: &ImageResourceManager
    ) {
        let mut max_index = self.pipelines.len();
        for update in &updates {
            max_index = max_index.max(update.resource_handle.index() as usize + 1);
        }

        self.pipelines.resize_with(max_index, || None);

        for update in updates {
            self.pipelines_by_uuid.entry(update.asset_uuid).or_insert(update.resource_handle);

            let image_handle = update.image_uuid.and_then(|image_uuid| {
                image_resource_manager.image_handle_by_uuid(&image_uuid)
            });

            // Do a swap so if there is an old sprite we can properly destroy it
            self.pipelines[update.resource_handle.index() as usize] = Some(PipelineResource {
                image_handle
            });
        }
    }
}

impl Drop for PipelineResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying PipelineResourceManager");

        self.drop_sink.destroy(self.device_context.device());

        log::debug!("destroyed PipelineResourceManager");
    }
}
