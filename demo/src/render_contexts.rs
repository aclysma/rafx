use ash::vk;
use legion::*;
use renderer::assets::{ResourceManager, ResourceContext, ResourceArc, RenderPassResource};
use renderer::vulkan::VkDeviceContext;

pub struct RenderJobExtractContext {
    pub world: &'static World,
    pub resources: &'static Resources,
    pub resource_manager: &'static ResourceManager,
}

impl RenderJobExtractContext {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
        resource_manager: &'a ResourceManager,
    ) -> Self {
        unsafe {
            RenderJobExtractContext {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
                resource_manager: force_to_static_lifetime(resource_manager),
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

// Used to produce RenderJobWriteContexts per each job
pub struct RenderJobWriteContextFactory {
    pub device_context: VkDeviceContext,
    pub resource_context: ResourceContext,
}

impl RenderJobWriteContextFactory {
    pub fn new(
        device_context: VkDeviceContext,
        resource_context: ResourceContext,
    ) -> Self {
        RenderJobWriteContextFactory {
            device_context,
            resource_context,
        }
    }

    pub fn create_context(
        &self,
        command_buffer: vk::CommandBuffer,
        renderpass: ResourceArc<RenderPassResource>,
        subpass_index: usize,
    ) -> RenderJobWriteContext {
        RenderJobWriteContext::new(
            self.device_context.clone(),
            self.resource_context.clone(),
            command_buffer,
            renderpass,
            subpass_index,
        )
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
}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}
