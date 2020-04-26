use std::mem;
use ash::vk;
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::VkDevice;
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use std::process::exit;
use image::{GenericImageView, ImageFormat};
use ash::vk::ShaderStageFlags;

#[derive(Clone)]
pub struct DecodedTexture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

fn decode_texture(buf: &[u8], format: ImageFormat) -> DecodedTexture {
    let example_image = image::load_from_memory_with_format(buf, format).unwrap();
    let dimensions = example_image.dimensions();
    let example_image = example_image.to_rgba().into_raw();
    DecodedTexture {
        width: dimensions.0,
        height: dimensions.1,
        data: example_image
    }
}

const MAX_TEXTURES : u32 = 100;

pub struct VkSpriteResourceManager {
    pub device: ash::Device,
    pub swapchain_info: SwapchainInfo,


    pub command_pool: vk::CommandPool,
    //pub command_buffers: Vec<vk::CommandBuffer>,

    // The raw texture resources
    pub images: Vec<ManuallyDrop<VkImage>>,
    pub image_views: Vec<vk::ImageView>,

    // The descriptor set layout, pools, and sets
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<Vec<vk::DescriptorSet>>,
}

impl VkSpriteResourceManager {
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn descriptor_sets(&self, present_index: usize) -> &Vec<vk::DescriptorSet> {
        &self.descriptor_sets[present_index]
    }


    pub fn new(
        device: &VkDevice,
        //swapchain: &VkSwapchain,
        swapchain_info: SwapchainInfo
    ) -> VkResult<Self> {
        let decoded_texture = decode_texture(include_bytes!("../../../../assets/textures/texture2.jpg"), image::ImageFormat::Jpeg);
        let mut decoded_textures = vec![];
        for _ in 0..MAX_TEXTURES {
            decoded_textures.push(decoded_texture.clone());
        }

        let decoded_textures = [
            decode_texture(include_bytes!("../../../../assets/textures/texture.jpg"), image::ImageFormat::Jpeg),
            decode_texture(include_bytes!("../../../../assets/textures/texture2.jpg"), image::ImageFormat::Jpeg),
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
            Self::create_command_pool(&device.logical_device, &device.queue_family_indices)?;

        //
        // Resources
        //
        let mut images = vec![];
        let mut image_views = vec![];
        for decoded_texture in &decoded_textures {
            let image = load_image(
                &device.logical_device,
                device.queues.graphics_queue,
                command_pool,
                &device.memory_properties,
                &decoded_texture,
            )?;

            let image_view = Self::create_texture_image_view(&device.logical_device, &image.image);

            images.push(image);
            image_views.push(image_view);
        }

        //
        // Descriptors
        //
        let descriptor_set_layout = Self::create_descriptor_set_layout(&device.logical_device)?;

        let descriptor_pool = Self::create_descriptor_pool(
            &device.logical_device,
            swapchain_info.image_count as u32,
        )?;

        let descriptor_sets = Self::create_descriptor_sets(
            &device.logical_device,
            &descriptor_pool,
            descriptor_set_layout,
            swapchain_info.image_count,
            &image_views,
        )?;

        Ok(VkSpriteResourceManager {
            device: device.logical_device.clone(),
            swapchain_info,
            descriptor_set_layout,
            command_pool,
            descriptor_pool,
            descriptor_sets,
            images,
            image_views,
        })
    }

    // Called per changed resource. The commit version should be bumped once for each set of
    // changes. So the call pattern would be add(1), add(1), add(1), commit(1), add(2), add(2), etc.
    fn load_texture(hash: u32, data: DecodedTexture, commit_version: u32) {
        // Get the texture uploaded, possibly on another thread
    }

    fn unload_texture(hash:u32) {

    }

    // Call after all adds for a single commit complete
    fn commit_texture_changes() {
        // Build descriptor sets
    }

    fn frame_begin(frame_index: u32) {
        // all descriptors we currently hold are guaranteed to remain until frame_end is called for
        // the same frame index
    }

    fn frame_end(frame_index: u32) {
        // this will potentially retire some descriptors
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
        swapchain_image_count: u32,
    ) -> VkResult<vk::DescriptorPool> {
        let pool_sizes = [
            vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(MAX_TEXTURES * swapchain_image_count)
                .build(),
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(MAX_TEXTURES * swapchain_image_count);

        unsafe { logical_device.create_descriptor_pool(&descriptor_pool_info, None) }
    }

    fn create_descriptor_sets(
        logical_device: &ash::Device,
        descriptor_pool: &vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
        swapchain_image_count: usize,
        image_views: &[vk::ImageView],
    ) -> VkResult<Vec<Vec<vk::DescriptorSet>>> {
        let mut all_sets = Vec::with_capacity(swapchain_image_count);

        for _ in 0..swapchain_image_count {
            // DescriptorSetAllocateInfo expects an array with an element per set
            let descriptor_set_layouts = vec![descriptor_set_layout; MAX_TEXTURES as usize];

            let alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*descriptor_pool)
                .set_layouts(descriptor_set_layouts.as_slice());

            let descriptor_sets = unsafe { logical_device.allocate_descriptor_sets(&alloc_info) }?;


            for (image_index, image_view) in image_views.iter().enumerate() {

                let mut descriptor_writes = Vec::with_capacity(MAX_TEXTURES as usize);
                let image_view_descriptor_image_info = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(*image_view)
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

//            unsafe {
//                logical_device.update_descriptor_sets(&descriptor_writes, &[]);
//            }

            all_sets.push(descriptor_sets);
        }

        Ok(all_sets)
    }
}

impl Drop for VkSpriteResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying VkSpriteResourceManager");

        unsafe {

            self.device.destroy_command_pool(self.command_pool, None);

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);

            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            for image_view in &self.image_views {
                self.device.destroy_image_view(*image_view, None);
            }

            for image in &mut self.images {
                ManuallyDrop::drop(image);
            }
        }

