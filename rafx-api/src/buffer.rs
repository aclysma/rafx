#[cfg(feature = "rafx-metal")]
use crate::metal::RafxBufferMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxBufferVulkan;
use crate::{RafxBufferDef, RafxResult};

/// A buffer is a piece of memory that can be accessed by the GPU. It may reside in CPU or GPU
/// memory depending on how it is created.
///
/// Buffers must not be dropped if they are in use by the GPU.
#[derive(Debug)]
pub enum RafxBuffer {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxBufferVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxBufferMetal),
}

impl RafxBuffer {
    /// Copy all the data in the given slice into the buffer. This function will fail if the buffer
    /// is not writable by the CPU. This function will assert/panic if the buffer is too small to
    /// hold the data.
    pub fn copy_to_host_visible_buffer<T: Copy>(
        &self,
        data: &[T],
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxBuffer::Vk(inner) => inner.copy_to_host_visible_buffer(data),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(inner) => inner.copy_to_host_visible_buffer(data),
        }
    }

    /// Copy all the data in the given slice into the buffer with a given offset. The offset is in
    /// bytes. This function will assert/panic if the size of the buffer <= size of data + offset
    pub fn copy_to_host_visible_buffer_with_offset<T: Copy>(
        &self,
        data: &[T],
        buffer_byte_offset: u64,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxBuffer::Vk(inner) => {
                inner.copy_to_host_visible_buffer_with_offset(data, buffer_byte_offset)
            }
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(inner) => {
                inner.copy_to_host_visible_buffer_with_offset(data, buffer_byte_offset)
            }
        }
    }

    /// Return the definition used to create the buffer
    pub fn buffer_def(&self) -> &RafxBufferDef {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxBuffer::Vk(inner) => inner.buffer_def(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(inner) => inner.buffer_def(),
        }
    }

    /// Map the contents of the buffer into CPU memory. This function will fail if the buffer is not
    /// possible to map into CPU memory (i.e. it's GPU-only).
    ///
    /// The mappings are "ref-counted". Repeated calls to map the same buffer are permitted and the
    /// buffer will remain mapped until an equal number of calls to unmap_buffer are made.
    ///
    /// Generally speaking, keeping a buffer mapped for its entire lifetime is acceptable.
    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxBuffer::Vk(inner) => inner.map_buffer(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(inner) => inner.map_buffer(),
        }
    }

    /// Unmap the contents of the buffer from CPU memory. This function will fail if the buffer is
    /// not possible to map into CPU memory (i.e. it's GPU-only). It will also fail if the buffer
    /// is not currently mapped.
    ///
    /// The mappings are "ref-counted". Repeated calls to map the same buffer are permitted and the
    /// buffer will remain mapped until an equal number of calls to unmap_buffer are made.
    pub fn unmap_buffer(&self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxBuffer::Vk(inner) => inner.unmap_buffer(),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(inner) => inner.unmap_buffer(),
        }
    }

    // This API call is currently disabled due to a bug in vk_mem. For now, call map_buffer() and
    // unmap_buffer() and use the returned pointer from map_buffer()
    // https://github.com/gwihlidal/vk-mem-rs/issues/43
    // /// Obtain a pointer to the mapped memory. If the buffer is not mapped, None is returned.
    // pub fn mapped_memory(&self) -> Option<*mut u8> {
    //     match self {
    //         #[cfg(feature = "rafx-vulkan")]
    //         RafxBuffer::Vk(inner) => inner.mapped_memory(),
    //         #[cfg(feature = "rafx-metal")]
    //         RafxBuffer::Metal(inner) => inner.mapped_memory(),
    //     }
    // }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_buffer(&self) -> Option<&RafxBufferVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxBuffer::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_buffer(&self) -> Option<&RafxBufferMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxBuffer::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
            RafxBuffer::Metal(inner) => Some(inner),
        }
    }
}
