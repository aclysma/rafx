#[cfg(feature = "rafx-metal")]
use crate::metal::RafxCommandPoolMetal;
use crate::vulkan::RafxCommandPoolVulkan;
use crate::{RafxCommandBuffer, RafxCommandBufferDef, RafxDeviceContext, RafxResult};

/// Create a single rafx API per process
pub enum RafxCommandPool {
    Vk(RafxCommandPoolVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxCommandPoolMetal),
}

impl RafxCommandPool {
    pub fn device_context(&self) -> RafxDeviceContext {
        match self {
            RafxCommandPool::Vk(inner) => RafxDeviceContext::Vk(inner.device_context().clone()),
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn reset_command_pool(&mut self) -> RafxResult<()> {
        match self {
            RafxCommandPool::Vk(inner) => inner.reset_command_pool(),
            // metal does not have the concept of command buffer pools in the API
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_) => Ok(()),
        }
    }

    pub fn create_command_buffer(
        &mut self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBuffer> {
        Ok(match self {
            RafxCommandPool::Vk(inner) => {
                RafxCommandBuffer::Vk(inner.create_command_buffer(command_buffer_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_inner) => unimplemented!(),
        })
    }

    pub fn vk_command_pool(&self) -> Option<&RafxCommandPoolVulkan> {
        match self {
            RafxCommandPool::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_command_pool(&self) -> Option<&RafxCommandPoolMetal> {
        match self {
            RafxCommandPool::Vk(_inner) => None,
            RafxCommandPool::Metal(inner) => Some(inner),
        }
    }
}
