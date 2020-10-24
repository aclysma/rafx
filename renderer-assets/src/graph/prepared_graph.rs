use super::{
    PhysicalImageId, RenderGraphOutputImageId, RenderGraphImageSpecification, RenderGraphOutputPass,
};
use fnv::{FnvHashMap, FnvHashSet};
use renderer_shell_vulkan::{VkDeviceContext, VkImage};
use ash::vk;
use crate::resources::ResourceLookupSet;
use crate::{ResourceArc, ImageViewResource, DynCommandWriterAllocator, ResourceManager};
use ash::prelude::VkResult;
use std::mem::ManuallyDrop;
use crate::vk_description as dsc;
use crate::vk_description::{ImageAspectFlags, SwapchainSurfaceInfo};
use crate::resources::RenderPassResource;
use crate::resources::FramebufferResource;
use crate::graph::graph_node::RenderGraphNodeId;
use ash::version::DeviceV1_0;
use crate::graph::{RenderGraph, RenderGraphImageUsageId};
use renderer_nodes::{RenderPhase, RenderPhaseIndex};

//TODO: A caching system that keeps resources alive across a few frames so that we can reuse
// images, framebuffers, and renderpasses

#[derive(Debug)]
pub struct RenderGraphPlanOutputImage {
    pub output_id: RenderGraphOutputImageId,
    pub dst_image: ResourceArc<ImageViewResource>,
}

/// The final output of a render graph, which will be consumed by PreparedRenderGraph. This just
/// includes the computed metadata and does not allocate resources.
#[derive(Debug)]
pub struct RenderGraphPlan {
    pub passes: Vec<RenderGraphOutputPass>,
    pub output_images: FnvHashMap<PhysicalImageId, RenderGraphPlanOutputImage>,
    pub intermediate_images: FnvHashMap<PhysicalImageId, RenderGraphImageSpecification>,
    pub node_to_renderpass_index: FnvHashMap<RenderGraphNodeId, usize>,
    pub image_usage_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId>,
}

/// Encapsulates a render graph plan and all resources required to execute it
pub struct PreparedRenderGraph {
    device_context: VkDeviceContext,
    image_resources: FnvHashMap<PhysicalImageId, ResourceArc<ImageViewResource>>,
    render_pass_resources: Vec<ResourceArc<RenderPassResource>>,
    framebuffer_resources: Vec<ResourceArc<FramebufferResource>>,
    graph_plan: RenderGraphPlan,
    swapchain_surface_info: SwapchainSurfaceInfo,
}

