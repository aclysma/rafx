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
use imgui::Image;
use std::error::Error;
use std::num::Wrapping;
use itertools::max;
use renderer_shell_vulkan::cleanup::CombinedDropSink;
use crate::asset_storage::ResourceHandle;
use fnv::FnvHashMap;
use crate::pipeline::image::ImageAsset;
use crate::resource_managers::ImageResourceManager;
use crate::pipeline::sprite::SpriteAsset;

/// Represents an image that will replace another image
pub struct SpriteResourceUpdate {
    //pub images: Vec<ManuallyDrop<VkImage>>,

    pub asset_uuid: AssetUuid,
    pub resource_handle: ResourceHandle<SpriteAsset>,

    pub images: Vec<AssetUuid>,
}

/// Represents the current state of the sprite and the GPU resources associated with it
pub struct SpriteResource {
    // pub image: ManuallyDrop<VkImage>,
    // pub image_view: vk::ImageView,

    //TODO: Link to frames instead of images and generate global frame indices that index into
    // the descriptor pool
    pub image_handles: Vec<Option<ResourceHandle<ImageAsset>>>
}

/// Keeps track of sprites/images and manages descriptor sets that allow shaders to bind to images
/// and use them
pub struct SpriteResourceManager {
    device_context: VkDeviceContext,

    // The raw texture resources
    sprites_by_uuid: FnvHashMap<AssetUuid, ResourceHandle<SpriteAsset>>,
    sprites: Vec<Option<SpriteResource>>,
    drop_sink: CombinedDropSink,

    // The descriptor set layout, pools, and sets
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool_allocator: VkDescriptorPoolAllocator,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    // For sending sprite updates in a thread-safe manner
    sprite_update_tx: Sender<SpriteResourceUpdate>,
    sprite_update_rx: Receiver<SpriteResourceUpdate>,
}

impl SpriteResourceManager {
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn descriptor_sets(&self) -> &Vec<vk::DescriptorSet> {
        &self.descriptor_sets
    }

    pub fn sprite_by_handle(
        &self,
        resource_handle: ResourceHandle<SpriteAsset>,
    ) -> Option<&SpriteResource> {
        //TODO: Stale handle detection?
        self.sprites[resource_handle.index() as usize].as_ref()
    }

    pub fn sprite_handle_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<ResourceHandle<SpriteAsset>> {
        self.sprites_by_uuid.get(asset_uuid).map(|x| *x)
    }

    pub fn sprite_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<&SpriteResource> {
        self.sprites_by_uuid
            .get(asset_uuid)
            .and_then(|handle| self.sprites[handle.index() as usize].as_ref())
    }

