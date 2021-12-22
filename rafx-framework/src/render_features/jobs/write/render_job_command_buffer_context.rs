use crate::graph::{RenderGraphContext, VisitRenderpassNodeArgs};
use crate::{DynCommandBuffer, GraphicsPipelineRenderTargetMeta, ResourceContext};
use rafx_api::RafxDeviceContext;

pub struct RenderJobCommandBufferContext<'graph, 'write> {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub command_buffer: DynCommandBuffer,
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
    pub graph_context: RenderGraphContext<'graph, 'write>,
}

impl<'graph, 'write> RenderJobCommandBufferContext<'graph, 'write> {
    pub fn new(
        resource_context: ResourceContext,
        command_buffer: DynCommandBuffer,
        render_target_meta: GraphicsPipelineRenderTargetMeta,
        graph_context: RenderGraphContext<'graph, 'write>,
    ) -> Self {
        RenderJobCommandBufferContext {
            device_context: resource_context.device_context().clone(),
            resource_context,
            command_buffer,
            render_target_meta,
            graph_context,
        }
    }

    pub fn from_graph_visit_render_pass_args(
        args: &'graph VisitRenderpassNodeArgs<'graph, 'write>
    ) -> RenderJobCommandBufferContext<'graph, 'write> {
        RenderJobCommandBufferContext::new(
            args.graph_context.resource_context().clone(),
            args.command_buffer.clone(),
            args.render_target_meta.clone(),
            args.graph_context,
        )
    }
}