impl PreparedRenderGraph {
    pub fn new(
        device_context: &VkDeviceContext,
        resources: &mut ResourceLookupSet,
        graph: RenderGraph,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> VkResult<Self> {
        let graph_plan = graph.into_plan(swapchain_surface_info);

        let image_resources = Self::allocate_images(
            device_context,
            &graph_plan,
            resources,
            swapchain_surface_info,
        )?;
        let render_pass_resources =
            Self::allocate_render_passes(&graph_plan, resources, swapchain_surface_info)?;

        let framebuffer_resources = Self::allocate_framebuffers(
            &graph_plan,
            resources,
            swapchain_surface_info,
            &image_resources,
            &render_pass_resources,
        )?;

        Ok(PreparedRenderGraph {
            device_context: device_context.clone(),
            image_resources,
            render_pass_resources,
            framebuffer_resources,
            graph_plan,
            swapchain_surface_info: swapchain_surface_info.clone(),
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
            let image_resource = resources.insert_image(ManuallyDrop::new(image));

            //println!("SPEC {:#?}", specification);
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
            let image_view =
                resources.get_or_create_image_view(&image_resource, &image_view_meta)?;

            image_resources.insert(*id, image_view);
        }
        Ok(image_resources)
    }

    fn allocate_render_passes(
        graph: &RenderGraphPlan,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<Vec<ResourceArc<RenderPassResource>>> {
        let mut render_pass_resources = Vec::with_capacity(graph.passes.len());
        for pass in &graph.passes {
            // println!("Allocate {:#?}", pass);
            // for dependency in &renderpass.description.dependencies {
            //     let builder = dependency.as_builder();
            //     let built = builder.build();
            //     println!("{:?}", built);
            // }
            let render_pass_resource =
                resources.get_or_create_renderpass(&pass.description, swapchain_surface_info)?;
            render_pass_resources.push(render_pass_resource);
        }
        Ok(render_pass_resources)
    }

    fn allocate_framebuffers(
        graph: &RenderGraphPlan,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
        image_resources: &FnvHashMap<PhysicalImageId, ResourceArc<ImageViewResource>>,
        render_pass_resources: &Vec<ResourceArc<RenderPassResource>>,
    ) -> VkResult<Vec<ResourceArc<FramebufferResource>>> {
        let mut framebuffers = Vec::with_capacity(graph.passes.len());
        for (pass_index, pass) in graph.passes.iter().enumerate() {
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
                render_pass_resources[pass_index].clone(),
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
        node_visitor: &dyn RenderGraphNodeVisitor,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        //
        // Start a command writer. For now just do a single primary writer, later we can multithread this.
        //
        let mut command_writer = command_writer_allocator.allocate_writer(
            self.device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            vk::CommandPoolCreateFlags::TRANSIENT,
            0,
        )?;

        let command_buffer = command_writer.begin_command_buffer(
            vk::CommandBufferLevel::PRIMARY,
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            None,
        )?;

        let device = self.device_context.device();

        //
        // Iterate through all passes
        //
        for (pass_index, pass) in self.graph_plan.passes.iter().enumerate() {
            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass_resources[pass_index].get_raw().renderpass)
                .framebuffer(self.framebuffer_resources[pass_index].get_raw().framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_surface_info.extents,
                })
                .clear_values(&pass.clear_values);

            assert_eq!(pass.subpass_nodes.len(), 1);
            let node_id = pass.subpass_nodes[0];

            unsafe {
                device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );

                // callback here!
                node_visitor.visit_renderpass(node_id, command_buffer)?;

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

        command_writer.end_command_buffer()?;

        Ok(vec![command_buffer])
    }
}

pub trait RenderGraphNodeVisitor {
    fn visit_renderpass(
        &self,
        node_id: RenderGraphNodeId,
        command_buffer: vk::CommandBuffer,
    ) -> VkResult<()>;
}

type RenderGraphNodeVisitorCallback<T> = dyn Fn(vk::CommandBuffer, &T) -> VkResult<()> + Send;

/// Created by RenderGraphNodeCallbacks::create_visitor(). Implements RenderGraphNodeVisitor and
/// forwards the call, adding the context as a parameter.
struct RenderGraphNodeVisitorImpl<'b, U> {
    context: &'b U,
    node_callbacks: &'b FnvHashMap<RenderGraphNodeId, Box<RenderGraphNodeVisitorCallback<U>>>,
}

impl<'b, U> RenderGraphNodeVisitor for RenderGraphNodeVisitorImpl<'b, U> {
    fn visit_renderpass(
        &self,
        node_id: RenderGraphNodeId,
        command_buffer: vk::CommandBuffer,
    ) -> VkResult<()> {
        (self.node_callbacks[&node_id])(command_buffer, self.context)
    }
}

/// All the callbacks associated with rendergraph nodes. We keep them separate from the nodes so
/// that we can avoid propagating generic parameters throughout the rest of the rendergraph code
pub struct RenderGraphNodeCallbacks<T> {
    node_callbacks: FnvHashMap<RenderGraphNodeId, Box<RenderGraphNodeVisitorCallback<T>>>,
    render_phase_dependencies: FnvHashMap<RenderGraphNodeId, FnvHashSet<RenderPhaseIndex>>,
}

impl<T> RenderGraphNodeCallbacks<T> {
    /// Adds a callback that receives the renderpass associated with the node
    pub fn set_renderpass_callback<F>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: F,
    ) where
        F: Fn(vk::CommandBuffer, &T) -> VkResult<()> + 'static + Send,
    {
        self.node_callbacks.insert(node_id, Box::new(f));
    }

    pub fn add_renderphase_dependency<U: RenderPhase>(
        &mut self,
        node_id: RenderGraphNodeId,
    ) {
        self.render_phase_dependencies
            .entry(node_id)
            .or_default()
            .insert(U::render_phase_index());
    }

    /// Pass to PreparedRenderGraph::execute_graph, this will cause the graph to be executed,
    /// triggering any registered callbacks
    pub fn create_visitor<'a>(
        &'a self,
        context: &'a T,
    ) -> Box<dyn RenderGraphNodeVisitor + 'a> {
        Box::new(RenderGraphNodeVisitorImpl::<'a, T> {
            context,
            node_callbacks: &self.node_callbacks,
        })
    }
}