    pub fn sprite_update_tx(&self) -> &Sender<SpriteResourceUpdate> {
        &self.sprite_update_tx
    }

    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
        image_resource_manager: &ImageResourceManager
    ) -> VkResult<Self> {
        let sprites_by_uuid = Default::default();
        let sprites = Vec::new();

        //
        // Descriptors
        //
        let descriptor_set_layout = Self::create_descriptor_set_layout(device_context.device())?;
        let mut descriptor_pool_allocator = VkDescriptorPoolAllocator::new(
            max_frames_in_flight,
            max_frames_in_flight + 1,
            |device| Self::create_descriptor_pool(device),
        );
        let descriptor_pool = descriptor_pool_allocator.allocate_pool(device_context.device())?;
        let descriptor_sets = Self::create_descriptor_set(
            device_context.device(),
            descriptor_set_layout,
            &descriptor_pool,
            &sprites,
            image_resource_manager
        )?;

        let (image_update_tx, image_update_rx) = crossbeam_channel::unbounded();

        let drop_sink = CombinedDropSink::new(max_frames_in_flight + 1);

        Ok(SpriteResourceManager {
            device_context: device_context.clone(),
            descriptor_set_layout,
            descriptor_pool_allocator,
            descriptor_pool,
            descriptor_sets,
            sprites_by_uuid,
            sprites,
            drop_sink,
            sprite_update_tx: image_update_tx,
            sprite_update_rx: image_update_rx,
        })
    }

    pub fn update(&mut self, image_resource_manager: &ImageResourceManager) {
        // This will handle any resources that need to be dropped
        self.descriptor_pool_allocator
            .update(self.device_context.device());
        self.drop_sink
            .on_frame_complete(self.device_context.device());

        // Check if we have any sprite updates to process
        self.apply_sprite_updates(image_resource_manager);
    }

    /// Checks if there are pending image updates, and if there are, regenerates the descriptor sets
    fn apply_sprite_updates(&mut self, image_resource_manager: &ImageResourceManager) {
        let mut updates = Vec::with_capacity(self.sprite_update_rx.len());
        while let Ok(update) = self.sprite_update_rx.recv_timeout(Duration::from_secs(0)) {
            updates.push(update);
        }

        if !updates.is_empty() {
            self.do_apply_sprite_updates(updates, image_resource_manager);
            self.refresh_descriptor_sets(image_resource_manager);
        }
    }

    /// Runs through the incoming image updates and applies them to the list of sprites
    fn do_apply_sprite_updates(
        &mut self,
        updates: Vec<SpriteResourceUpdate>,
        image_resource_manager: &ImageResourceManager
    ) {
        let mut max_index = self.sprites.len();
        for update in &updates {
            max_index = max_index.max(update.resource_handle.index() as usize + 1);
        }

        self.sprites.resize_with(max_index, || None);

        for update in updates {
            self.sprites_by_uuid.entry(update.asset_uuid).or_insert(update.resource_handle);
            let mut image_handles = Vec::with_capacity(update.images.len());
            for image_uuid in update.images {
                image_handles.push(image_resource_manager.image_handle_by_uuid(&image_uuid));
            }

            self.sprites[update.resource_handle.index() as usize] = Some(SpriteResource {
                image_handles
            });
        }
    }

    fn refresh_descriptor_sets(
        &mut self,
        image_resource_manager: &ImageResourceManager
    ) -> VkResult<()> {
        self.descriptor_pool_allocator
            .retire_pool(self.descriptor_pool);

        let descriptor_pool = self
            .descriptor_pool_allocator
            .allocate_pool(self.device_context.device())?;
        let descriptor_sets = Self::create_descriptor_set(
            self.device_context.device(),
            self.descriptor_set_layout,
            &descriptor_pool,
            &self.sprites,
            image_resource_manager
        )?;

        self.descriptor_pool = descriptor_pool;
        self.descriptor_sets = descriptor_sets;

        Ok(())
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

    fn create_descriptor_set_layout(
        logical_device: &ash::Device
    ) -> VkResult<vk::DescriptorSetLayout> {
        let descriptor_set_layout_bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build()];

        let descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::builder().bindings(&descriptor_set_layout_bindings);

        unsafe {
            logical_device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
        }
    }

    fn create_descriptor_pool(logical_device: &ash::Device) -> VkResult<vk::DescriptorPool> {
        const MAX_TEXTURES: u32 = 1000;

        let pool_sizes = [vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(MAX_TEXTURES)
            .build()];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(MAX_TEXTURES);

        unsafe { logical_device.create_descriptor_pool(&descriptor_pool_info, None) }
    }

    fn create_descriptor_set(
        logical_device: &ash::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
        descriptor_pool: &vk::DescriptorPool,
        sprites: &[Option<SpriteResource>],
        image_resource_manager: &ImageResourceManager
    ) -> VkResult<Vec<vk::DescriptorSet>> {
        let descriptor_set_layouts = vec![descriptor_set_layout; sprites.len()];

        let descriptor_sets = if !sprites.is_empty() {
            let alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*descriptor_pool)
                .set_layouts(descriptor_set_layouts.as_slice());

            unsafe { logical_device.allocate_descriptor_sets(&alloc_info) }?
        } else {
            vec![]
        };

        for (image_index, sprite) in sprites.iter().enumerate() {
            if let Some(sprite) = sprite.as_ref() {
                for image_handle in &sprite.image_handles {
                    if let Some(image_handle) = image_handle {
                        if let Some(image_resource) = image_resource_manager.image_by_handle(*image_handle) {
                            let mut descriptor_writes = Vec::with_capacity(sprites.len());
                            let image_view_descriptor_image_info = vk::DescriptorImageInfo::builder()
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                .image_view(image_resource.image_view)
                                .build();

                            descriptor_writes.push(
                                vk::WriteDescriptorSet::builder()
                                    .dst_set(descriptor_sets[image_index])
                                    .dst_binding(0)
                                    .dst_array_element(0)
                                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                                    .image_info(&[image_view_descriptor_image_info])
                                    .build(),
                            );
                            unsafe {
                                logical_device.update_descriptor_sets(&descriptor_writes, &[]);
                            }
                        }
                    }
                }
            }
        }

        Ok(descriptor_sets)
    }
}

impl Drop for SpriteResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying VkSpriteResourceManager");

        unsafe {
            self.descriptor_pool_allocator
                .retire_pool(self.descriptor_pool);
            self.descriptor_pool_allocator
                .destroy(self.device_context.device());

            self.device_context
                .device()
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            self.drop_sink.destroy(self.device_context.device());
        }

        log::debug!("destroyed VkSpriteResourceManager");
    }
}
