use super::{
    PhysicalImageId, RenderGraphOutputImageId, RenderGraphImageSpecification, RenderGraphOutputPass,
};
use fnv::FnvHashMap;
use renderer_shell_vulkan::{VkDeviceContext, VkImage, VkImageRaw};
use ash::vk;
use crate::resources::ResourceLookupSet;
use crate::{ResourceArc, ImageKey, ImageViewResource, DynCommandWriter, DynCommandWriterAllocator};
use ash::prelude::VkResult;
use std::mem::ManuallyDrop;
use crate::vk_description as dsc;
use crate::vk_description::{ImageAspectFlags, SwapchainSurfaceInfo};
use crate::resources::RenderPassResource;
use crate::resources::FramebufferResource;
use crate::graph::graph_node::RenderGraphNodeId;
use ash::version::DeviceV1_0;
use crate::graph::RenderGraph;

#[derive(Debug)]
pub struct RenderGraphPlanOutputImage {
    pub output_id: RenderGraphOutputImageId,
    pub dst_image: ResourceArc<ImageViewResource>,
}

/// The final output of a render graph, which will be consumed by PreparedRenderGraph. This just
/// includes the computed metadata and does not allocate resources.
#[derive(Debug)]
pub struct RenderGraphPlan {
    pub renderpasses: Vec<RenderGraphOutputPass>,
    pub output_images: FnvHashMap<PhysicalImageId, RenderGraphPlanOutputImage>,
    pub intermediate_images: FnvHashMap<PhysicalImageId, RenderGraphImageSpecification>,
}

/// Encapsulates a render graph plan and all resources required to execute it
pub struct PreparedRenderGraph {
    device_context: VkDeviceContext,
    image_resources: FnvHashMap<PhysicalImageId, ResourceArc<ImageViewResource>>,
    pass_resources: Vec<ResourceArc<RenderPassResource>>,
    framebuffer_resources: Vec<ResourceArc<FramebufferResource>>,
    graph_plan: RenderGraphPlan,
    swapchain_surface_info: SwapchainSurfaceInfo
}

impl PreparedRenderGraph {
    pub fn new(
        device_context: &VkDeviceContext,
        resources: &mut ResourceLookupSet,
        graph: RenderGraph,
        swapchain_surface_info: &SwapchainSurfaceInfo
    ) -> VkResult<Self> {
        let graph_plan = graph.into_plan(swapchain_surface_info);

        let image_resources = Self::allocate_images(device_context, &graph_plan, resources, swapchain_surface_info)?;
        let pass_resources = Self::allocate_passes(&graph_plan, resources, swapchain_surface_info)?;

        let framebuffer_resources = Self::allocate_framebuffers(
            &graph_plan,
            resources,
            swapchain_surface_info,
            &image_resources,
            &pass_resources,
        )?;

        Ok(PreparedRenderGraph {
            device_context: device_context.clone(),
            image_resources,
            pass_resources,
            framebuffer_resources,
            graph_plan,
            swapchain_surface_info: swapchain_surface_info.clone()
        })
    }

