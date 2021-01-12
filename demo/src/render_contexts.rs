use legion::*;
use rafx::api::RafxDeviceContext;
use rafx::assets::AssetManager;
use rafx::graph::VisitRenderpassNodeArgs;
use rafx::resources::ResourceContext;
use rafx::resources::{DynCommandBuffer, GraphicsPipelineRenderTargetMeta};

pub struct RenderJobExtractContext {
    pub world: &'static World,
    pub resources: &'static Resources,
    pub asset_manager: &'static AssetManager,
}

impl RenderJobExtractContext {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
        asset_manager: &'a AssetManager,
    ) -> Self {
        unsafe {
            RenderJobExtractContext {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
                asset_manager: force_to_static_lifetime(asset_manager),
            }
        }
    }
}

pub struct RenderJobPrepareContext {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
}

impl RenderJobPrepareContext {
    pub fn new(
        device_context: RafxDeviceContext,
        resource_context: ResourceContext,
    ) -> Self {
        RenderJobPrepareContext {
            device_context,
            resource_context,
        }
    }
}

pub struct RenderJobWriteContext {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub command_buffer: DynCommandBuffer,
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
    pub subpass_index: usize,
}

impl RenderJobWriteContext {
    pub fn new(
        device_context: RafxDeviceContext,
        resource_context: ResourceContext,
        command_buffer: DynCommandBuffer,
        render_target_meta: GraphicsPipelineRenderTargetMeta,
        subpass_index: usize,
    ) -> Self {
        RenderJobWriteContext {
            device_context,
            resource_context,
            command_buffer,
            render_target_meta,
            subpass_index,
        }
    }

    pub fn from_graph_visit_render_pass_args(
        visit_renderpass_args: &VisitRenderpassNodeArgs
    ) -> RenderJobWriteContext {
        RenderJobWriteContext::new(
            visit_renderpass_args.graph_context.device_context().clone(),
            visit_renderpass_args
                .graph_context
                .resource_context()
                .clone(),
            visit_renderpass_args.command_buffer.clone(),
            visit_renderpass_args.render_target_meta.clone(),
            visit_renderpass_args.subpass_index,
        )
    }
}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}
