use std::mem;
use ash::vk;
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{VkDevice, VkUpload, VkTransferUpload, VkTransferUploadState};
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


struct ImageInFlight {
    image: ManuallyDrop<VkImage>,
    frames_remaining_until_reset: u32
}

struct ImageViewInFlight {
    image_view: vk::ImageView,
    frames_remaining_until_reset: u32
}

pub struct ImageDropSink {
    in_flight_images: VecDeque<ImageInFlight>,
    in_flight_image_views: VecDeque<ImageViewInFlight>,
    max_in_flight_frames: u32,
}

impl ImageDropSink {
    pub fn new(
        max_in_flight_frames: u32
    ) -> Self {
        ImageDropSink {
            in_flight_images: Default::default(),
            in_flight_image_views: Default::default(),
            max_in_flight_frames
        }
    }

    pub fn retire_image(&mut self, image: ManuallyDrop<VkImage>) {
        self.in_flight_images.push_back(ImageInFlight {
            image,
            frames_remaining_until_reset: self.max_in_flight_frames
        });
    }


    pub fn retire_image_view(&mut self, image_view: vk::ImageView) {
        self.in_flight_image_views.push_back(ImageViewInFlight {
            image_view,
            frames_remaining_until_reset: self.max_in_flight_frames
        });
    }


    pub fn update(&mut self, device: &ash::Device) {
        {
            for image_views_in_flight in &mut self.in_flight_image_views {
                image_views_in_flight.frames_remaining_until_reset -= 1;
            }

            // Determine how many image views we can drain
            let mut image_views_to_drop = 0;
            for in_flight_image_view in &self.in_flight_image_views {
                if in_flight_image_view.frames_remaining_until_reset <= 0 {
                    image_views_to_drop += 1;
                } else {
                    break;
                }
            }

            // Reset them and add them to the list of pools ready to be allocated
            let image_views_to_drop : Vec<_> = self.in_flight_image_views.drain(0..image_views_to_drop).collect();
            for mut image_view in image_views_to_drop {
                unsafe {
                    device.destroy_image_view(image_view.image_view, None);
                }
            }
        }

        {
            // Decrease frame count by one for all retiring pools
            for image_in_flight in &mut self.in_flight_images {
                image_in_flight.frames_remaining_until_reset -= 1;
            }

            // Determine how many images we can drain
            let mut images_to_drop = 0;
            for in_flight_image in &self.in_flight_images {
                if in_flight_image.frames_remaining_until_reset <= 0 {
                    images_to_drop += 1;
                } else {
                    break;
                }
            }

            // Reset them and add them to the list of pools ready to be allocated
            let images_to_drop : Vec<_> = self.in_flight_images.drain(0..images_to_drop).collect();
            for mut image in images_to_drop {
                unsafe {
                    ManuallyDrop::drop(&mut image.image);
                }
            }
        }
    }


    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.device_wait_idle();
        }

        let images_to_drop : Vec<_> = self.in_flight_images.drain(..).collect();
        for mut image in images_to_drop {
            unsafe {
                ManuallyDrop::drop(&mut image.image);
            }
        }
    }
}



struct DescriptorPoolInFlight {
    pool: vk::DescriptorPool,
    frames_remaining_until_reset: u32
}

pub type DescriptorPoolAllocatorAllocFn = Fn(&ash::Device) -> VkResult<vk::DescriptorPool>;

pub struct DescriptorPoolAllocator {
    allocate_fn: Box<DescriptorPoolAllocatorAllocFn>,
    in_flight_pools: VecDeque<DescriptorPoolInFlight>,
    reset_pool: Vec<vk::DescriptorPool>,
    max_in_flight_frames: u32,

    // Number of pools we have created in total
    created_pool_count: u32,
    max_pool_count: u32
}

