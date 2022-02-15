use crate::graph::{RenderGraphContext, VisitRenderpassNodeArgs};
use crate::{DynCommandBuffer, GraphicsPipelineRenderTargetMeta, RenderResources, ResourceContext};
use rafx_api::RafxDeviceContext;

pub struct RenderJobCommandBufferContext<'graph, 'write> {
    //NOTE: render_resources is included in the graph context
    pub command_buffer: DynCommandBuffer,
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
    pub graph_context: RenderGraphContext<'graph, 'write>,
}

impl<'graph, 'write> RenderJobCommandBufferContext<'graph, 'write> {
    pub fn new(
        command_buffer: DynCommandBuffer,
        render_target_meta: GraphicsPipelineRenderTargetMeta,
        graph_context: RenderGraphContext<'graph, 'write>,
    ) -> Self {
        RenderJobCommandBufferContext {
            command_buffer,
            render_target_meta,
            graph_context,
        }
    }

    pub fn from_graph_visit_render_pass_args(
        args: &'graph VisitRenderpassNodeArgs<'graph, 'write>
    ) -> RenderJobCommandBufferContext<'graph, 'write> {
        RenderJobCommandBufferContext::new(
            args.command_buffer.clone(),
            args.render_target_meta.clone(),
            args.graph_context,
        )
    }

    pub fn device_context(&self) -> &RafxDeviceContext {
        &self.graph_context.device_context()
    }

    pub fn resource_context(&self) -> &ResourceContext {
        &self.graph_context.resource_context()
    }

    pub fn command_buffer(&self) -> &DynCommandBuffer {
        &self.command_buffer
    }

    pub fn render_target_meta(&self) -> &GraphicsPipelineRenderTargetMeta {
        &self.render_target_meta
    }

    pub fn graph_context(&self) -> &RenderGraphContext<'graph, 'write> {
        &self.graph_context
    }

    pub fn render_resources(&self) -> &RenderResources {
        self.graph_context.render_resources()
    }
}
