#[cfg(feature = "rafx-metal")]
use crate::metal::RafxCommandPoolMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxCommandPoolVulkan;
use crate::{RafxCommandBuffer, RafxCommandBufferDef, RafxDeviceContext, RafxResult};

/// Creates a pool of command buffers. A command pool is necessary to create a command buffer.
///
/// A command pool cannot be modified (including allocating from it) if one of its command buffers
/// is being modified or in-use by the GPU.
///
/// Resetting a command pool clears all of the command buffers allocated from it, but the command
/// buffers remain allocated.
///
/// The command pool must not be dropped while any of its command buffers are in use. However, it
/// is ok to drop a command pool while command buffers are allocated, as long as those command
/// buffers are never used again. (The command pool owns the memory the command buffer points to)
pub enum RafxCommandPool {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxCommandPoolVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxCommandPoolMetal),
}

impl RafxCommandPool {
    pub fn device_context(&self) -> RafxDeviceContext {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => RafxDeviceContext::Vk(inner.device_context().clone()),
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => {
                RafxDeviceContext::Metal(inner.device_context().clone())
            }
        }
    }

    /// Allocate a command buffer from the pool. This must not be called if a command buffer from
    /// this pool is being written or is in-use by the GPU.
    pub fn create_command_buffer(
        &mut self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBuffer> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => {
                RafxCommandBuffer::Vk(inner.create_command_buffer(command_buffer_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => {
                RafxCommandBuffer::Metal(inner.create_command_buffer(command_buffer_def)?)
            }
        })
    }

    /// Reset all command buffers to an "unwritten" state. This must not be called if any command
    /// buffers allocated from this pool are in use by the GPU.
    ///
    /// This does not "free" allocated command buffers for reallocation.
    pub fn reset_command_pool(&mut self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => inner.reset_command_pool(),
            // metal does not have the concept of command buffer pools in the API
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => inner.reset_command_pool(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_command_pool(&self) -> Option<&RafxCommandPoolVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_command_pool(&self) -> Option<&RafxCommandPoolMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => Some(inner),
        }
    }
}
