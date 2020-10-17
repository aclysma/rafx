use super::{
    PhysicalImageId, RenderGraphOutputImageId, RenderGraphImageSpecification, RenderGraphOutputPass,
};
use fnv::FnvHashMap;
use renderer_shell_vulkan::{VkDeviceContext, VkImage, VkImageRaw};
use ash::vk;
use crate::resources::ResourceLookupSet;
use crate::{ResourceArc, ImageKey, ImageViewResource};
use ash::prelude::VkResult;
use std::mem::ManuallyDrop;
use crate::vk_description as dsc;
use crate::vk_description::ImageAspectFlags;

#[derive(Debug)]
pub struct PreparedRenderGraph {
    pub renderpasses: Vec<RenderGraphOutputPass>,
    pub output_images: FnvHashMap<PhysicalImageId, RenderGraphOutputImageId>,
    pub intermediate_images: FnvHashMap<PhysicalImageId, RenderGraphImageSpecification>,
}

struct FramebufferImage {
    image_key: ImageKey,
    image: ResourceArc<VkImageRaw>,
    image_view: ResourceArc<ImageViewResource>,
}

pub struct FramebufferAllocator {
    device_context: VkDeviceContext,
    //images: FnvHashMap<vk::Image, FramebufferImage>,
    //available_image_pool: FnvHashMap<RenderGraphImageSpecification, Vec<VkImage>>,
    //allocated_images: FnvHashMap<PhysicalImageId, VkImage>,
}

impl FramebufferAllocator {
    pub fn new(device_context: VkDeviceContext) -> Self {
        FramebufferAllocator {
            device_context,
            //images: Default::default(),
            //available_image_pool: Default::default(),
            //allocated_images: Default::default()
        }
    }

    fn allocate_images(
        &mut self,
        graph: &PreparedRenderGraph,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<FnvHashMap<PhysicalImageId, FramebufferImage>> {
        let mut image_resources: FnvHashMap<PhysicalImageId, FramebufferImage> = Default::default();
        for (id, specification) in &graph.intermediate_images {
            let image = VkImage::new(
                &self.device_context,
                vk_mem::MemoryUsage::GpuOnly,
                specification.usage_flags,
                vk::Extent3D {
                    width: swapchain_surface_info.extents.width,
                    height: swapchain_surface_info.extents.height,
                    depth: 1,
                },
                specification.format,
                vk::ImageTiling::OPTIMAL,
                specification.samples,
                1,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            let (image_key, image) = resources.insert_image(ManuallyDrop::new(image));

            println!("SPEC {:#?}", specification);
            let subresource_range = dsc::ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::from_bits(specification.aspect_flags.as_raw())
                    .unwrap(),
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            };

            let image_view_meta = dsc::ImageViewMeta {
                format: specification.format.into(),
                components: Default::default(),
                subresource_range,
                view_type: dsc::ImageViewType::Type2D,
            };
            let image_view = resources.get_or_create_image_view(image_key, &image_view_meta)?;

            image_resources.insert(
                *id,
                FramebufferImage {
                    image_key,
                    image,
                    image_view,
                },
            );
        }
        Ok(image_resources)
    }

    fn allocate_passes(
        &mut self,
        graph: &PreparedRenderGraph,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<Vec<ResourceArc<vk::RenderPass>>> {
        let mut pass_resources = Vec::with_capacity(graph.renderpasses.len());
        for renderpass in &graph.renderpasses {
            println!("Allocate {:#?}", renderpass);
            // for dependency in &renderpass.description.dependencies {
            //     let builder = dependency.as_builder();
            //     let built = builder.build();
            //     println!("{:?}", built);
            // }
            let pass_resource = resources
                .get_or_create_renderpass(&renderpass.description, swapchain_surface_info)?;
            pass_resources.push(pass_resource);
        }
        Ok(pass_resources)
    }

    pub fn allocate_resources(
        &mut self,
        graph: &PreparedRenderGraph,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<()> {
        let image_resources = self.allocate_images(graph, resources, swapchain_surface_info)?;
        let pass_resources = self.allocate_passes(graph, resources, swapchain_surface_info)?;

        let mut framebuffers = Vec::with_capacity(graph.renderpasses.len());
        for (pass_index, pass) in graph.renderpasses.iter().enumerate() {
            let attachments: Vec<_> = pass
                .attachment_images
                .iter()
                .map(|x| image_resources[x].image_view.get_raw().image_view)
                .collect();

            let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(pass_resources[pass_index].get_raw())
                .attachments(attachments.as_slice())
                .width(swapchain_surface_info.extents.width)
                .height(swapchain_surface_info.extents.height)
                .layers(1);

            use ash::version::DeviceV1_0;
            let device = self.device_context.device();
            let framebuffer =
                unsafe { device.create_framebuffer(&frame_buffer_create_info, None)? };
            framebuffers.push(framebuffer);
        }

        for (pass_index, pass) in graph.renderpasses.iter().enumerate() {
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(pass_resources[pass_index].get_raw())
                .framebuffer(framebuffers[pass_index])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain_surface_info.extents,
                })
                .clear_values(&pass.clear_values);
        }

        for framebuffer in framebuffers {
            let device = self.device_context.device();
            use ash::version::DeviceV1_0;
            unsafe {
                device.destroy_framebuffer(framebuffer, None);
            }
        }

        Ok(())
    }
}
