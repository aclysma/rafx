use super::PhysicalImageId;
use crate::graph::graph_buffer::PhysicalBufferId;
use crate::graph::graph_image::PhysicalImageViewId;
use crate::graph::graph_node::{RenderGraphNodeId, RenderGraphNodeName};
use crate::graph::graph_pass::{PrepassBufferBarrier, PrepassImageBarrier, RenderGraphOutputPass};
use crate::graph::graph_plan::RenderGraphPlan;
use crate::graph::{RenderGraphBufferUsageId, RenderGraphBuilder, RenderGraphImageUsageId};
use crate::resources::DynCommandBuffer;
use crate::resources::ResourceLookupSet;
use crate::{BufferResource, GraphicsPipelineRenderTargetMeta, ImageResource};
use crate::{ImageViewResource, ResourceArc, ResourceContext};
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::{
    RafxBarrierQueueTransition, RafxBufferBarrier, RafxColorRenderTargetBinding, RafxCommandBuffer,
    RafxCommandBufferDef, RafxCommandPoolDef, RafxDepthStencilRenderTargetBinding,
    RafxDeviceContext, RafxExtents2D, RafxFormat, RafxQueue, RafxResult, RafxTextureBarrier,
};
use crate::nodes::{RenderPhase, RenderPhaseIndex};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SwapchainSurfaceInfo {
    pub extents: RafxExtents2D,
    pub format: RafxFormat,
}

#[derive(Copy, Clone)]
pub struct RenderGraphContext<'a> {
    prepared_graph: &'a PreparedRenderGraph,
}

impl<'a> RenderGraphContext<'a> {
    pub fn buffer(
        &self,
        buffer: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        self.prepared_graph.buffer(buffer)
    }

    pub fn image_view(
        &self,
        image: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        self.prepared_graph.image_view(image)
    }

    pub fn device_context(&self) -> &RafxDeviceContext {
        &self.prepared_graph.device_context
    }

    pub fn resource_context(&self) -> &ResourceContext {
        &self.prepared_graph.resource_context
    }
}

pub struct OnBeginExecuteGraphArgs<'a> {
    pub command_buffer: DynCommandBuffer,
    pub graph_context: RenderGraphContext<'a>,
}

pub struct VisitComputeNodeArgs<'a> {
    pub command_buffer: DynCommandBuffer,
    pub graph_context: RenderGraphContext<'a>,
}

pub struct VisitRenderpassNodeArgs<'a> {
    pub command_buffer: DynCommandBuffer,
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
    pub graph_context: RenderGraphContext<'a>,
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
        resources: &ResourceLookupSet,
        graph: RenderGraphBuilder,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> RafxResult<Self> {
        let graph_plan = graph.build_plan();
        let mut cache_guard = resource_context.render_graph_cache().inner.lock().unwrap();
        let cache = &mut *cache_guard;

        profiling::scope!("allocate resources");
        let buffer_resources = cache.allocate_buffers(device_context, &graph_plan, resources)?;

        let image_resources = cache.allocate_images(
            device_context,
            &graph_plan,
            resources,
            swapchain_surface_info,
        )?;

        let image_view_resources =
            cache.allocate_image_views(&graph_plan, resources, &image_resources)?;

        // let render_pass_resources =
        //     cache.allocate_render_passes(&graph_plan, resources, swapchain_surface_info)?;
        //
        // let framebuffer_resources = cache.allocate_framebuffers(
        //     &graph_plan,
        //     resources,
        //     &image_view_resources,
        //     &render_pass_resources,
        // )?;

        Ok(PreparedRenderGraph {
            device_context: device_context.clone(),
            resource_context: resource_context.clone(),
            buffer_resources,
            image_resources,
            image_view_resources,
            //renderpass_resources: render_pass_resources,
            //framebuffer_resources,
            graph_plan,
        })
    }

    fn buffer(
        &self,
        buffer: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        let physical_buffer = self.graph_plan.buffer_usage_to_physical.get(&buffer)?;
        self.buffer_resources.get(physical_buffer).cloned()
    }

    fn image_view(
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

    pub fn execute_graph(
        &self,
        node_visitor: &dyn RenderGraphNodeVisitor,
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
            prepared_graph: &self,
        };

        let args = OnBeginExecuteGraphArgs {
            graph_context: render_graph_context,
            command_buffer: command_buffer.clone(),
        };

        node_visitor.execute_graph_begin(args)?;

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

                    node_visitor.visit_renderpass_node(node_id, args)?;

                    command_buffer.cmd_end_render_pass()?;
                }
                RenderGraphOutputPass::Compute(_pass) => {
                    let args = VisitComputeNodeArgs {
                        graph_context: render_graph_context,
                        command_buffer: command_buffer.clone(),
                    };

                    node_visitor.visit_compute_node(node_id, args)?;
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
}

