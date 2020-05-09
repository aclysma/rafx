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
use crate::image_importer::ImageAsset;
use fnv::FnvHashMap;

/// Represents an image that will replace another image
pub struct ImageUpdate {
    pub images: Vec<ManuallyDrop<VkImage>>,
    pub resource_handles: Vec<ResourceHandle<ImageAsset>>,
}

/// Represents the current state of the sprite and the GPU resources associated with it
pub struct Sprite {
    pub image: ManuallyDrop<VkImage>,
    pub image_view: vk::ImageView,
}

/// Keeps track of sprites/images and manages descriptor sets that allow shaders to bind to images
/// and use them
pub struct VkSpriteResourceManager {
    device_context: VkDeviceContext,

    // The raw texture resources
    sprites_lookup: FnvHashMap<AssetUuid, ResourceHandle<ImageAsset>>,
    sprites: Vec<Option<Sprite>>,
    drop_sink: CombinedDropSink,

    // The descriptor set layout, pools, and sets
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool_allocator: VkDescriptorPoolAllocator,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    // For sending image updates in a thread-safe manner
    image_update_tx: Sender<ImageUpdate>,
    image_update_rx: Receiver<ImageUpdate>,
}

impl VkSpriteResourceManager {
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn descriptor_sets(&self) -> &Vec<vk::DescriptorSet> {
        &self.descriptor_sets
    }

    pub fn sprite_by_handle(&self, resource_handle: ResourceHandle<ImageAsset>) -> Option<&Sprite> {
        //TODO: Stale handle detection?
        self.sprites[resource_handle.index() as usize].as_ref()
    }

    pub fn sprite_handle_by_uuid(&self, asset_uuid: &AssetUuid) -> Option<ResourceHandle<ImageAsset>> {
        self.sprites_lookup.get(asset_uuid).map(|x| *x)
    }

    pub fn sprite_by_uuid(&self, asset_uuid: &AssetUuid) -> Option<&Sprite> {
        self.sprites_lookup
            .get(asset_uuid)
            .and_then(|handle| self.sprites[handle.index() as usize].as_ref())
    }

    pub fn image_update_tx(&self) -> &Sender<ImageUpdate> {
        &self.image_update_tx
    }

    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> VkResult<Self> {
        let sprites_lookup = Default::default();
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
        )?;

        let (image_update_tx, image_update_rx) = crossbeam_channel::unbounded();

        let drop_sink = CombinedDropSink::new(max_frames_in_flight + 1);

        Ok(VkSpriteResourceManager {
            device_context: device_context.clone(),
            descriptor_set_layout,
            descriptor_pool_allocator,
            descriptor_pool,
            descriptor_sets,
            sprites_lookup,
            sprites,
            drop_sink,
            image_update_tx,
            image_update_rx,
        })
    }

    pub fn update(&mut self) {
        // This will handle any resources that need to be dropped
        self.descriptor_pool_allocator
            .update(self.device_context.device());
        self.drop_sink
            .on_frame_complete(self.device_context.device());

        // Check if we have any image updates to process
        //TODO: This may need to be deferred until a commit, and the commit may be to update to a
        // particular version of the assets
        self.try_update_sprites();
    }

    /// Runs through the incoming image updates and applies them to the list of sprites
    fn do_update_sprites(
        &mut self,
        image_update: ImageUpdate,
    ) {
        let mut max_index = self.sprites.len();
        for resource_handle in &image_update.resource_handles {
            max_index = max_index.max(resource_handle.index() as usize + 1);
        }

        self.sprites.resize_with(max_index, || None);

        let mut old_sprites = vec![];
        for (i, image) in image_update.images.into_iter().enumerate() {
            let resource_handle = image_update.resource_handles[i];

            let image_view =
                Self::create_texture_image_view(self.device_context.device(), &image.image);

            // Do a swap so if there is an old sprite we can properly destroy it
            let mut sprite = Some(Sprite { image, image_view });
            std::mem::swap(
                &mut sprite,
                &mut self.sprites[resource_handle.index() as usize],
            );
            if sprite.is_some() {
                old_sprites.push(sprite);
            }
        }

        // retire old images/views
        for sprite in old_sprites.drain(..) {
            let sprite = sprite.unwrap();
            self.drop_sink.retire_image(sprite.image);
            self.drop_sink.retire_image_view(sprite.image_view);
        }
    }

    /// Checks if there are pending image updates, and if there are, regenerates the descriptor sets
    fn try_update_sprites(&mut self) {
        let mut has_update = false;
        while let Ok(update) = self.image_update_rx.recv_timeout(Duration::from_secs(0)) {
            self.do_update_sprites(update);
            has_update = true;
        }

        if has_update {
            self.refresh_descriptor_sets();
        }
    }

    fn refresh_descriptor_sets(&mut self) -> VkResult<()> {
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
        sprites: &[Option<Sprite>],
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
                let mut descriptor_writes = Vec::with_capacity(sprites.len());
                let image_view_descriptor_image_info = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(sprite.image_view)
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

        Ok(descriptor_sets)
    }
}

impl Drop for VkSpriteResourceManager {
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

            for sprite in self.sprites.drain(..) {
                if let Some(sprite) = sprite {
                    self.drop_sink.retire_image(sprite.image);
                    self.drop_sink.retire_image_view(sprite.image_view);
                }
            }
            self.drop_sink.destroy(self.device_context.device());
        }

        log::debug!("destroyed VkSpriteResourceManager");
    }
}
