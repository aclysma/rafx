use std::mem;
use ash::{vk, Device};
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{VkDevice, VkUpload, VkTransferUpload, VkTransferUploadState, VkResourceDropSink, VkDescriptorPoolAllocator};
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use std::process::exit;
use image::{GenericImageView, ImageFormat, load};
use ash::vk::{ShaderStageFlags, DescriptorPoolResetFlags};
use crate::image_utils::DecodedTexture;
use std::collections::VecDeque;
use imgui::FontSource::DefaultFontData;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::time::Duration;
use imgui::Image;
use std::error::Error;
use std::num::Wrapping;
use itertools::max;
use renderer_shell_vulkan::cleanup::CombinedDropSink;
use crate::asset_storage::ResourceHandle;
use crate::image_importer::ImageAsset;

/*
#[derive(Debug)]
pub enum LoadingSpritePollResult {
    Pending,
    Complete(Vec<ManuallyDrop<VkImage>>),
    Error(Box<Error + 'static + Send>),
    Destroyed
}

struct LoadingSpriteInner {
    images: Vec<ManuallyDrop<VkImage>>,
    uploader: VkTransferUpload,
    load_op: atelier_assets::loader::AssetLoadOp
}


pub struct LoadingSprite {
    inner: Option<LoadingSpriteInner>
}

impl LoadingSprite {
    pub fn new(
        images: Vec<ManuallyDrop<VkImage>>,
        uploader: VkTransferUpload,
        load_op: atelier_assets::loader::AssetLoadOp
    ) -> Self {
        let inner = LoadingSpriteInner {
            images,
            uploader,
            load_op
        };

        LoadingSprite {
            inner: Some(inner)
        }
    }

    pub fn poll_load(
        &mut self,
        device: &VkDevice
    ) -> LoadingSpritePollResult {
        loop {
            if let Some(mut inner) = self.take_inner() {
                match inner.uploader.state() {
                    Ok(state) => {
                        match state {
                            VkTransferUploadState::Writable => {
                                println!("VkTransferUploadState::Writable");
                                inner.uploader.submit_transfer(device.queues.transfer_queue);
                                self.inner = Some(inner);
                            },
                            VkTransferUploadState::SentToTransferQueue => {
                                println!("VkTransferUploadState::SentToTransferQueue");
                                self.inner = Some(inner);
                                break LoadingSpritePollResult::Pending;
                            },
                            VkTransferUploadState::PendingSubmitDstQueue => {
                                println!("VkTransferUploadState::PendingSubmitDstQueue");
                                inner.uploader.submit_dst(device.queues.graphics_queue);
                                self.inner = Some(inner);
                            },
                            VkTransferUploadState::SentToDstQueue => {
                                println!("VkTransferUploadState::SentToDstQueue");
                                self.inner = Some(inner);
                                break LoadingSpritePollResult::Pending;
                            },
                            VkTransferUploadState::Complete => {
                                println!("VkTransferUploadState::Complete");
                                inner.load_op.complete();
                                break LoadingSpritePollResult::Complete(inner.images);
                            },
                        }
                    },
                    Err(err) => {
                        inner.load_op.error(err);
                        break LoadingSpritePollResult::Error(Box::new(err));
                    },
                }
            } else {
                break LoadingSpritePollResult::Destroyed;
            }
        }
    }

    fn take_inner(&mut self) -> Option<LoadingSpriteInner> {
        let mut inner = None;
        std::mem::swap(&mut self.inner, &mut inner);
        inner
    }
}

impl Drop for LoadingSprite {
    fn drop(&mut self) {
        if let Some(mut inner) = self.take_inner() {
            for image in &mut inner.images {
                unsafe {
                    ManuallyDrop::drop(image);
                }
            }
            //TODO: error() probably needs to accept a box
            inner.load_op.error(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY);
        }
    }
}
*/

pub struct SpriteUpdate {
    pub images: Vec<ManuallyDrop<VkImage>>,
    pub resource_handles: Vec<ResourceHandle<ImageAsset>>
}

pub struct VkSprite {
    pub image: ManuallyDrop<VkImage>,
    pub image_view: vk::ImageView
}

pub struct VkSpriteResourceManager {
    device: ash::Device,
    swapchain_info: SwapchainInfo,