impl DescriptorPoolAllocator {
    pub fn new<F: Fn(&ash::Device) -> VkResult<vk::DescriptorPool> + 'static>(
        max_in_flight_frames: u32,
        max_pool_count: u32,
        allocate_fn: F
    ) -> Self {
        DescriptorPoolAllocator {
            allocate_fn: Box::new(allocate_fn),
            in_flight_pools: Default::default(),
            reset_pool: Default::default(),
            max_in_flight_frames,
            created_pool_count: 0,
            max_pool_count
        }
    }

    pub fn allocate_pool(&mut self, device: &ash::Device) -> VkResult<vk::DescriptorPool> {
        self.reset_pool.pop()
            .map(|pool| Ok(pool))
            .unwrap_or_else(|| {
                self.created_pool_count += 1;
                assert!(self.created_pool_count <= self.max_pool_count);
                (self.allocate_fn)(device)
            })
    }

    pub fn retire_pool(&mut self, pool: vk::DescriptorPool) {
        self.in_flight_pools.push_back(DescriptorPoolInFlight {
            pool,
            frames_remaining_until_reset: self.max_in_flight_frames
        });
    }

    pub fn update(&mut self, device: &ash::Device) {
        // Decrease frame count by one for all retiring pools
        for pool_in_flight in &mut self.in_flight_pools {
            pool_in_flight.frames_remaining_until_reset -= 1;
        }

        // Determine how many pools we can drain
        let mut pools_to_drain = 0;
        for in_flight_pool in &self.in_flight_pools {
            if in_flight_pool.frames_remaining_until_reset <= 0 {
                pools_to_drain += 1;
            } else {
                break;
            }
        }

        // Reset them and add them to the list of pools ready to be allocated
        let pools_to_reset : Vec<_> = self.in_flight_pools.drain(0..pools_to_drain).collect();
        for pool_to_reset in pools_to_reset {
            unsafe {
                device.reset_descriptor_pool(pool_to_reset.pool, DescriptorPoolResetFlags::empty());
            }

            self.reset_pool.push(pool_to_reset.pool);
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.device_wait_idle();
        }

        while !self.in_flight_pools.is_empty() {
            self.update(device);
        }

        for pool in self.reset_pool.drain(..) {
            unsafe {
                device.destroy_descriptor_pool(pool, None);
            }
        }
    }
}

impl Drop for DescriptorPoolAllocator {
    fn drop(&mut self) {
        assert!(self.in_flight_pools.is_empty());
        assert!(self.reset_pool.is_empty());
    }
}

#[derive(Debug)]
pub enum LoadingSpritePollResult {
    Pending,
    Complete,
    Error(Box<Error + 'static + Send>),
}

pub struct LoadingSprite {
    pub images: Vec<ManuallyDrop<VkImage>>,
    pub uploader: VkTransferUpload,
    pub load_op: atelier_assets::loader::AssetLoadOp
}

impl LoadingSprite {
    pub fn poll_load(
        &mut self,
        device: &VkDevice
    ) -> LoadingSpritePollResult {
        loop {
            match self.uploader.state() {
                Ok(state) => {
                    match state {
                        VkTransferUploadState::Writable => {
                            println!("VkTransferUploadState::Writable");
                            self.uploader.submit_transfer(device.queues.transfer_queue);
                        },
                        VkTransferUploadState::SentToTransferQueue => {
                            println!("VkTransferUploadState::SentToTransferQueue");
                            break LoadingSpritePollResult::Pending;
                        },
                        VkTransferUploadState::PendingSubmitDstQueue => {
                            println!("VkTransferUploadState::PendingSubmitDstQueue");
                            self.uploader.submit_dst(device.queues.graphics_queue);
                        },
                        VkTransferUploadState::SentToDstQueue => {
                            println!("VkTransferUploadState::SentToDstQueue");
                            break LoadingSpritePollResult::Pending;
                        },
                        VkTransferUploadState::Complete => {
                            println!("VkTransferUploadState::Complete");
                            //let front = self.loading_sprites.pop_front().unwrap();
                            //front.load_op.complete();
                            break LoadingSpritePollResult::Complete;
                        },
                    }
                },
                Err(err) => {
                    //let front = self.loading_sprites.pop_front().unwrap();
                    //front.load_op.error(err);
                    //break;
                    break LoadingSpritePollResult::Error(Box::new(err));
                },
            }
        }
    }
}

pub struct VkSprite {
    pub image: ManuallyDrop<VkImage>,
    pub image_view: vk::ImageView
}

pub struct VkSpriteResourceManager {
    pub device: ash::Device,
    pub swapchain_info: SwapchainInfo,

    pub command_pool: vk::CommandPool,

    // The raw texture resources
    pub sprites: Vec<VkSprite>,
    pub image_drop_sink: ImageDropSink,

    // The descriptor set layout, pools, and sets
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool_allocator: DescriptorPoolAllocator,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: Vec<vk::DescriptorSet>,

    pub loading_sprite_tx: Sender<LoadingSprite>,
    pub loading_sprite_rx: Receiver<LoadingSprite>,
    pub loading_sprites: Vec<LoadingSprite>
}

impl VkSpriteResourceManager {
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn descriptor_set(&self) -> &Vec<vk::DescriptorSet> {
        &self.descriptor_set
    }

