use ash::vk;
use legion::*;
use renderer::assets::{ResourceManager, DynResourceAllocatorSet, ResourceManagerContext};
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
    pub resource_manager_context: ResourceManagerContext,
}

impl RenderJobPrepareContext {
    pub fn new(resource_manager_context: ResourceManagerContext) -> Self {
        RenderJobPrepareContext {
            resource_manager_context,
        }
    }
}

// Used to produce RenderJobWriteContexts per each job
pub struct RenderJobWriteContextFactory {
    pub device_context: VkDeviceContext,
    pub resource_manager_context: ResourceManagerContext,
}

impl RenderJobWriteContextFactory {
    pub fn new(
        device_context: VkDeviceContext,
        resource_manager_context: ResourceManagerContext,
    ) -> Self {
        RenderJobWriteContextFactory {
            device_context,
            resource_manager_context,
        }
    }

    pub fn create_context(
        &self,
        command_buffer: vk::CommandBuffer,
    ) -> RenderJobWriteContext {
        RenderJobWriteContext::new(
            self.device_context.clone(),
            self.resource_manager_context
                .create_dyn_resource_allocator_set(),
            command_buffer,
        )
    }
}

pub struct RenderJobWriteContext {
    pub device_context: VkDeviceContext,
    pub dyn_resource_lookups: DynResourceAllocatorSet,
    pub command_buffer: vk::CommandBuffer,
}

impl RenderJobWriteContext {
    pub fn new(
        device_context: VkDeviceContext,
        resource_allocators: DynResourceAllocatorSet,
        command_buffer: vk::CommandBuffer,
    ) -> Self {
        RenderJobWriteContext {
            device_context,
            dyn_resource_lookups: resource_allocators,
            command_buffer,
        }
    }
}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}
