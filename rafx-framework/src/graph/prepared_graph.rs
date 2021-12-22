use super::PhysicalImageId;
use crate::graph::graph_buffer::PhysicalBufferId;
use crate::graph::graph_image::PhysicalImageViewId;
use crate::graph::graph_node::{RenderGraphNodeId, RenderGraphNodeName};
use crate::graph::graph_pass::{PrepassBufferBarrier, PrepassImageBarrier, RenderGraphOutputPass};
use crate::graph::graph_plan::RenderGraphPlan;
use crate::graph::{
    RenderGraphBufferUsageId, RenderGraphBuilder, RenderGraphImageUsageId,
    RenderGraphNodeVisitNodeCallback,
};
use crate::render_features::{
    PreparedRenderData, RenderJobBeginExecuteGraphContext, RenderJobCommandBufferContext,
    RenderJobWriteContext, RenderPhase, RenderView,
};
use crate::resources::DynCommandBuffer;
use crate::{BufferResource, GraphicsPipelineRenderTargetMeta, ImageResource, RenderResources};
use crate::{ImageViewResource, ResourceArc, ResourceContext};
use fnv::FnvHashMap;
use rafx_api::{
    RafxBarrierQueueTransition, RafxBufferBarrier, RafxColorRenderTargetBinding, RafxCommandBuffer,
    RafxCommandBufferDef, RafxCommandPoolDef, RafxDepthStencilRenderTargetBinding,
    RafxDeviceContext, RafxExtents2D, RafxFormat, RafxQueue, RafxResourceState, RafxResult,
    RafxSwapchainColorSpace, RafxTextureBarrier,
};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SwapchainSurfaceInfo {
    pub extents: RafxExtents2D,
    pub format: RafxFormat,
    pub color_space: RafxSwapchainColorSpace,
}

#[derive(Copy, Clone)]
pub struct RenderGraphContext<'graph, 'write> {
    prepared_render_graph: &'graph PreparedRenderGraph,
    prepared_render_data: &'graph PreparedRenderData<'write>,
    render_resources: &'graph RenderResources,
}

impl<'graph, 'write> RenderGraphContext<'graph, 'write> {
    pub fn buffer(
        &self,
        buffer: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        self.prepared_render_graph.buffer(buffer)
    }

    pub fn image_view(
        &self,
        image: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        self.prepared_render_graph.image_view(image)
    }

    pub fn device_context(&self) -> &RafxDeviceContext {
        &self.prepared_render_graph.device_context
    }

    pub fn resource_context(&self) -> &ResourceContext {
        &self.prepared_render_graph.resource_context
    }

    pub fn prepared_render_data(&self) -> &PreparedRenderData<'write> {
        &self.prepared_render_data
    }

    pub fn render_resources(&self) -> &RenderResources {
        &self.render_resources
    }
}

pub struct OnBeginExecuteGraphArgs<'graph, 'write> {
    pub command_buffer: DynCommandBuffer,
    pub graph_context: RenderGraphContext<'graph, 'write>,
}

pub struct VisitComputeNodeArgs<'graph, 'write> {
    pub command_buffer: DynCommandBuffer,
    pub graph_context: RenderGraphContext<'graph, 'write>,
}

pub struct VisitRenderpassNodeArgs<'graph, 'write> {
    pub command_buffer: DynCommandBuffer,
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
    pub graph_context: RenderGraphContext<'graph, 'write>,
}

// Convenience function for creating a write context and triggering writing a phase for a view.
// (Alternatively you can make your own write context, which allows calling write_view_phase
// multiple times with the same context)
impl<'graph, 'write> VisitRenderpassNodeArgs<'graph, 'write> {
    pub fn write_view_phase<PhaseT: RenderPhase>(
        &self,
        render_view: &RenderView,
    ) -> RafxResult<()> {
        let mut write_context =
            RenderJobCommandBufferContext::from_graph_visit_render_pass_args(self);
        self.graph_context
            .prepared_render_data()
            .write_view_phase::<PhaseT>(render_view, &mut write_context)
    }
}

/// Encapsulates a render graph plan and all resources required to execute it
pub struct PreparedRenderGraph {
    device_context: RafxDeviceContext,
    resource_context: ResourceContext,
    buffer_resources: FnvHashMap<PhysicalBufferId, ResourceArc<BufferResource>>,
    image_resources: FnvHashMap<PhysicalImageId, ResourceArc<ImageResource>>,
    image_view_resources: FnvHashMap<PhysicalImageViewId, ResourceArc<ImageViewResource>>,
    graph_plan: RenderGraphPlan,
}

