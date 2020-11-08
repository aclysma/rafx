use ash::vk;
use legion::*;
use renderer::assets::AssetManager;
use renderer::resources::graph::VisitRenderpassArgs;
use renderer::resources::{RenderPassResource, ResourceArc, ResourceContext};
use renderer::vulkan::VkDeviceContext;

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
    pub device_context: VkDeviceContext,
    pub resource_context: ResourceContext,
}

impl RenderJobPrepareContext {
    pub fn new(
        device_context: VkDeviceContext,
        resource_context: ResourceContext,
    ) -> Self {
        RenderJobPrepareContext {
            device_context,
            resource_context,
        }
    }
}

pub struct RenderJobWriteContext {
    pub device_context: VkDeviceContext,
    pub resource_context: ResourceContext,
    pub command_buffer: vk::CommandBuffer,
    pub renderpass: ResourceArc<RenderPassResource>,
    pub subpass_index: usize,
}

impl RenderJobWriteContext {
    pub fn new(
        device_context: VkDeviceContext,
        resource_context: ResourceContext,
        command_buffer: vk::CommandBuffer,
        renderpass: ResourceArc<RenderPassResource>,
        subpass_index: usize,
    ) -> Self {
        RenderJobWriteContext {
            device_context,
            resource_context,
            command_buffer,
            renderpass,
            subpass_index,
        }
    }

    pub fn from_graph_visit_render_pass_args(
        visit_renderpass_args: &VisitRenderpassArgs
    ) -> RenderJobWriteContext {
        RenderJobWriteContext::new(
            visit_renderpass_args.graph_context.device_context().clone(),
            visit_renderpass_args
                .graph_context
                .resource_context()
                .clone(),
            visit_renderpass_args.command_buffer,
            visit_renderpass_args.renderpass.clone(),
            visit_renderpass_args.subpass_index,
        )
    }
}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}
