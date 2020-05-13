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
use std::process::exit;
use ash::vk::{ShaderStageFlags, DescriptorPoolResetFlags};
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
use crate::resource_managers::ImageResourceManager;
use crate::pipeline::shader::ShaderAsset;

/// Represents an incoming update to a shader
pub struct ShaderResourceUpdate {
    pub asset_uuid: AssetUuid,
    pub resource_handle: ResourceHandle<ShaderAsset>,

    pub shader_module: vk::ShaderModule,
}

/// Represents a loaded shader
pub struct ShaderResource {
    pub shader_module: vk::ShaderModule,
}

/// Keeps track of loaded shaders
pub struct ShaderResourceManager {
    device_context: VkDeviceContext,

    // The raw texture resources
    shaders_by_uuid: FnvHashMap<AssetUuid, ResourceHandle<ShaderAsset>>,
    shaders: Vec<Option<ShaderResource>>,

    // For sending shader updates in a thread-safe manner
    shader_update_tx: Sender<ShaderResourceUpdate>,
    shader_update_rx: Receiver<ShaderResourceUpdate>,
}

impl ShaderResourceManager {
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> VkResult<Self> {
        let shaders_by_uuid = Default::default();
        let shaders = Vec::new();

        let (shader_update_tx, shader_update_rx) = crossbeam_channel::unbounded();

        Ok(ShaderResourceManager {
            device_context: device_context.clone(),
            shaders_by_uuid,
            shaders,
            shader_update_tx,
            shader_update_rx,
        })
    }

    pub fn shader_by_handle(
        &self,
        resource_handle: ResourceHandle<ShaderAsset>,
    ) -> Option<&ShaderResource> {
        //TODO: Stale handle detection?
        self.shaders[resource_handle.index() as usize].as_ref()
    }

    pub fn shader_handle_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<ResourceHandle<ShaderAsset>> {
        self.shaders_by_uuid.get(asset_uuid).map(|x| *x)
    }

    pub fn shader_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<&ShaderResource> {
        self.shaders_by_uuid
            .get(asset_uuid)
            .and_then(|handle| self.shaders[handle.index() as usize].as_ref())
    }

    pub fn shader_update_tx(&self) -> &Sender<ShaderResourceUpdate> {
        &self.shader_update_tx
    }

    pub fn update(
        &mut self,
    ) {
        self.apply_shader_updates();
    }

    /// Checks if there are pending shader updates, and if there are, regenerates the descriptor sets
    fn apply_shader_updates(
        &mut self,
    ) {
        let mut updates = Vec::with_capacity(self.shader_update_rx.len());
        while let Ok(update) = self.shader_update_rx.recv_timeout(Duration::from_secs(0)) {
            updates.push(update);
        }

        if !updates.is_empty() {
            self.do_apply_shader_updates(updates);
        }
    }

    /// Runs through the incoming shader updates and applies them to the list of sprites
    fn do_apply_shader_updates(
        &mut self,
        updates: Vec<ShaderResourceUpdate>
    ) {
        let mut max_index = self.shaders.len();
        for update in &updates {
            max_index = max_index.max(update.resource_handle.index() as usize + 1);
        }

        self.shaders.resize_with(max_index, || None);

        for update in updates {
            self.shaders_by_uuid.entry(update.asset_uuid).or_insert(update.resource_handle);

            // Do a swap so if there is an old sprite we can properly destroy it
            self.shaders[update.resource_handle.index() as usize] = Some(ShaderResource {
                shader_module: update.shader_module
            });
        }
    }
}

impl Drop for ShaderResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying ShaderResourceManager");

        for shader in &mut self.shaders {
            if let Some(shader) = shader {
                unsafe {
                    self.device_context.device().destroy_shader_module(shader.shader_module, None);
                }
            }
        }

        log::debug!("destroyed ShaderResourceManager");
    }
}
