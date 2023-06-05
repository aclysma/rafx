#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxCommandPoolDx12;
#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-dx12",
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2",
        feature = "rafx-gles3"
    ))
))]
use crate::empty::RafxCommandPoolEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxCommandPoolGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxCommandPoolGles3;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxCommandPoolMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxCommandPoolVulkan;
use crate::{RafxCommandBuffer, RafxCommandBufferDef, RafxDeviceContext, RafxResult};

/// A pool of command buffers. A command pool is necessary to create a command buffer.
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
    #[cfg(feature = "rafx-dx12")]
    Dx12(RafxCommandPoolDx12),
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxCommandPoolVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxCommandPoolMetal),
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxCommandPoolGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxCommandPoolGles3),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    Empty(RafxCommandPoolEmpty),
}

impl RafxCommandPool {
    pub fn device_context(&self) -> RafxDeviceContext {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(inner) => unimplemented!(), // RafxDeviceContext::Dx12(inner.device_context().clone()),
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => RafxDeviceContext::Vk(inner.device_context().clone()),
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => {
                RafxDeviceContext::Metal(inner.device_context().clone())
            }
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(inner) => {
                RafxDeviceContext::Gles2(inner.device_context().clone())
            }
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(inner) => {
                RafxDeviceContext::Gles3(inner.device_context().clone())
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(inner) => {
                RafxDeviceContext::Empty(inner.device_context().clone())
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
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(inner) => {
                RafxCommandBuffer::Dx12(inner.create_command_buffer(command_buffer_def)?)
            }
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => {
                RafxCommandBuffer::Vk(inner.create_command_buffer(command_buffer_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => {
                RafxCommandBuffer::Metal(inner.create_command_buffer(command_buffer_def)?)
            }
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(inner) => {
                RafxCommandBuffer::Gles2(inner.create_command_buffer(command_buffer_def)?)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(inner) => {
                RafxCommandBuffer::Gles3(inner.create_command_buffer(command_buffer_def)?)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(inner) => {
                RafxCommandBuffer::Empty(inner.create_command_buffer(command_buffer_def)?)
            }
        })
    }

    /// Reset all command buffers to an "unwritten" state. This must not be called if any command
    /// buffers allocated from this pool are in use by the GPU.
    ///
    /// This does not "free" allocated command buffers for reallocation.
    pub fn reset_command_pool(&mut self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(inner) => inner.reset_command_pool(),
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => inner.reset_command_pool(),
            // metal does not have the concept of command buffer pools in the API
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => inner.reset_command_pool(),
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(inner) => inner.reset_command_pool(),
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(inner) => inner.reset_command_pool(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(inner) => inner.reset_command_pool(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_command_pool(&self) -> Option<&RafxCommandPoolDx12> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(inner) => Some(inner),
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(_) => None,
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_command_pool(&self) -> Option<&RafxCommandPoolVulkan> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_command_pool(&self) -> Option<&RafxCommandPoolMetal> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_command_pool(&self) -> Option<&RafxCommandPoolGles2> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_command_pool(&self) -> Option<&RafxCommandPoolGles3> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxCommandPool::Gles3(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(_) => None,
        }
    }

    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    pub fn empty_command_pool(&self) -> Option<&RafxCommandPoolEmpty> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxCommandPool::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandPool::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxCommandPool::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxCommandPool::Gles2(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxCommandPool::Empty(inner) => Some(inner),
        }
    }
}
