use ash::vk;
use legion::prelude::*;
use renderer::resources::{ResourceManager, DynResourceAllocatorSet};
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
    pub dyn_resource_lookups: DynResourceAllocatorSet,
}

impl RenderJobPrepareContext {
    pub fn new(resource_allocators: DynResourceAllocatorSet) -> Self {
        RenderJobPrepareContext {
            dyn_resource_lookups: resource_allocators,
        }
    }
}

// Used to produce RenderJobWriteContexts per each job
pub struct RenderJobWriteContextFactory {
    pub device_context: VkDeviceContext,
    pub dyn_resource_lookups: DynResourceAllocatorSet,
}

impl RenderJobWriteContextFactory {
    pub fn new(
        device_context: VkDeviceContext,
        resource_allocators: DynResourceAllocatorSet,
    ) -> Self {
        RenderJobWriteContextFactory {
            device_context,
            dyn_resource_lookups: resource_allocators,
        }
    }

    pub fn create_context(
        &self,
        command_buffer: vk::CommandBuffer,
    ) -> RenderJobWriteContext {
        RenderJobWriteContext {
            device_context: self.device_context.clone(),
            dyn_resource_lookups: self.dyn_resource_lookups.clone(),
            command_buffer,
        }
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
unsafe fn force_to_static_lifetime_mut<T>(value: &mut T) -> &'static mut T {
    std::mem::transmute(value)
}