    // The raw texture resources
    sprites: Vec<Option<VkSprite>>,
    drop_sink: CombinedDropSink,

    // The descriptor set layout, pools, and sets
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool_allocator: VkDescriptorPoolAllocator,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    sprite_update_tx: Sender<SpriteUpdate>,
    sprite_update_rx: Receiver<SpriteUpdate>

    //pub pending_sprite_updates: Vec<VkSpriteUpdate>

    // pub loading_sprite_tx: Sender<LoadingSprite>,
    // pub loading_sprite_rx: Receiver<LoadingSprite>,
    // pub loading_sprites: Vec<LoadingSprite>
}

impl VkSpriteResourceManager {
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn descriptor_sets(&self) -> &Vec<vk::DescriptorSet> {
        &self.descriptor_sets
    }

    pub fn sprite_update_tx(&self) -> &Sender<SpriteUpdate> {
        &self.sprite_update_tx
    }

    fn do_update_sprites(&mut self, sprite_update: SpriteUpdate) {
        let mut max_index = self.sprites.len();
        for resource_handle in &sprite_update.resource_handles {
            max_index = max_index.max(resource_handle.index() as usize + 1);
        }

        self.sprites.resize_with(max_index, || None);

        for (i, image) in sprite_update.images.into_iter().enumerate() {
            let resource_handle = sprite_update.resource_handles[i];

            let image_view = Self::create_texture_image_view(&self.device, &image.image);
            self.sprites[resource_handle.index() as usize] = Some(VkSprite {
                image,
                image_view
            });
        }
    }

    fn try_update_sprites(&mut self) {
        let mut has_update = false;
        while let Ok(update) = self.sprite_update_rx.recv_timeout(Duration::from_secs(0)) {
            self.do_update_sprites(update);
            has_update = true;
        }

        if has_update {
            self.refresh_descriptor_sets();
        }
    }

    // pub fn sprite_update_tx(&self) -> &Sender<SpriteUpdate> {
    //     &self.sprite_update_tx
    // }

    // pub fn update_sprites(&mut self, images: Vec<ManuallyDrop<VkImage>>, resource_handles: Vec<ResourceHandle<ImageAsset>>) {
    //     self.pending_sprite_updates.push(SpriteUpdate {
    //         images,
    //         resource_handles
    //     });
    // }

    /*
    pub fn set_images(&mut self, images: Vec<ManuallyDrop<VkImage>>) -> VkResult<()> {
        let mut sprites = Vec::with_capacity(images.len());
        for image in images {
            let image_view = Self::create_texture_image_view(&self.device, &image.image);
            sprites.push(VkSprite {
                image,
                image_view
            });
        }

        std::mem::swap(&mut sprites, &mut self.sprites);
        self.refresh_descriptor_set()?;

        for sprite in sprites.drain(..) {
            self.drop_sink.retire_image(sprite.image);
            self.drop_sink.retire_image_view(sprite.image_view);
        }

        Ok(())
    }

    pub fn loading_sprite_tx(&self) -> &Sender<LoadingSprite> {
        &self.loading_sprite_tx
    }
*/

    fn refresh_descriptor_sets(&mut self) -> VkResult<()> {
        self.descriptor_pool_allocator.retire_pool(self.descriptor_pool);

        let descriptor_pool = self.descriptor_pool_allocator.allocate_pool(&self.device)?;
        let descriptor_sets = Self::create_descriptor_set(
            &self.device,
            &descriptor_pool,
            self.descriptor_set_layout,
            &self.sprites
        )?;

        self.descriptor_pool = descriptor_pool;
        self.descriptor_sets = descriptor_sets;

        Ok(())
    }

    pub fn update(&mut self, device: &VkDevice) {
        self.descriptor_pool_allocator.update(&self.device);
        self.drop_sink.on_frame_complete(&self.device);
        self.try_update_sprites();

        /*
        for loading_sprite in self.loading_sprite_rx.recv_timeout(Duration::from_secs(0)) {
            self.loading_sprites.push(loading_sprite);
        }

        for i in (0..self.loading_sprites.len()).rev() {
            let result = self.loading_sprites[i].poll_load(device);
            match result {
                LoadingSpritePollResult::Pending => {
                    // do nothing
                },
                LoadingSpritePollResult::Complete(images) => {
                    let loading_sprite = self.loading_sprites.swap_remove(i);
                    self.set_images(images);
                    self.refresh_descriptor_set();
                },
                LoadingSpritePollResult::Error(e) => {
                    let image = self.loading_sprites.swap_remove(i);
                    //TODO: error() probably needs to accept a box
                    //image.load_op.error(e);
                },
                LoadingSpritePollResult::Destroyed => {
                    // not expected
                    unreachable!();
                }
            }
        }
        */
    }