impl PreparedRenderGraph {
    pub fn node_debug_name(
        &self,
        node_id: RenderGraphNodeId,
    ) -> Option<RenderGraphNodeName> {
        let pass_index = *self.graph_plan.node_to_pass_index.get(&node_id)?;
        self.graph_plan.passes[pass_index].debug_name()
    }

    pub fn new(
        device_context: &RafxDeviceContext,
        resource_context: &ResourceContext,
        graph: RenderGraphBuilder,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> RafxResult<Self> {
        let graph_plan = graph.build_plan();
        let mut cache_guard = resource_context.render_graph_cache().inner.lock().unwrap();
        let cache = &mut *cache_guard;

        profiling::scope!("allocate resources");
        let buffer_resources =
            cache.allocate_buffers(device_context, &graph_plan, resource_context.resources())?;

        let image_resources = cache.allocate_images(
            device_context,
            &graph_plan,
            resource_context.resources(),
            swapchain_surface_info,
        )?;

        let image_view_resources = cache.allocate_image_views(
            &graph_plan,
            resource_context.resources(),
            &image_resources,
        )?;

        Ok(PreparedRenderGraph {
            device_context: device_context.clone(),
            resource_context: resource_context.clone(),
            buffer_resources,
            image_resources,
            image_view_resources,
            graph_plan,
        })
    }

    pub fn buffer(
        &self,
        buffer: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        let physical_buffer = self.graph_plan.buffer_usage_to_physical.get(&buffer)?;
        self.buffer_resources.get(physical_buffer).cloned()
    }

    // pub fn image(
    //     &self,
    //     image_usage: RenderGraphImageUsageId,
    // ) -> Option<ResourceArc<ImageResource>> {
    //     let image = self.graph_plan.image_usage_to_physical.get(&image_usage)?;
    //     self.image_resources.get(image).cloned()
    // }

    pub fn image_view(
        &self,
        image: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        let physical_image = self.graph_plan.image_usage_to_view.get(&image)?;
        self.image_view_resources.get(physical_image).cloned()
    }

    fn insert_barriers(
        &self,
        command_buffer: &RafxCommandBuffer,
        pass_buffer_barriers: &[PrepassBufferBarrier],
        pass_image_barriers: &[PrepassImageBarrier],
    ) -> RafxResult<()> {
        assert!(!pass_buffer_barriers.is_empty() || !pass_image_barriers.is_empty());

        let mut buffer_barriers = Vec::with_capacity(pass_buffer_barriers.len());
        let buffers: Vec<_> = pass_buffer_barriers
            .iter()
            .map(|x| self.buffer_resources[&x.buffer].get_raw().buffer.clone())
            .collect();
        for (buffer_barrier, buffer) in pass_buffer_barriers.iter().zip(&buffers) {
            log::trace!(
                "add buffer barrier for buffer {:?} state {:?} -> {:?}",
                buffer_barrier.buffer,
                buffer_barrier.old_state,
                buffer_barrier.new_state
            );

            buffer_barriers.push(RafxBufferBarrier {
                buffer: buffer.as_ref(),
                src_state: buffer_barrier.old_state,
                dst_state: buffer_barrier.new_state,
                queue_transition: RafxBarrierQueueTransition::None,
            });
        }

        let mut image_barriers = Vec::with_capacity(pass_image_barriers.len());
        let images: Vec<_> = pass_image_barriers
            .iter()
            .map(|x| self.image_resources[&x.image].get_raw().image.clone())
            .collect();
        for (image_barrier, image) in pass_image_barriers.iter().zip(&images) {
            log::trace!(
                "add image barrier for image {:?} state {:?} -> {:?}",
                image_barrier.image,
                image_barrier.old_state,
                image_barrier.new_state
            );

            image_barriers.push(RafxTextureBarrier {
                texture: image,
                src_state: image_barrier.old_state,
                dst_state: image_barrier.new_state,
                array_slice: None,
                mip_slice: None,
                queue_transition: RafxBarrierQueueTransition::None,
            });
        }

        // for buffer_barrier in rafx_buffer_barriers {
        //     println!("{:?}", buffer_barrier);
        // }
        //
        // for rt_barrier in rt_barriers {
        //     println!("{:?}", rt_barrier);
        // }

        command_buffer.cmd_resource_barrier(&buffer_barriers, &image_barriers)
    }

    fn visit_renderpass_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitRenderpassNodeArgs,
    ) -> RafxResult<()> {
        if let Some(callback) = self.graph_plan.visit_node_callbacks.get(&node_id) {
            if let RenderGraphNodeVisitNodeCallback::Renderpass(render_callback) = callback {
                (render_callback)(args)?
            } else {
                let debug_name = args
                    .graph_context
                    .prepared_render_graph
                    .node_debug_name(node_id);
                log::error!("Tried to call a render node callback but a compute callback was registered for node {:?} ({:?})", node_id, debug_name);
            }
        } else {
            //let debug_name = args.graph_context.prepared_render_graph.node_debug_name(node_id);
            //log::error!("No callback found for node {:?} ({:?})", node_id, debug_name);
        }

        Ok(())
    }

