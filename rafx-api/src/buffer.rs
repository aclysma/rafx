#[cfg(feature = "rafx-metal")]
use crate::metal::RafxBufferMetal;
use crate::vulkan::RafxBufferVulkan;
use crate::{RafxBufferDef, RafxResult};

/// A buffer is a piece of memory that can be accessed by the GPU. It may reside in CPU or GPU
/// memory depending on how it is created.
///
/// Buffers must not be dropped if they are in use by the GPU.
#[derive(Debug)]
pub enum RafxBuffer {
    Vk(RafxBufferVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxBufferMetal),
}

impl RafxBuffer {
    pub fn copy_to_host_visible_buffer<T: Copy>(
        &self,
        data: &[T],
    ) -> RafxResult<()> {
        match self {
            RafxBuffer::Vk(inner) => inner.copy_to_host_visible_buffer(data),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn copy_to_host_visible_buffer_with_offset<T: Copy>(
        &self,
        data: &[T],
        buffer_offset: u64,
    ) -> RafxResult<()> {
        match self {
            RafxBuffer::Vk(inner) => {
                inner.copy_to_host_visible_buffer_with_offset(data, buffer_offset)
            }
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn buffer_def(&self) -> &RafxBufferDef {
        match self {
            RafxBuffer::Vk(inner) => inner.buffer_def(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        match self {
            RafxBuffer::Vk(inner) => inner.map_buffer(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        match self {
            RafxBuffer::Vk(inner) => inner.unmap_buffer(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn mapped_memory(&self) -> Option<*mut u8> {
        match self {
            RafxBuffer::Vk(inner) => inner.mapped_memory(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn access(&self) -> RafxResult<(bool, *mut u8)> {
        match self {
            RafxBuffer::Vk(inner) => inner.access(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_buffer(&self) -> Option<&RafxBufferVulkan> {
        match self {
            RafxBuffer::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_buffer(&self) -> Option<&RafxBufferMetal> {
        match self {
            RafxBuffer::Vk(_inner) => None,
            RafxBuffer::Metal(inner) => Some(inner),
        }
    }
}
