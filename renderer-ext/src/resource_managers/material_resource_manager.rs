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
use crate::pipeline::image::ImageAsset;
use crate::pipeline::gltf::MaterialAsset;
use crate::resource_managers::ImageResourceManager;

/// Represents an image that will replace another image
pub struct MaterialResourceUpdate {
    pub asset_uuid: AssetUuid,
    pub resource_handle: ResourceHandle<MaterialAsset>,

    pub image_uuid: Option<AssetUuid>,
}

/// Represents the current state of the sprite and the GPU resources associated with it
pub struct MaterialResource {
    pub image_handle: Option<ResourceHandle<ImageAsset>>,
}

/// Keeps track of sprites/images and manages descriptor sets that allow shaders to bind to images
/// and use them
pub struct MaterialResourceManager {
    device_context: VkDeviceContext,

    // The raw texture resources
    materials_by_uuid: FnvHashMap<AssetUuid, ResourceHandle<MaterialAsset>>,
    materials: Vec<Option<MaterialResource>>,
    drop_sink: CombinedDropSink,

    // For sending image updates in a thread-safe manner
    material_update_tx: Sender<MaterialResourceUpdate>,
    material_update_rx: Receiver<MaterialResourceUpdate>,
}

impl MaterialResourceManager {
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> VkResult<Self> {
        let materials_by_uuid = Default::default();
        let materials = Vec::new();

        let (material_update_tx, material_update_rx) = crossbeam_channel::unbounded();

        let drop_sink = CombinedDropSink::new(max_frames_in_flight + 1);

        Ok(MaterialResourceManager {
            device_context: device_context.clone(),
            materials_by_uuid,
            materials,
            drop_sink,
            material_update_tx,
            material_update_rx,
        })
    }

    pub fn material_by_handle(
        &self,
        resource_handle: ResourceHandle<ImageAsset>,
    ) -> Option<&MaterialResource> {
        //TODO: Stale handle detection?
        self.materials[resource_handle.index() as usize].as_ref()
    }

    pub fn material_handle_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<ResourceHandle<MaterialAsset>> {
        self.materials_by_uuid.get(asset_uuid).map(|x| *x)
    }

    pub fn material_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<&MaterialResource> {
        self.materials_by_uuid
            .get(asset_uuid)
            .and_then(|handle| self.materials[handle.index() as usize].as_ref())
    }

    pub fn material_update_tx(&self) -> &Sender<MaterialResourceUpdate> {
        &self.material_update_tx
    }

    pub fn update(
        &mut self,
        image_resource_manager: &ImageResourceManager
    ) {
        // This will handle any resources that need to be dropped
        self.drop_sink
            .on_frame_complete(self.device_context.device());

        // Check if we have any image updates to process
        self.apply_material_updates(image_resource_manager);
    }

    /// Checks if there are pending image updates, and if there are, regenerates the descriptor sets
    fn apply_material_updates(
        &mut self,
        image_resource_manager: &ImageResourceManager
    ) {
        let mut updates = Vec::with_capacity(self.material_update_rx.len());
        while let Ok(update) = self.material_update_rx.recv_timeout(Duration::from_secs(0)) {
            updates.push(update);
        }

        if !updates.is_empty() {
            self.do_apply_material_updates(updates, image_resource_manager);
        }
    }

    /// Runs through the incoming image updates and applies them to the list of sprites
    fn do_apply_material_updates(
        &mut self,
        updates: Vec<MaterialResourceUpdate>,
        image_resource_manager: &ImageResourceManager
    ) {
        let mut max_index = self.materials.len();
        for update in &updates {
            max_index = max_index.max(update.resource_handle.index() as usize + 1);
        }

        self.materials.resize_with(max_index, || None);

        for update in updates {
            self.materials_by_uuid.entry(update.asset_uuid).or_insert(update.resource_handle);

            let image_handle = update.image_uuid.and_then(|image_uuid| {
                image_resource_manager.image_handle_by_uuid(&image_uuid)
            });

            // Do a swap so if there is an old sprite we can properly destroy it
            self.materials[update.resource_handle.index() as usize] = Some(MaterialResource {
                image_handle
            });
        }
    }

    pub fn create_texture_image_view(
        logical_device: &ash::Device,
        image: &vk::Image,
    ) -> vk::ImageView {
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let image_view_info = vk::ImageViewCreateInfo::builder()
            .image(*image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(*subresource_range);

        unsafe {
            logical_device
                .create_image_view(&image_view_info, None)
                .unwrap()
        }
    }
}

impl Drop for MaterialResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying MaterialResourceManager");

        self.drop_sink.destroy(self.device_context.device());

        log::debug!("destroyed MaterialResourceManager");
    }
}