    pub fn new(
        device: &VkDevice,
        //swapchain: &VkSwapchain,
        swapchain_info: SwapchainInfo
    ) -> VkResult<Self> {
        // let decoded_textures = [
        //     //crate::image_utils::decode_texture(include_bytes!("../../../../assets/textures/texture.jpg"), image::ImageFormat::Jpeg),
        //     //decode_texture(include_bytes!("../../../../assets/textures/texture2.jpg"), image::ImageFormat::Jpeg),
        //     //decode_texture(include_bytes!("../../../../texture.jpg"), image::ImageFormat::Jpeg),
        // ];

        //
        // Resources
        //
        // let images = crate::image_utils::load_images(
        //     &device.context,
        //     device.queue_family_indices.transfer_queue_family_index,
        //     device.queues.transfer_queue,
        //     device.queue_family_indices.graphics_queue_family_index,
        //     device.queues.graphics_queue,
        //     &decoded_textures
        // )?;

        //let mut image_views = Vec::with_capacity(decoded_textures.len());
        //let mut sprites = Vec::with_capacity(images.len());
        // for image in images {
        //     //image_views.push(Self::create_texture_image_view(device.device(), &image.image));
        //     let image_view = Self::create_texture_image_view(device.device(), &image.image);
        //     sprites.push(Some(VkSprite {
        //         image,
        //         image_view
        //     }));
        // }

        let sprites = Vec::new();

        //
        // Descriptors
        //
        let descriptor_set_layout = Self::create_descriptor_set_layout(device.device())?;
        let mut descriptor_pool_allocator = VkDescriptorPoolAllocator::new(
            swapchain_info.image_count as u32,
            swapchain_info.image_count as u32 + 1,
            |device| Self::create_descriptor_pool(device)
        );
        let descriptor_pool = descriptor_pool_allocator.allocate_pool(device.device())?;
        let descriptor_sets = Self::create_descriptor_set(
            device.device(),
            &descriptor_pool,
            descriptor_set_layout,
            &sprites
        )?;

        let (sprite_update_tx, sprite_update_rx) = mpsc::channel();

        let drop_sink = CombinedDropSink::new(swapchain_info.image_count as u32 + 1);

        Ok(VkSpriteResourceManager {
            device: device.device().clone(),
            swapchain_info,
            //command_pool,
            descriptor_set_layout,
            descriptor_pool_allocator,
            descriptor_pool,
            descriptor_sets,
            sprites,
            drop_sink,
            sprite_update_tx,
            sprite_update_rx
            // loading_sprite_tx,
            // loading_sprite_rx,
            // loading_sprites: Default::default()
        })
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
        let descriptor_set_layout_bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
        ];

        let descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::builder().bindings(&descriptor_set_layout_bindings);

        unsafe {
            logical_device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
        }
    }

    fn create_descriptor_pool(
        logical_device: &ash::Device,
    ) -> VkResult<vk::DescriptorPool> {
        const MAX_TEXTURES : u32 = 1000;

        let pool_sizes = [
            vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(MAX_TEXTURES)
                .build(),
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(MAX_TEXTURES);

        unsafe { logical_device.create_descriptor_pool(&descriptor_pool_info, None) }
    }

    fn create_descriptor_set(
        logical_device: &ash::Device,
        descriptor_pool: &vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
        sprites: &[Option<VkSprite>],
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
                        .build()
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
            self.descriptor_pool_allocator.retire_pool(self.descriptor_pool);
            self.descriptor_pool_allocator.destroy(&self.device);

            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            for sprite in self.sprites.drain(..) {
                if let Some(sprite) = sprite {
                    self.drop_sink.retire_image(sprite.image);
                    self.drop_sink.retire_image_view(sprite.image_view);
                }
            }
            self.drop_sink.destroy(&self.device);

            //self.loading_sprites.clear();
        }

        log::debug!("destroyed VkSpriteResourceManager");
    }
}