    fn allocate_images(
        device_context: &VkDeviceContext,
        graph: &RenderGraphPlan,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<FnvHashMap<PhysicalImageId, ResourceArc<ImageViewResource>>> {
        let mut image_resources: FnvHashMap<PhysicalImageId, ResourceArc<ImageViewResource>> =
            Default::default();

        for (id, image) in &graph.output_images {
            image_resources.insert(*id, image.dst_image.clone());
        }

        for (id, specification) in &graph.intermediate_images {
            let image = VkImage::new(
                device_context,
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

            image_resources.insert(*id, image_view);
        }
        Ok(image_resources)
    }

    fn allocate_passes(
        graph: &RenderGraphPlan,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<Vec<ResourceArc<RenderPassResource>>> {
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

    fn allocate_framebuffers(
        graph: &RenderGraphPlan,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
        image_resources: &FnvHashMap<PhysicalImageId, ResourceArc<ImageViewResource>>,
        pass_resources: &Vec<ResourceArc<RenderPassResource>>,
    ) -> VkResult<Vec<ResourceArc<FramebufferResource>>> {
        let mut framebuffers = Vec::with_capacity(graph.renderpasses.len());
        for (pass_index, pass) in graph.renderpasses.iter().enumerate() {
            let attachments: Vec<_> = pass
                .attachment_images
                .iter()
                .map(|x| image_resources[x].clone())
                .collect();

            let framebuffer_meta = dsc::FramebufferMeta {
                width: swapchain_surface_info.extents.width,
                height: swapchain_surface_info.extents.height,
                layers: 1,
            };

            let framebuffer = resources.get_or_create_framebuffer(
                pass_resources[pass_index].clone(),
                &attachments,
                &framebuffer_meta,
            )?;

            framebuffers.push(framebuffer);
        }
        Ok(framebuffers)
    }

    pub fn execute_graph(
        &mut self,
        command_writer_allocator: &DynCommandWriterAllocator,
        node_visitor: &RenderGraphNodeVisitor
    ) -> VkResult<Vec<vk::CommandBuffer>> {

        //
        // Start a command writer. For now just do a single primary writer, later we can multithread this.
        //
        let mut command_writer = command_writer_allocator.allocate_writer(
            self.device_context.queue_family_indices().graphics_queue_family_index,
            vk::CommandPoolCreateFlags::TRANSIENT,
            0
        )?;

        let command_buffer = command_writer.begin_command_buffer(
            vk::CommandBufferLevel::PRIMARY,
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            None
        )?;

        let device = self.device_context.device();

        //
        // Iterate through all passes
        //
        for (pass_index, pass) in self.graph_plan.renderpasses.iter().enumerate() {
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.pass_resources[pass_index].get_raw().renderpass)
                .framebuffer(self.framebuffer_resources[pass_index].get_raw().framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_surface_info.extents,
                })
                .clear_values(&pass.clear_values);

            assert_eq!(pass.subpass_nodes.len(), 1);
            let node_id = pass.subpass_nodes[0];

            unsafe {
                device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);

                // callback here!
                node_visitor.visit_renderpass(node_id, command_buffer);

                device.cmd_end_render_pass(command_buffer);
            }
        }

        // for framebuffer in framebuffers {
        //     let device = self.device_context.device();
        //     use ash::version::DeviceV1_0;
        //     unsafe {
        //         device.destroy_framebuffer(framebuffer, None);
        //     }
        // }

        command_writer.end_command_buffer();

        Ok(vec![command_buffer])
    }
}

pub trait RenderGraphNodeVisitor {
    fn visit_renderpass(&self, node_id: RenderGraphNodeId, command_buffer: vk::CommandBuffer) -> VkResult<()>;
}

type RenderGraphNodeVisitorCallback<T> = dyn Fn(vk::CommandBuffer, &T) -> VkResult<()> + Send;

/// Created by RenderGraphNodeCallbacks::create_visitor(). Implements RenderGraphNodeVisitor and
/// forwards the call, adding the context as a parameter.
struct RenderGraphNodeVisitorImpl<'b, U> {
    context: &'b U,
    node_callbacks: &'b FnvHashMap<RenderGraphNodeId, Box<RenderGraphNodeVisitorCallback<U>>>
}

impl<'b, U> RenderGraphNodeVisitor for RenderGraphNodeVisitorImpl<'b, U> {
    fn visit_renderpass(&self, node_id: RenderGraphNodeId, command_buffer: vk::CommandBuffer) -> VkResult<()> {
        (self.node_callbacks[&node_id])(command_buffer, self.context)
    }
}

/// All the callbacks associated with rendergraph nodes. We keep them separate from the nodes so
/// that we can avoid propagating generic parameters throughout the rest of the rendergraph code
pub struct RenderGraphNodeCallbacks<T> {
    node_callbacks: FnvHashMap<RenderGraphNodeId, Box<RenderGraphNodeVisitorCallback<T>>>
}

impl<T> RenderGraphNodeCallbacks<T> {
    /// Adds a callback that receives the renderpass associated with the node
    pub fn add_renderpass_callback<F>(&mut self, node_id: RenderGraphNodeId, f: F)
        where F : Fn(vk::CommandBuffer, &T) -> VkResult<()> + 'static + Send
    {
        self.node_callbacks.insert(node_id, Box::new(f));
    }

    /// Pass to PreparedRenderGraph::execute_graph, this will cause the graph to be executed,
    /// triggering any registered callbacks
    pub fn create_visitor<'a>(&'a self, context: &'a T) -> Box<dyn RenderGraphNodeVisitor + 'a> {
        Box::new(RenderGraphNodeVisitorImpl::<'a, T> {
            context,
            node_callbacks: &self.node_callbacks
        })
    }
}

impl<T> Default for RenderGraphNodeCallbacks<T> {
    fn default() -> Self {
        RenderGraphNodeCallbacks {
            node_callbacks: Default::default()
        }
    }
}

/// A wrapper around a prepared render graph and callbacks that will be hit when executing the graph
pub struct RenderGraphExecutor<T> {
    prepared_graph: PreparedRenderGraph,
    callbacks: RenderGraphNodeCallbacks<T>
}

impl<T> RenderGraphExecutor<T> {
    /// Create the executor. This allows the prepared graph, resources required to execute it, and
    /// callbacks that will be triggered while executing it to be passed around and executed later.
    pub fn new(
        device_context: &VkDeviceContext,
        graph: RenderGraph,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        callbacks: RenderGraphNodeCallbacks<T>
    ) -> VkResult<Self> {
        Ok(RenderGraphExecutor {
            prepared_graph: PreparedRenderGraph::new(device_context, resources, graph, swapchain_surface_info)?,
            callbacks
        })
    }

    /// Executes the graph, passing through the given context parameter
    pub fn execute_graph(mut self, command_writer_allocator: &DynCommandWriterAllocator, context: &T) -> VkResult<Vec<vk::CommandBuffer>> {
        let visitor = self.callbacks.create_visitor(context);
        self.prepared_graph.execute_graph(command_writer_allocator, &*visitor)
    }
}




