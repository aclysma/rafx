use super::slice_size_in_bytes;
use cocoa_foundation::foundation::NSRange;

#[derive(Debug)]
pub struct RafxBufferMetal {
    buffer: metal::Buffer,
    size_in_bytes: u64,
}

unsafe impl Send for RafxBufferMetal {}
unsafe impl Sync for RafxBufferMetal {}

impl RafxBufferMetal {
    // Only safe if the provided parameters are valid
    pub unsafe fn new_from_metal_buffer(
        buffer: metal::Buffer,
        size_in_bytes: u64,
    ) -> Self {
        assert_eq!(buffer.allocated_size(), size_in_bytes);
        RafxBufferMetal {
            buffer,
            size_in_bytes,
        }
    }

    pub fn new(
        device: &metal::Device,
        size_in_bytes: u64,
    ) -> Self {
        let buffer = device.new_buffer(
            size_in_bytes,
            metal::MTLResourceOptions::CPUCacheModeDefaultCache
                | metal::MTLResourceOptions::StorageModeManaged,
        );

        RafxBufferMetal {
            buffer,
            size_in_bytes,
        }
    }

    pub fn buffer(&self) -> &metal::Buffer {
        &self.buffer
    }

    pub fn size_in_bytes(&self) -> u64 {
        self.size_in_bytes
    }

    pub fn copy_to_buffer<T: Copy>(
        &self,
        data: &[T],
    ) {
        assert_eq!(self.size_in_bytes, slice_size_in_bytes(data) as u64);
        self.copy_to_buffer_with_offset(data, 0)
    }

    pub fn copy_to_buffer_with_offset<T: Copy>(
        &self,
        data: &[T],
        buffer_offset: u64,
    ) {
        let data_size_in_bytes = slice_size_in_bytes(data) as u64;
        assert!(buffer_offset + data_size_in_bytes <= self.size_in_bytes);

        let contents = self.buffer.contents() as *mut u8;
        let src = data.as_ptr() as *const u8;

        unsafe {
            let dst = contents.add(buffer_offset as usize);
            std::ptr::copy_nonoverlapping(src, dst, data_size_in_bytes as usize);
        }

        //TODO: Only if managed?
        self.buffer
            .did_modify_range(NSRange::new(buffer_offset, data_size_in_bytes));
    }
}
