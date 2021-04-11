use crate::gl::{RafxDeviceContextGl, NONE_BUFFER, BufferId};
use crate::{RafxBufferDef, RafxMemoryUsage, RafxResourceType, RafxResult};

use crate::gl::gles20;
use crate::gl::gles20::types::GLenum;
use std::sync::atomic::{AtomicU32, AtomicBool};
use std::sync::atomic::Ordering;


#[derive(Debug)]
pub struct RafxBufferGl {
    device_context: RafxDeviceContextGl,
    buffer_def: RafxBufferDef,
    buffer_id: Option<BufferId>,
    buffer_contents: Option<Vec<u8>>,
    buffer_contents_ptr: Option<*mut u8>,
    mapped_count: AtomicU32,
    target: GLenum, // may be gles20::NONE
}

// for gl_rs::Buffer
unsafe impl Send for RafxBufferGl {}
unsafe impl Sync for RafxBufferGl {}

impl RafxBufferGl {
    pub fn buffer_def(&self) -> &RafxBufferDef {
        &self.buffer_def
    }

    pub fn gl_buffer_d(&self) -> Option<BufferId> {
        self.buffer_id
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        self.mapped_count.fetch_add(1, Ordering::Acquire);
        assert_ne!(self.buffer_def.memory_usage, RafxMemoryUsage::GpuOnly);
        Ok(self.buffer_contents_ptr.unwrap())
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        // We flush on every unmap because if some code leaves the buffer in an "always mapped"
        // state the buffer would never get flushed
        if self.target != gles20::NONE {
            let gl_context = self.device_context.gl_context();
            gl_context.gl_bind_buffer(self.target, self.buffer_id.unwrap())?;
            gl_context.gl_buffer_sub_data(self.target, 0, self.buffer_def.size, self.buffer_contents.as_ref().unwrap().as_ptr())?;
            gl_context.gl_bind_buffer(self.target, NONE_BUFFER)?;
        }

        self.mapped_count.fetch_sub(1, Ordering::Release);
        Ok(())
    }

    pub fn mapped_memory(&self) -> Option<*mut u8> {
        if self.mapped_count.load(Ordering::Relaxed) > 0 {
            self.buffer_contents_ptr
        } else {
            None
        }
    }

    pub fn copy_to_host_visible_buffer<T: Copy>(
        &self,
        data: &[T],
    ) -> RafxResult<()> {
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

        unsafe {
            let dst = self.map_buffer()?.add(buffer_byte_offset as usize);
            assert_eq!(((dst as usize) % required_alignment), 0);
            std::ptr::copy_nonoverlapping(src, dst, data_size_in_bytes as usize);
        }

        self.unmap_buffer()?;

        Ok(())
    }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<Self> {
        buffer_def.verify();
        let mut buffer_def = buffer_def.clone();

        let mut buffer_id = None;
        let mut buffer_contents = None;
        let mut target = gles20::NONE;
        if buffer_def.resource_type.intersects(RafxResourceType::INDEX_BUFFER | RafxResourceType::VERTEX_BUFFER) {
            target = if buffer_def.resource_type.contains(RafxResourceType::INDEX_BUFFER) {
                gles20::ELEMENT_ARRAY_BUFFER
            } else {
                gles20::ARRAY_BUFFER
            };

            buffer_id = Some(device_context.gl_context().gl_create_buffer()?);
            device_context.gl_context().gl_bind_buffer(target, buffer_id.unwrap())?;

            let usage = buffer_def.memory_usage.gl_usage().unwrap();
            if usage != gles20::NONE {
                device_context.gl_context().gl_buffer_data(target, buffer_def.size, std::ptr::null(), usage);
            }

            device_context.gl_context().gl_bind_buffer(target, NONE_BUFFER);

            if buffer_def.memory_usage != RafxMemoryUsage::GpuOnly {
                buffer_contents = Some(vec![0_u8; buffer_def.size as _]);
            }
        } else {
            let mut allocation_size = buffer_def.size;
            if buffer_def.resource_type.intersects(RafxResourceType::UNIFORM_BUFFER) {
                allocation_size = rafx_base::memory::round_size_up_to_alignment_u64(
                    allocation_size,
                    device_context.device_info().min_uniform_buffer_offset_alignment as u64
                );
            }

            buffer_def.memory_usage = RafxMemoryUsage::CpuOnly;
            buffer_contents = Some(vec![0_u8; buffer_def.size as _]);
            target = gles20::NONE;
        }

        let buffer_contents_ptr = buffer_contents.as_mut().map(|x| x.as_mut_ptr());

        Ok(RafxBufferGl {
            device_context: device_context.clone(),
            buffer_def: buffer_def.clone(),
            buffer_id,
            buffer_contents,
            buffer_contents_ptr,
            mapped_count: AtomicU32::new(0),
            target
        })
    }
}