impl<T> Default for RenderGraphNodeCallbacks<T> {
    fn default() -> Self {
        RenderGraphNodeCallbacks {
            node_callbacks: Default::default(),
            render_phase_dependencies: Default::default(),
        }
    }
}

/// A wrapper around a prepared render graph and callbacks that will be hit when executing the graph
pub struct RenderGraphExecutor<T> {
    prepared_graph: PreparedRenderGraph,
    callbacks: RenderGraphNodeCallbacks<T>,
}

impl<T> RenderGraphExecutor<T> {
    /// Create the executor. This allows the prepared graph, resources required to execute it, and
    /// callbacks that will be triggered while executing it to be passed around and executed later.
    pub fn new(
        device_context: &VkDeviceContext,
        graph: RenderGraph,
        resource_manager: &mut ResourceManager,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        callbacks: RenderGraphNodeCallbacks<T>,
    ) -> VkResult<Self> {
        //
        // Allocate the resources for the graph
        //
        let prepared_graph = PreparedRenderGraph::new(
            device_context,
            resource_manager.resources_mut(),
            graph,
            swapchain_surface_info,
        )?;

        //
        // Ensure expensive resources are persisted across frames so they can be reused
        //
        for renderpass in &prepared_graph.render_pass_resources {
            resource_manager
                .resource_caches_mut()
                .cache_render_pass(renderpass.clone());
        }

        for framebuffer in &prepared_graph.framebuffer_resources {
            resource_manager
                .resource_caches_mut()
                .cache_framebuffer(framebuffer.clone());
        }

        //
        // Pre-warm caches for pipelines that we may need
        //

        for (node_id, render_phase_indices) in &callbacks.render_phase_dependencies {
            // Passes may get culled if the images are not used. This means the renderpass would
            // not be created so pipelines are also not needed
            if let Some(&renderpass_index) = prepared_graph
                .graph_plan
                .node_to_renderpass_index
                .get(node_id)
            {
                for &render_phase_index in render_phase_indices {
                    resource_manager
                        .graphics_pipeline_cache_mut()
                        .register_renderpass_to_phase_index_per_frame(
                            &prepared_graph.render_pass_resources[renderpass_index],
                            render_phase_index,
                        )
                }
            }
        }
        resource_manager.cache_all_graphics_pipelines()?;

        //
        // Return the executor which can be triggered later
        //
        Ok(RenderGraphExecutor {
            prepared_graph,
            callbacks,
        })
    }

    pub fn renderpass_resource(
        &self,
        node_id: RenderGraphNodeId,
    ) -> Option<ResourceArc<RenderPassResource>> {
        let renderpass_index = *self
            .prepared_graph
            .graph_plan
            .node_to_renderpass_index
            .get(&node_id)?;
        Some(self.prepared_graph.render_pass_resources[renderpass_index].clone())
    }

    pub fn image_resource(
        &self,
        image_usage: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        let image = self
            .prepared_graph
            .graph_plan
            .image_usage_to_physical
            .get(&image_usage)?;
        Some(self.prepared_graph.image_resources[image].clone())
    }

    /// Executes the graph, passing through the given context parameter
    pub fn execute_graph(
        mut self,
        command_writer_allocator: &DynCommandWriterAllocator,
        context: &T,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let visitor = self.callbacks.create_visitor(context);
        self.prepared_graph
            .execute_graph(command_writer_allocator, &*visitor)
    }
}