    fn visit_compute_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitComputeNodeArgs,
    ) -> RafxResult<()> {
        if let Some(callback) = self.graph_plan.visit_node_callbacks.get(&node_id) {
            if let RenderGraphNodeVisitNodeCallback::Compute(compute_callback) = callback {
                (compute_callback)(args)?
            } else {
                let debug_name = args
                    .graph_context
                    .prepared_render_graph
                    .node_debug_name(node_id);
                log::error!("Tried to call a compute node callback but a render node callback was registered for node {:?} ({:?})", node_id, debug_name);
            }
        } else {
            //let debug_name = args.graph_context.prepared_render_graph.node_debug_name(node_id);
            //log::error!("No callback found for node {:?} {:?}", node_id, debug_name);
        }

        Ok(())
    }

    pub fn execute_graph<'write>(
        &'write self,
        write_context: &RenderJobWriteContext,
        prepared_render_data: PreparedRenderData<'write>,
        queue: &RafxQueue,
    ) -> RafxResult<Vec<DynCommandBuffer>> {
        profiling::scope!("Execute Graph");
        //
        // Start a command writer. For now just do a single primary writer, later we can multithread this.
        //
        let mut command_writer = self
            .resource_context
            .create_dyn_command_pool_allocator()
            .allocate_dyn_pool(queue, &RafxCommandPoolDef { transient: true }, 0)?;

        let command_buffer = command_writer.allocate_dyn_command_buffer(&RafxCommandBufferDef {
            is_secondary: false,
        })?;

        command_buffer.begin()?;

        let render_graph_context = RenderGraphContext {
            prepared_render_graph: &self,
            prepared_render_data: &prepared_render_data,
            render_resources: write_context.render_resources,
        };

        render_graph_context
            .prepared_render_data()
            .on_begin_execute_graph(
                &mut RenderJobBeginExecuteGraphContext::from_on_begin_execute_graph_args(
                    &OnBeginExecuteGraphArgs {
                        graph_context: render_graph_context.clone(),
                        command_buffer: command_buffer.clone(),
                    },
                ),
            )?;

        //
        // Iterate through all passes
        //
        for (pass_index, pass) in self.graph_plan.passes.iter().enumerate() {
            //TODO output pass is?
            //TODO: add_compute_node/add_render_node?

            profiling::scope!("pass", pass.debug_name().unwrap_or("unnamed"));
            log::trace!("Execute pass name: {:?}", pass.debug_name());

            let node_id = pass.node();

            if let Some(pre_pass_barrier) = pass.pre_pass_barrier() {
                log::trace!(
                    "prepass barriers for pass {} {:?}",
                    pass_index,
                    pass.debug_name()
                );
                self.insert_barriers(
                    &command_buffer,
                    &pre_pass_barrier.buffer_barriers,
                    &pre_pass_barrier.image_barriers,
                )?;
            }

            //TODO: Do we really need barriers here? We only do prepass clears when creating/modifying
            // storage buffers/storage images
            self.handle_resource_clears(&command_buffer, render_graph_context, pass)?;

            match pass {
                RenderGraphOutputPass::Renderpass(pass) => {
                    let color_images: Vec<_> = pass
                        .color_render_targets
                        .iter()
                        .map(|x| self.image_resources[&x.image].get_raw().image.clone())
                        .collect();

                    let resolve_images: Vec<_> = pass
                        .color_render_targets
                        .iter()
                        .map(|x| {
                            //x.map(|x| self.image_resources[&x.image].get_raw().image.clone())
                            x.resolve_image
                                .map(|x| self.image_resources[&x].get_raw().image.clone())
                        })
                        .collect();

                    let color_target_bindings: Vec<_> = pass
                        .color_render_targets
                        .iter()
                        .enumerate()
                        .map(
                            |(color_image_index, color_image)| RafxColorRenderTargetBinding {
                                texture: &color_images[color_image_index],
                                clear_value: color_image.clear_value.clone(),
                                load_op: color_image.load_op,
                                store_op: color_image.store_op,
                                array_slice: color_image.array_slice,
                                mip_slice: color_image.mip_slice,
                                resolve_target: resolve_images[color_image_index].as_ref(),
                                resolve_store_op: color_image.resolve_store_op.into(),
                                resolve_array_slice: color_image.resolve_array_slice,
                                resolve_mip_slice: color_image.resolve_mip_slice,
                            },
                        )
                        .collect();

                    let mut depth_stencil_image = None;
                    let depth_target_binding = pass.depth_stencil_render_target.as_ref().map(|x| {
                        depth_stencil_image =
                            Some(self.image_resources[&x.image].get_raw().image.clone());
                        RafxDepthStencilRenderTargetBinding {
                            texture: depth_stencil_image.as_ref().unwrap(),
                            clear_value: x.clear_value.clone(),
                            depth_load_op: x.depth_load_op,
                            stencil_load_op: x.stencil_load_op,
                            depth_store_op: x.depth_store_op,
                            stencil_store_op: x.stencil_store_op,
                            array_slice: x.array_slice,
                            mip_slice: x.mip_slice,
                        }
                    });

                    //println!("color bindings:\n{:#?}", color_target_bindings);
                    //println!("depth binding:\n{:#?}", depth_target_binding);

                    command_buffer
                        .cmd_begin_render_pass(&color_target_bindings, depth_target_binding)?;

                    let args = VisitRenderpassNodeArgs {
                        render_target_meta: pass.render_target_meta.clone(),
                        graph_context: render_graph_context,
                        command_buffer: command_buffer.clone(),
                    };

                    self.visit_renderpass_node(node_id, args)?;

                    command_buffer.cmd_end_render_pass()?;
                }
                RenderGraphOutputPass::Compute(_pass) => {
                    let args = VisitComputeNodeArgs {
                        graph_context: render_graph_context,
                        command_buffer: command_buffer.clone(),
                    };

                    self.visit_compute_node(node_id, args)?;
                }
            }

            if let Some(post_pass_barrier) = pass.post_pass_barrier() {
                log::trace!(
                    "postpass barriers for pass {} {:?}",
                    pass_index,
                    pass.debug_name()
                );
                self.insert_barriers(
                    &command_buffer,
                    &post_pass_barrier.buffer_barriers,
                    &post_pass_barrier.image_barriers,
                )?;
            }
        }

        command_buffer.end()?;

        Ok(vec![command_buffer])
    }

    fn handle_resource_clears(
        &self,
        command_buffer: &DynCommandBuffer,
        render_graph_context: RenderGraphContext,
        pass: &RenderGraphOutputPass,
    ) -> RafxResult<()> {
        if !pass.image_clears().is_empty() {
            unimplemented!("Image clears for storage buffers not yet implemented");
        }

        // Dispatch compute shader to clear buffers as needed
        if !pass.buffer_clears().is_empty() {
            let mut descriptor_set_allocator = render_graph_context
                .resource_context()
                .create_descriptor_set_allocator();

            let util_fill_buffer_pipeline = &self
                .resource_context
                .builtin_pipelines()
                .util_fill_buffer_pipeline;

            command_buffer.cmd_bind_pipeline(&*util_fill_buffer_pipeline.get_raw().pipeline)?;

            // Do the clear, there should be a barrier included in the pre_pass barrier to avoid
            // write after read hazards and other similar hazards
            for buffer_clear in pass.buffer_clears() {
                let buffer = &self.buffer_resources[buffer_clear];
                self.resource_context
                    .builtin_pipelines()
                    .do_fill_buffer_compute_pass(
                        &command_buffer,
                        &mut descriptor_set_allocator,
                        buffer,
                        0,
                    )?;
            }

            // We need a local list of buffers so we can make a list of barriers that reference them
            let buffers_to_clear: Vec<_> = pass
                .buffer_clears()
                .iter()
                .map(|buffer_id| self.buffer_resources[buffer_id].get_raw().buffer.clone())
                .collect();

            // Create the list of barriers
            let clear_barriers: Vec<_> = buffers_to_clear
                .iter()
                .map(|buffer| RafxBufferBarrier {
                    buffer,
                    src_state: RafxResourceState::UNORDERED_ACCESS,
                    dst_state: RafxResourceState::UNORDERED_ACCESS,
                    queue_transition: RafxBarrierQueueTransition::None,
                })
                .collect();

            command_buffer.cmd_resource_barrier(&clear_barriers, &[])?;
        }

        Ok(())
    }
}