        log::debug!("destroyed VkSpriteResourceManager");
    }
}

pub fn transition_image_layout(
    logical_device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    image: vk::Image,
    _format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> VkResult<()> {
    util::submit_single_use_command_buffer(logical_device, queue, command_pool, |command_buffer| {
        struct SyncInfo {
            src_access_mask: vk::AccessFlags,
            dst_access_mask: vk::AccessFlags,
            src_stage: vk::PipelineStageFlags,
            dst_stage: vk::PipelineStageFlags,
        }

        let sync_info = match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => SyncInfo {
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                src_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                dst_stage: vk::PipelineStageFlags::TRANSFER,
            },
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
                SyncInfo {
                    src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                    src_stage: vk::PipelineStageFlags::TRANSFER,
                    dst_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
                }
            }
            _ => {
                // Layout transition not yet supported
                unimplemented!();
            }
        };

        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let barrier_info = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(*subresource_range)
            .src_access_mask(sync_info.src_access_mask)
            .dst_access_mask(sync_info.dst_access_mask);

        unsafe {
            logical_device.cmd_pipeline_barrier(
                command_buffer,
                sync_info.src_stage,
                sync_info.dst_stage,
                vk::DependencyFlags::BY_REGION,
                &[],
                &[],
                &[*barrier_info],
            ); //TODO: Can remove build() by using *?
        }
    })
}

pub fn copy_buffer_to_image(
    logical_device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    buffer: vk::Buffer,
    image: vk::Image,
    extent: &vk::Extent3D,
) -> VkResult<()> {
    util::submit_single_use_command_buffer(logical_device, queue, command_pool, |command_buffer| {
        let image_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1);

        let image_copy = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(*image_subresource)
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(*extent);

        unsafe {
            logical_device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[*image_copy],
            );
        }
    })
}

pub fn load_image(
    logical_device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
    decoded_texture: &DecodedTexture,
) -> VkResult<ManuallyDrop<VkImage>> {
    let extent = vk::Extent3D {
        width: decoded_texture.width,
        height: decoded_texture.height,
        depth: 1,
    };

    let mut staging_buffer = VkBuffer::new(
        logical_device,
        device_memory_properties,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        decoded_texture.data.len() as u64,
    )?;

    staging_buffer.write_to_host_visible_buffer(&decoded_texture.data)?;

    let image = VkImage::new(
        logical_device,
        device_memory_properties,
        extent,
        vk::Format::R8G8B8A8_UNORM,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    transition_image_layout(
        logical_device,
        queue,
        command_pool,
        image.image,
        vk::Format::R8G8B8A8_UNORM,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    )?;

    copy_buffer_to_image(
        logical_device,
        queue,
        command_pool,
        staging_buffer.buffer,
        image.image,
        &image.extent,
    )?;

    transition_image_layout(
        logical_device,
        queue,
        command_pool,
        image.image,
        vk::Format::R8G8B8A8_UNORM,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )?;

    Ok(ManuallyDrop::new(image))
}