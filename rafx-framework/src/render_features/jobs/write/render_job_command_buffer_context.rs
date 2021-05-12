use crate::graph::VisitRenderpassNodeArgs;
use crate::{DynCommandBuffer, GraphicsPipelineRenderTargetMeta, ResourceContext};
use rafx_api::RafxDeviceContext;

pub struct RenderJobCommandBufferContext {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub command_buffer: DynCommandBuffer,
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
}

impl RenderJobCommandBufferContext {
    pub fn new(
        resource_context: ResourceContext,
        command_buffer: DynCommandBuffer,
        render_target_meta: GraphicsPipelineRenderTargetMeta,
    ) -> Self {
        RenderJobCommandBufferContext {
            device_context: resource_context.device_context().clone(),
            resource_context,
            command_buffer,
            render_target_meta,
        }
    }

    pub fn from_graph_visit_render_pass_args(
        args: &VisitRenderpassNodeArgs
    ) -> RenderJobCommandBufferContext {
        RenderJobCommandBufferContext::new(
            args.graph_context.resource_context().clone(),
            args.command_buffer.clone(),
            args.render_target_meta.clone(),
        )
    }
}