    pub fn set_images(&mut self, images: Vec<ManuallyDrop<VkImage>>) -> VkResult<()> {
        let mut sprites = Vec::with_capacity(images.len());
        for image in images {
            let image_view = Self::create_texture_image_view(&self.device, &image.image);
            sprites.push(VkSprite {
                image,
                image_view
            });
        }

        self.sprites = sprites;
        self.refresh_descriptor_set()?;
        Ok(())
    }

    pub fn loading_sprite_tx(&self) -> &Sender<LoadingSprite> {
        &self.loading_sprite_tx
    }

    fn refresh_descriptor_set(&mut self) -> VkResult<()> {
        self.descriptor_pool_allocator.retire_pool(self.descriptor_pool);

        let descriptor_pool = self.descriptor_pool_allocator.allocate_pool(&self.device)?;
        let descriptor_set = Self::create_descriptor_set(
            &self.device,
            &descriptor_pool,
            self.descriptor_set_layout,
            &self.sprites
        )?;

        self.descriptor_pool = descriptor_pool;
        self.descriptor_set = descriptor_set;

        Ok(())
    }

    pub fn update(&mut self, device: &VkDevice) {
        self.descriptor_pool_allocator.update(&self.device);
        self.image_drop_sink.update(&self.device);

        for loading_sprite in self.loading_sprite_rx.recv_timeout(Duration::from_secs(0)) {
            self.loading_sprites.push(loading_sprite);
        }

        // let loading_sprite = self.loading_sprites.pop_front();
        // if let Some(loading_sprite) = loading_sprite {
        //     self.set_images(loading_sprite.images);
        //     self.refresh_descriptor_set();
        // }

        for i in (0..self.loading_sprites.len()).rev() {
            let result = self.loading_sprites[i].poll_load(device);
            println!("poll_load {:?}", result);
            match result {
                LoadingSpritePollResult::Pending => {
                    // do nothing
                },
                LoadingSpritePollResult::Complete => {
                    let loading_sprite = self.loading_sprites.swap_remove(i);
                    self.set_images(loading_sprite.images);
                    self.refresh_descriptor_set();
                    loading_sprite.load_op.complete();
                },
                LoadingSpritePollResult::Error(e) => {
                    let image = self.loading_sprites.swap_remove(i);
                    //image.load_op.error(e);
                }
            }
        }

        // if let Some(loading_sprite) = self.loading_sprites.front_mut() {
        //     loop {
        //         match loading_sprite.uploader.state() {
        //             Ok(state) => {
        //                 match state {
        //                     VkTransferUploadState::Writable => {
        //                         println!("VkTransferUploadState::Writable");
        //                         loading_sprite.uploader.submit_transfer(device.queues.transfer_queue);
        //                         break;
        //                     },
        //                     VkTransferUploadState::SentToTransferQueue => {
        //                         println!("VkTransferUploadState::SentToTransferQueue");
        //                         break;
        //                     },
        //                     VkTransferUploadState::PendingSubmitDstQueue => {
        //                         println!("VkTransferUploadState::PendingSubmitDstQueue");
        //                         loading_sprite.uploader.submit_dst(device.queues.graphics_queue);
        //                         break;
        //                     },
        //                     VkTransferUploadState::SentToDstQueue => {
        //                         println!("VkTransferUploadState::SentToDstQueue");
        //                         break;
        //                     },
        //                     VkTransferUploadState::Complete => {
        //                         println!("VkTransferUploadState::Complete");
        //                         let front = self.loading_sprites.pop_front().unwrap();
        //                         front.load_op.complete();
        //                     },
        //                 }
        //             },
        //             Err(err) => {
        //                 let front = self.loading_sprites.pop_front().unwrap();
        //                 front.load_op.error(err);
        //             },
        //         }
        //     }
        // }
    }