pub trait RenderGraphNodeVisitor {
    fn execute_graph_begin(
        &self,
        args: OnBeginExecuteGraphArgs,
    ) -> RafxResult<()>;

    fn visit_renderpass_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitRenderpassNodeArgs,
    ) -> RafxResult<()>;

    fn visit_compute_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitComputeNodeArgs,
    ) -> RafxResult<()>;
}

type RenderGraphNodeBeginExecuteGraphCallback<RenderGraphUserContextT> =
    dyn Fn(OnBeginExecuteGraphArgs, &RenderGraphUserContextT) -> RafxResult<()> + Send;

type RenderGraphNodeVisitRenderpassNodeCallback<RenderGraphUserContextT> =
    dyn Fn(VisitRenderpassNodeArgs, &RenderGraphUserContextT) -> RafxResult<()> + Send;

type RenderGraphNodeVisitComputeNodeCallback<RenderGraphUserContextT> =
    dyn Fn(VisitComputeNodeArgs, &RenderGraphUserContextT) -> RafxResult<()> + Send;

enum RenderGraphNodeVisitNodeCallback<RenderGraphUserContextT> {
    Renderpass(Box<RenderGraphNodeVisitRenderpassNodeCallback<RenderGraphUserContextT>>),
    Compute(Box<RenderGraphNodeVisitComputeNodeCallback<RenderGraphUserContextT>>),
}

/// Created by RenderGraphNodeCallbacks::create_visitor(). Implements RenderGraphNodeVisitor and
/// forwards the call, adding the user context as a parameter.
struct RenderGraphNodeVisitorImpl<'b, RenderGraphUserContextT> {
    context: &'b RenderGraphUserContextT,
    begin_execute_graph_callback: &'b Option<Box<RenderGraphNodeBeginExecuteGraphCallback<RenderGraphUserContextT>>>,
    callbacks: &'b FnvHashMap<
        RenderGraphNodeId,
        RenderGraphNodeVisitNodeCallback<RenderGraphUserContextT>,
    >,
}

impl<'b, RenderGraphUserContextT> RenderGraphNodeVisitor
    for RenderGraphNodeVisitorImpl<'b, RenderGraphUserContextT>
{
    fn execute_graph_begin(
        &self,
        args: OnBeginExecuteGraphArgs,
    ) -> RafxResult<()> {
        if let Some(callback) = self.begin_execute_graph_callback {
            (callback)(args, self.context)?;
        }

        Ok(())
    }

    fn visit_renderpass_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitRenderpassNodeArgs,
    ) -> RafxResult<()> {
        if let Some(callback) = self.callbacks.get(&node_id) {
            if let RenderGraphNodeVisitNodeCallback::Renderpass(render_callback) = callback {
                (render_callback)(args, self.context)?
            } else {
                let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
                log::error!("Tried to call a render node callback but a compute callback was registered for node {:?} ({:?})", node_id, debug_name);
            }
        } else {
            //let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
            //log::error!("No callback found for node {:?} ({:?})", node_id, debug_name);
        }

        Ok(())
    }

    fn visit_compute_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitComputeNodeArgs,
    ) -> RafxResult<()> {
        if let Some(callback) = self.callbacks.get(&node_id) {
            if let RenderGraphNodeVisitNodeCallback::Compute(compute_callback) = callback {
                (compute_callback)(args, self.context)?
            } else {
                let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
                log::error!("Tried to call a compute node callback but a render node callback was registered for node {:?} ({:?})", node_id, debug_name);
            }
        } else {
            //let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
            //log::error!("No callback found for node {:?} {:?}", node_id, debug_name);
        }

        Ok(())
    }
}

/// All the callbacks associated with rendergraph nodes. We keep them separate from the nodes so
/// that we can avoid propagating generic parameters throughout the rest of the rendergraph code
pub struct RenderGraphNodeCallbacks<RenderGraphUserContextT> {
    callbacks:
        FnvHashMap<RenderGraphNodeId, RenderGraphNodeVisitNodeCallback<RenderGraphUserContextT>>,
    begin_execute_graph_callback: Option<Box<RenderGraphNodeBeginExecuteGraphCallback<RenderGraphUserContextT>>>,
    render_phase_dependencies: FnvHashMap<RenderGraphNodeId, FnvHashSet<RenderPhaseIndex>>,
}

impl<RenderGraphUserContextT> RenderGraphNodeCallbacks<RenderGraphUserContextT> {
    pub fn set_begin_execute_graph_callback<CallbackFnT>(
        &mut self,
        f: CallbackFnT
    ) where
        CallbackFnT: Fn(OnBeginExecuteGraphArgs, &RenderGraphUserContextT) -> RafxResult<()>
        + 'static
        + Send,
    {
        self.begin_execute_graph_callback = Some(Box::new(f));
    }

