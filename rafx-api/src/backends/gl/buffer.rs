use crate::gl::{BufferId, RafxDeviceContextGl, NONE_BUFFER};
use crate::{RafxBufferDef, RafxMemoryUsage, RafxResourceType, RafxResult};

use crate::gl::gles20;
use crate::gl::gles20::types::GLenum;
use rafx_base::trust_cell::TrustCell;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

// This struct exists so that descriptor sets can (somewhat) safely point at the contents of a buffer
#[derive(Debug, Clone)]
pub(crate) struct GlBufferContents {
    data: Arc<TrustCell<Box<[u8]>>>,
}

impl GlBufferContents {
    pub fn new(data: Vec<u8>) -> Self {
        GlBufferContents {
            data: Arc::new(TrustCell::new(data.into_boxed_slice())),
        }
    }

    // Get the ptr, this should be considered a raw FFI pointer that may or may not actually be
    // a unique reference
    pub unsafe fn as_ptr(&self) -> *const u8 {
        self.data.borrow().as_ptr()
    }

    pub unsafe fn as_mut_ptr(&self) -> *mut u8 {
        self.data.borrow_mut().as_mut_ptr()
    }

    pub unsafe fn as_slice(&self) -> &[u8] {
        self.data.borrow().value
    }
}

#[derive(Debug)]
pub struct RafxBufferGl {
    device_context: RafxDeviceContextGl,
    buffer_def: RafxBufferDef,
    buffer_id: Option<BufferId>,
    buffer_contents: Option<GlBufferContents>,
    mapped_count: AtomicU32,
    target: GLenum, // may be gles20::NONE
}

impl Drop for RafxBufferGl {
    fn drop(&mut self) {
        if let Some(buffer_id) = self.buffer_id {
            self.device_context
                .gl_context()
                .gl_destroy_buffer(buffer_id)
                .unwrap();
        }
    }
}

impl RafxBufferGl {
    pub fn buffer_def(&self) -> &RafxBufferDef {
        &self.buffer_def
    }

    // only some for vertex and index buffers
    pub fn gl_buffer_id(&self) -> Option<BufferId> {
        self.buffer_id
    }

    pub fn gl_target(&self) -> GLenum {
        self.target
    }

    pub(crate) fn buffer_contents(&self) -> &Option<GlBufferContents> {
        &self.buffer_contents
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        self.mapped_count.fetch_add(1, Ordering::Acquire);
        assert_ne!(self.buffer_def.memory_usage, RafxMemoryUsage::GpuOnly);
        unsafe { Ok(self.buffer_contents.as_ref().unwrap().as_mut_ptr()) }
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        // We flush on every unmap because if some code leaves the buffer in an "always mapped"
        // state the buffer would never get flushed
        if self.target != gles20::NONE {
            let gl_context = self.device_context.gl_context();
            gl_context.gl_bind_buffer(self.target, self.buffer_id.unwrap())?;
            let ptr = unsafe { self.buffer_contents.as_ref().unwrap().as_ptr() };
            gl_context.gl_buffer_sub_data(self.target, 0, self.buffer_def.size, ptr)?;
            gl_context.gl_bind_buffer(self.target, NONE_BUFFER)?;
        }

        self.mapped_count.fetch_sub(1, Ordering::Release);
        Ok(())
    }

    pub fn mapped_memory(&self) -> Option<*mut u8> {
        if self.mapped_count.load(Ordering::Relaxed) > 0 {
            self.buffer_contents
                .as_ref()
                .map(|x| unsafe { x.as_mut_ptr() })
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
        let target;
        if buffer_def
            .resource_type
            .intersects(RafxResourceType::INDEX_BUFFER | RafxResourceType::VERTEX_BUFFER)
        {
            target = if buffer_def
                .resource_type
                .contains(RafxResourceType::INDEX_BUFFER)
            {
                gles20::ELEMENT_ARRAY_BUFFER
            } else {
                gles20::ARRAY_BUFFER
            };

            buffer_id = Some(device_context.gl_context().gl_create_buffer()?);
            device_context
                .gl_context()
                .gl_bind_buffer(target, buffer_id.unwrap())?;

            let usage = buffer_def.memory_usage.gl_usage().unwrap();
            if usage != gles20::NONE {
                device_context.gl_context().gl_buffer_data(
                    target,
                    buffer_def.size,
                    std::ptr::null(),
                    usage,
                )?;
            }

            device_context
                .gl_context()
                .gl_bind_buffer(target, NONE_BUFFER)?;

            if buffer_def.memory_usage != RafxMemoryUsage::GpuOnly {
                buffer_contents = Some(vec![0_u8; buffer_def.size as _]);
            }
        } else {
            let mut allocation_size = buffer_def.size;
            if buffer_def
                .resource_type
                .intersects(RafxResourceType::UNIFORM_BUFFER)
            {
                allocation_size = rafx_base::memory::round_size_up_to_alignment_u64(
                    allocation_size,
                    device_context
                        .device_info()
                        .min_uniform_buffer_offset_alignment as u64,
                );
            }

            buffer_def.memory_usage = RafxMemoryUsage::CpuOnly;
            buffer_contents = Some(vec![0_u8; allocation_size as _]);
            target = gles20::NONE;
        }

        Ok(RafxBufferGl {
            device_context: device_context.clone(),
            buffer_def: buffer_def.clone(),
            buffer_id,
            buffer_contents: buffer_contents.map(|x| GlBufferContents::new(x)),
            mapped_count: AtomicU32::new(0),
            target,
        })
    }
}