    pub fn new(
        device: &VkDevice,
        //swapchain: &VkSwapchain,
        swapchain_info: SwapchainInfo
    ) -> VkResult<Self> {

        // let decoded_texture = decode_texture(include_bytes!("../../../../assets/textures/texture2.jpg"), image::ImageFormat::Jpeg);
        // let mut decoded_textures = vec![];
        // for _ in 0..MAX_TEXTURES {
        //     decoded_textures.push(decoded_texture.clone());
        // }

        let decoded_textures = [
            //crate::image_utils::decode_texture(include_bytes!("../../../../assets/textures/texture.jpg"), image::ImageFormat::Jpeg),
            //decode_texture(include_bytes!("../../../../assets/textures/texture2.jpg"), image::ImageFormat::Jpeg),
            //decode_texture(include_bytes!("../../../../texture.jpg"), image::ImageFormat::Jpeg),
        ];

        //let tiny_texture = decode_texture(include_bytes!("../../../../assets/textures/texture2.jpg"), image::ImageFormat::Jpeg);
        //let tiny_texture = decode_texture(include_bytes!("../../../../assets/textures/texture-tiny-rust.png"), image::ImageFormat::Png);
        //let tiny_texture = decode_texture(include_bytes!("../../../../assets/textures/texture-tiny-rust.jpg"), image::ImageFormat::Jpeg);

        //let decoded_textures : Vec<DecodedTexture> = (0..MAX_TEXTURES).map(|_| tiny_texture.clone()).collect();


        //
        // Command Buffers
        //
        let command_pool =
            Self::create_command_pool(device.device(), &device.queue_family_indices)?;

        //
        // Resources
        //
        let images = crate::image_utils::load_images(
            &device.context,
            device.queue_family_indices.transfer_queue_family_index,
            device.queues.transfer_queue,
            device.queue_family_indices.graphics_queue_family_index,
            device.queues.graphics_queue,
            &decoded_textures
        )?;

        //let mut image_views = Vec::with_capacity(decoded_textures.len());
        let mut sprites = Vec::with_capacity(images.len());
        for image in images {
            //image_views.push(Self::create_texture_image_view(device.device(), &image.image));
            let image_view = Self::create_texture_image_view(device.device(), &image.image);
            sprites.push(VkSprite {
                image,
                image_view
            });
        }

        //
        // Descriptors
        //
        let descriptor_set_layout = Self::create_descriptor_set_layout(device.device())?;
        let mut descriptor_pool_allocator = DescriptorPoolAllocator::new(
            swapchain_info.image_count as u32,
            swapchain_info.image_count as u32 + 1,
            |device| Self::create_descriptor_pool(device)
        );
        let descriptor_pool = descriptor_pool_allocator.allocate_pool(device.device())?;
        let descriptor_set = Self::create_descriptor_set(
            device.device(),
            &descriptor_pool,
            descriptor_set_layout,
            &sprites
        )?;

        let (loading_sprite_tx, loading_sprite_rx) = mpsc::channel();

        let image_drop_sink = ImageDropSink::new(swapchain_info.image_count as u32 + 1);

        Ok(VkSpriteResourceManager {
            device: device.device().clone(),
            swapchain_info,
            command_pool,
            descriptor_set_layout,
            descriptor_pool_allocator,
            descriptor_pool,
            descriptor_set,
            sprites,
            image_drop_sink,
            loading_sprite_tx,
            loading_sprite_rx,
            loading_sprites: Default::default()
        })
    }

    fn create_command_pool(
        logical_device: &ash::Device,
        queue_family_indices: &VkQueueFamilyIndices,
    ) -> VkResult<vk::CommandPool> {
        log::info!(
            "Creating command pool with queue family index {}",
            queue_family_indices.graphics_queue_family_index
        );
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(
                vk::CommandPoolCreateFlags::TRANSIENT
                    | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            )
            .queue_family_index(queue_family_indices.graphics_queue_family_index);

        unsafe { logical_device.create_command_pool(&pool_create_info, None) }
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
        sprites: &[VkSprite],
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

        Ok(descriptor_sets)
    }
}

impl Drop for VkSpriteResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying VkSpriteResourceManager");

        unsafe {

            self.device.destroy_command_pool(self.command_pool, None);

            self.descriptor_pool_allocator.retire_pool(self.descriptor_pool);
            self.descriptor_pool_allocator.destroy(&self.device);

            // self.device
            //     .destroy_descriptor_pool(self.descriptor_pool, None);

            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            // for image_view in &self.image_views {
            //     self.device.destroy_image_view(*image_view, None);
            // }
            //
            // for image in &mut self.images {
            //     ManuallyDrop::drop(image);
            // }

            for sprite in self.sprites.drain(..) {
                self.image_drop_sink.retire_image(sprite.image);
                self.image_drop_sink.retire_image_view(sprite.image_view);
            }



            self.image_drop_sink.destroy(&self.device);

            for sprite in &mut self.sprites {
                self.device.destroy_image_view(sprite.image_view, None);
                ManuallyDrop::drop(&mut sprite.image)
            }
        }

        log::debug!("destroyed VkSpriteResourceManager");
    }
}

