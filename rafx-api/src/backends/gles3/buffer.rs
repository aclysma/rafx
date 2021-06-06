use crate::gles3::{BufferId, RafxDeviceContextGles3, NONE_BUFFER};
use crate::{RafxBufferDef, RafxMemoryUsage, RafxResourceType, RafxResult};

use crate::gles3::gles3_bindings;
use crate::gles3::gles3_bindings::types::GLenum;
use rafx_base::trust_cell::TrustCell;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

// This struct exists so that descriptor sets can (somewhat) safely point at the contents of a buffer
#[derive(Debug)]
pub(crate) struct Gles3BufferContentsInner {
    data: Option<TrustCell<Box<[u8]>>>,
    id: Option<BufferId>,
    allocation_size: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct Gles3BufferContents {
    inner: Arc<Gles3BufferContentsInner>,
}

impl Gles3BufferContents {
    pub fn new(
        data: Option<Vec<u8>>,
        id: Option<BufferId>,
        allocation_size: u64,
    ) -> Self {
        let inner = Gles3BufferContentsInner {
            data: data.map(|x| TrustCell::new(x.into_boxed_slice())),
            id,
            allocation_size,
        };

        Gles3BufferContents {
            inner: Arc::new(inner),
        }
    }

    #[allow(dead_code)]
    pub fn as_buffer_id(&self) -> BufferId {
        self.inner.id.unwrap()
    }

    pub unsafe fn try_as_ptr(&self) -> Option<*const u8> {
        Some(self.inner.data.as_ref()?.borrow().as_ptr())
    }

    pub unsafe fn try_as_mut_ptr(&self) -> Option<*mut u8> {
        Some(self.inner.data.as_ref()?.borrow_mut().as_mut_ptr())
    }

    #[allow(dead_code)]
    pub unsafe fn try_as_slice(&self) -> Option<&[u8]> {
        Some(self.inner.data.as_ref()?.borrow().value)
    }

    pub unsafe fn try_as_slice_with_offset(
        &self,
        offset: u64,
    ) -> Option<&[u8]> {
        Some(&self.inner.data.as_ref()?.borrow().value[offset as _..])
    }

    #[allow(dead_code)]
    pub fn allocation_size(&self) -> u64 {
        self.inner.allocation_size
    }
}

#[derive(Debug)]
pub struct RafxBufferGles3 {
    device_context: RafxDeviceContextGles3,
    buffer_def: RafxBufferDef,
    buffer_id: Option<BufferId>,
    buffer_contents: Gles3BufferContents,
    mapped_count: AtomicU32,
    target: GLenum, // may be gles30::NONE
}

impl Drop for RafxBufferGles3 {
    fn drop(&mut self) {
        if let Some(buffer_id) = self.buffer_id {
            self.device_context
                .gl_context()
                .gl_destroy_buffer(buffer_id)
                .unwrap();
        }
    }
}

impl RafxBufferGles3 {
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

    pub(crate) fn buffer_contents(&self) -> &Gles3BufferContents {
        &self.buffer_contents
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        self.mapped_count.fetch_add(1, Ordering::Acquire);
        assert_ne!(self.buffer_def.memory_usage, RafxMemoryUsage::GpuOnly);
        unsafe {
            Ok(self
                .buffer_contents
                .try_as_mut_ptr()
                .expect("Buffer must be CPU-visible to be mapped"))
        }
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        // We flush on every unmap because if some code leaves the buffer in an "always mapped"
        // state the buffer would never get flushed
        if self.target != gles3_bindings::NONE {
            let gl_context = self.device_context.gl_context();
            gl_context.gl_bind_buffer(self.target, self.buffer_id.unwrap())?;
            let ptr = unsafe {
                self.buffer_contents
                    .try_as_ptr()
                    .expect("Buffer must be CPU-visible to be unmapped")
            };
            gl_context.gl_buffer_sub_data(
                self.target,
                0,
                self.buffer_contents.inner.allocation_size as u64,
                ptr,
            )?;
            gl_context.gl_bind_buffer(self.target, NONE_BUFFER)?;
        }

        self.mapped_count.fetch_sub(1, Ordering::Release);
        Ok(())
    }

    pub fn mapped_memory(&self) -> Option<*mut u8> {
        if self.mapped_count.load(Ordering::Relaxed) > 0 {
            unsafe { self.buffer_contents.try_as_mut_ptr() }
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
        assert!(
            buffer_byte_offset + data_size_in_bytes <= self.buffer_contents.inner.allocation_size
        );

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
        device_context: &RafxDeviceContextGles3,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<Self> {
        buffer_def.verify();
        let mut buffer_def = buffer_def.clone();

        assert!(
            !buffer_def
                .resource_type
                .contains(RafxResourceType::VERTEX_BUFFER | RafxResourceType::INDEX_BUFFER),
            "GL ES 3.0 does not support buffers compatible with both vertex and index buffers"
        );

        let mut buffer_id = None;
        let mut buffer_contents = None;
        let target;

        let mut allocation_size = buffer_def.size;
        if buffer_def.resource_type.intersects(
            RafxResourceType::INDEX_BUFFER
                | RafxResourceType::VERTEX_BUFFER
                | RafxResourceType::UNIFORM_BUFFER,
        ) {
            target = if buffer_def
                .resource_type
                .contains(RafxResourceType::UNIFORM_BUFFER)
            {
                allocation_size = rafx_base::memory::round_size_up_to_alignment_u64(
                    allocation_size,
                    device_context
                        .device_info()
                        .min_uniform_buffer_offset_alignment as u64,
                );

                gles3_bindings::UNIFORM_BUFFER
            } else if buffer_def
                .resource_type
                .contains(RafxResourceType::INDEX_BUFFER)
            {
                gles3_bindings::ELEMENT_ARRAY_BUFFER
            } else {
                gles3_bindings::ARRAY_BUFFER
            };

            buffer_id = Some(device_context.gl_context().gl_create_buffer()?);
            device_context
                .gl_context()
                .gl_bind_buffer(target, buffer_id.unwrap())?;

            let usage = buffer_def.memory_usage.gles3_usage().unwrap();
            if usage != gles3_bindings::NONE {
                device_context.gl_context().gl_buffer_data(
                    target,
                    allocation_size,
                    std::ptr::null(),
                    usage,
                )?;
            }

            device_context
                .gl_context()
                .gl_bind_buffer(target, NONE_BUFFER)?;

            if buffer_def.memory_usage != RafxMemoryUsage::GpuOnly {
                buffer_contents = Some(vec![0_u8; allocation_size as _]);
            }
        } else {
            buffer_def.memory_usage = RafxMemoryUsage::CpuOnly;
            buffer_contents = Some(vec![0_u8; allocation_size as _]);
            target = gles3_bindings::NONE;
        }

        let buffer_contents = Gles3BufferContents::new(buffer_contents, buffer_id, allocation_size);

        Ok(RafxBufferGles3 {
            device_context: device_context.clone(),
            buffer_def: buffer_def.clone(),
            buffer_id,
            buffer_contents,
            mapped_count: AtomicU32::new(0),
            target,
        })
    }
}
