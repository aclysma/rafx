use crate::metal::RafxDeviceContextMetal;
use crate::{RafxBufferDef, RafxMemoryUsage, RafxResourceType, RafxResult};

#[derive(Debug)]
pub struct RafxBufferMetal {
    device_context: RafxDeviceContextMetal,
    buffer_def: RafxBufferDef,
    buffer: metal_rs::Buffer,
}

// for metal_rs::Buffer
unsafe impl Send for RafxBufferMetal {}
unsafe impl Sync for RafxBufferMetal {}

impl RafxBufferMetal {
    pub fn buffer_def(&self) -> &RafxBufferDef {
        &self.buffer_def
    }

    pub fn metal_buffer(&self) -> &metal_rs::BufferRef {
        self.buffer.as_ref()
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        if self.buffer_def.memory_usage == RafxMemoryUsage::GpuOnly {
            return Err("Cannot map GPU-only buffer")?;
        }

        Ok(self.buffer.contents() as *mut u8)
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        // don't do anything, buffers are always mapped in metal
        Ok(())
    }

    pub fn copy_to_host_visible_buffer<T: Copy>(
        &self,
        data: &[T],
    ) -> RafxResult<()> {
        // Cannot check size of data == buffer because buffer size might be rounded up
        self.copy_to_host_visible_buffer_with_offset(data, 0)
    }

    pub fn copy_to_host_visible_buffer_with_offset<T: Copy>(
        &self,
        data: &[T],
        buffer_byte_offset: u64,
    ) -> RafxResult<()> {
        let data_size_in_bytes = rafx_base::memory::slice_size_in_bytes(data) as u64;
        assert!(buffer_byte_offset + data_size_in_bytes <= self.buffer_def.size);

        let src = data.as_ptr() as *const u8;

        let required_alignment = std::mem::align_of::<T>();

        // Buffers are always mapped, but calling map/unmap is essentially free and follows the same
        // codepath as other backends and end-users
        unsafe {
            let dst = self.map_buffer()?.add(buffer_byte_offset as usize);
            assert_eq!(((dst as usize) % required_alignment), 0);
            std::ptr::copy_nonoverlapping(src, dst, data_size_in_bytes as usize);
        }

        self.unmap_buffer()?;

        Ok(())
    }

    pub fn new(
        device_context: &RafxDeviceContextMetal,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<Self> {
        let mut allocation_size = buffer_def.size;
        if buffer_def
            .resource_type
            .intersects(RafxResourceType::UNIFORM_BUFFER)
        {
            allocation_size = rafx_base::memory::round_size_up_to_alignment_u64(
                buffer_def.size,
                device_context
                    .device_info()
                    .min_uniform_buffer_offset_alignment as u64,
            )
        }

        let buffer = device_context.device().new_buffer(
            allocation_size,
            buffer_def.memory_usage.mtl_resource_options(),
        );

        Ok(RafxBufferMetal {
            device_context: device_context.clone(),
            buffer_def: buffer_def.clone(),
            buffer,
        })
    }
}