    /// Adds a callback that receives the renderpass associated with the node
    pub fn set_renderpass_callback<CallbackFnT>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: CallbackFnT,
    ) where
        CallbackFnT: Fn(VisitRenderpassNodeArgs, &RenderGraphUserContextT) -> RafxResult<()>
            + 'static
            + Send,
    {
        let old = self.callbacks.insert(
            node_id,
            RenderGraphNodeVisitNodeCallback::Renderpass(Box::new(f)),
        );
        // If this trips, multiple callbacks were set on the node
        assert!(old.is_none());
    }

    /// Adds a callback for compute based nodes
    pub fn set_compute_callback<CallbackFnT>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: CallbackFnT,
    ) where
        CallbackFnT:
            Fn(VisitComputeNodeArgs, &RenderGraphUserContextT) -> RafxResult<()> + 'static + Send,
    {
        let old = self.callbacks.insert(
            node_id,
            RenderGraphNodeVisitNodeCallback::Compute(Box::new(f)),
        );
        // If this trips, multiple callbacks were set on the node
        assert!(old.is_none());
    }

    pub fn add_render_phase_dependency<PhaseT: RenderPhase>(
        &mut self,
        node_id: RenderGraphNodeId,
    ) {
        self.render_phase_dependencies
            .entry(node_id)
            .or_default()
            .insert(PhaseT::render_phase_index());
    }

    /// Pass to PreparedRenderGraph::execute_graph, this will cause the graph to be executed,
    /// triggering any registered callbacks
    pub fn create_visitor<'a>(
        &'a self,
        context: &'a RenderGraphUserContextT,
    ) -> Box<dyn RenderGraphNodeVisitor + 'a> {
        Box::new(RenderGraphNodeVisitorImpl::<'a, RenderGraphUserContextT> {
            context,
            begin_execute_graph_callback: &self.begin_execute_graph_callback,
            callbacks: &self.callbacks,
        })
    }
}

impl<T> Default for RenderGraphNodeCallbacks<T> {
    fn default() -> Self {
        RenderGraphNodeCallbacks {
            callbacks: Default::default(),
            begin_execute_graph_callback: Default::default(),
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
        device_context: &RafxDeviceContext,
        resource_context: &ResourceContext,
        graph: RenderGraphBuilder,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        callbacks: RenderGraphNodeCallbacks<T>,
    ) -> RafxResult<Self> {
        //
        // Allocate the resources for the graph
        //
        let prepared_graph = PreparedRenderGraph::new(
            device_context,
            resource_context,
            resource_context.resources(),
            graph,
            swapchain_surface_info,
        )?;

        //
        // Pre-warm caches for pipelines that we may need
        //
        // for (node_id, render_phase_indices) in &callbacks.render_phase_dependencies {
        //     // Passes may get culled if the images are not used. This means the renderpass would
        //     // not be created so pipelines are also not needed
        //     if let Some(&renderpass_index) =
        //         prepared_graph.graph_plan.node_to_pass_index.get(node_id)
        //     {
        //         let renderpass = &prepared_graph.renderpass_resources[renderpass_index];
        //         if let Some(renderpass) = renderpass {
        //             for &render_phase_index in render_phase_indices {
        //                 resource_context
        //                     .graphics_pipeline_cache()
        //                     .register_renderpass_to_phase_index_per_frame(
        //                         renderpass,
        //                         render_phase_index,
        //                     )
        //             }
        //         } else {
        //             log::error!("add_render_phase_dependency was called on node {:?} ({:?}) that is not a renderpass", node_id, prepared_graph.graph_plan.passes[renderpass_index].debug_name());
        //         }
        //     }
        // }
        // resource_context
        //     .graphics_pipeline_cache()
        //     .precache_pipelines_for_all_phases()?;

        //
        // Return the executor which can be triggered later
        //
        Ok(RenderGraphExecutor {
            prepared_graph,
            callbacks,
        })
    }

    pub fn buffer_resource(
        &self,
        buffer_usage: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        let buffer = self
            .prepared_graph
            .graph_plan
            .buffer_usage_to_physical
            .get(&buffer_usage)?;
        Some(self.prepared_graph.buffer_resources[buffer].clone())
    }

    pub fn image_resource(
        &self,
        image_usage: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageResource>> {
        let image = self
            .prepared_graph
            .graph_plan
            .image_usage_to_physical
            .get(&image_usage)?;
        Some(self.prepared_graph.image_resources[image].clone())
    }

    pub fn image_view_resource(
        &self,
        image_usage: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        let image = self
            .prepared_graph
            .graph_plan
            .image_usage_to_view
            .get(&image_usage)?;
        Some(self.prepared_graph.image_view_resources[image].clone())
    }

    /// Executes the graph, passing through the given context parameter
    pub fn execute_graph(
        self,
        context: &T,
        queue: &RafxQueue,
    ) -> RafxResult<Vec<DynCommandBuffer>> {
        let visitor = self.callbacks.create_visitor(context);
        self.prepared_graph.execute_graph(&*visitor, queue)
    }
}
