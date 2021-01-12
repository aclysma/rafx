use crate::{RafxBufferDef, RafxFormat, RafxMemoryUsage, RafxResourceType, RafxResult};
use ash::vk;

#[derive(Debug)]
struct RafxBufferVulkanInner {
    buffer: VkBuffer,
    buffer_def: RafxBufferDef,
    uniform_texel_view: Option<vk::BufferView>,
    storage_texel_view: Option<vk::BufferView>,
}

impl Drop for RafxBufferVulkanInner {
    fn drop(&mut self) {
        let device = self.buffer.device_context.device();
        if let Some(uniform_texel_view) = self.uniform_texel_view {
            unsafe {
                device.destroy_buffer_view(uniform_texel_view, None);
            }
        }
        if let Some(storage_texel_view) = self.storage_texel_view {
            unsafe {
                device.destroy_buffer_view(storage_texel_view, None);
            }
        }
    }
}

#[derive(Debug)]
pub struct RafxBufferVulkan {
    inner: RafxBufferVulkanInner,
}

impl RafxBufferVulkan {
    pub fn vk_buffer(&self) -> vk::Buffer {
        self.inner.buffer.buffer()
    }

    pub fn vk_uniform_texel_view(&self) -> Option<vk::BufferView> {
        self.inner.uniform_texel_view
    }

    pub fn vk_storage_texel_view(&self) -> Option<vk::BufferView> {
        self.inner.storage_texel_view
    }

    pub fn buffer_def(&self) -> &RafxBufferDef {
        &self.inner.buffer_def
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        Ok(self
            .inner
            .buffer
            .device_context()
            .allocator()
            .map_memory(&self.inner.buffer.allocation())?)
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        Ok(self
            .inner
            .buffer
            .device_context()
            .allocator()
            .unmap_memory(&self.inner.buffer.allocation())?)
    }

    pub fn mapped_memory(&self) -> Option<*mut u8> {
        let ptr = self.inner.buffer.allocation_info().get_mapped_data();
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    pub fn access(&self) -> RafxResult<(bool, *mut u8)> {
        let was_mapped = self.inner.buffer.allocation_info().get_mapped_data();
        Ok(if was_mapped.is_null() {
            (false, self.map_buffer(/*read_range*/)?)
        } else {
            (true, was_mapped)
        })
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
        buffer_offset: u64,
    ) -> RafxResult<()> {
        let data_size_in_bytes = rafx_base::memory::slice_size_in_bytes(data) as u64;
        assert!(buffer_offset + data_size_in_bytes <= self.inner.buffer.size());

        let (was_mapped, contents) = self.access()?;
        let src = data.as_ptr() as *const u8;

        let required_alignment = std::mem::align_of::<T>();

        unsafe {
            let dst = contents.add(buffer_offset as usize);
            assert_eq!(((dst as usize) % required_alignment), 0);
            std::ptr::copy_nonoverlapping(src, dst, data_size_in_bytes as usize);
        }

        if !was_mapped {
            self.unmap_buffer()?;
        }

        //self.inner.buffer.write_to_host_visible_buffer_with_offset(data, buffer_offset)?;

        Ok(())
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<Self> {
        let mut allocation_size = buffer_def.size;
        if buffer_def
            .resource_type
            .intersects(RafxResourceType::UNIFORM_BUFFER)
        {
            allocation_size = rafx_base::memory::round_size_up_to_alignment_u64(
                buffer_def.size,
                device_context.limits().min_uniform_buffer_offset_alignment,
            )
        }

        let mut usage_flags = super::util::resource_type_buffer_usage_flags(
            buffer_def.resource_type,
            buffer_def.format != RafxFormat::UNDEFINED,
        );

        if buffer_def.memory_usage == RafxMemoryUsage::GpuOnly
            || buffer_def.memory_usage == RafxMemoryUsage::CpuToGpu
        {
            usage_flags |= vk::BufferUsageFlags::TRANSFER_DST;
        }

        let buffer = VkBuffer::new(
            device_context,
            buffer_def.memory_usage.into(),
            usage_flags,
            vk::MemoryPropertyFlags::empty(),
            allocation_size,
            buffer_def.always_mapped,
        )?;

        // let mapped_buffer_ptr = AtomicPtr::new(if buffer_def.always_mapped {
        //     buffer.allocation_info.get_mapped_data()
        // } else {
        //     std::ptr::null_mut::<u8>()
        // });

        // let mut buffer_offset = 0;
        // if buffer_def.resource_type.intersects(RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE) {
        //     buffer_offset = buffer_def.struct_stride * buffer_def.first_element;
        // }

        let uniform_texel_view = if usage_flags
            .intersects(vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER)
        {
            let create_info = vk::BufferViewCreateInfo::builder()
                .buffer(buffer.buffer())
                .format(buffer_def.format.into())
                .offset(
                    buffer_def.elements.element_stride * buffer_def.elements.element_begin_index,
                )
                .range(
                    buffer_def.elements.element_stride * buffer_def.elements.element_begin_index,
                );

            //TODO: Verify we support the format
            unsafe {
                Some(
                    device_context
                        .device()
                        .create_buffer_view(&*create_info, None)?,
                )
            }
        } else {
            None
        };

        let storage_texel_view = if usage_flags
            .intersects(vk::BufferUsageFlags::STORAGE_TEXEL_BUFFER)
        {
            let create_info = vk::BufferViewCreateInfo::builder()
                .buffer(buffer.buffer())
                .format(buffer_def.format.into())
                .offset(
                    buffer_def.elements.element_stride * buffer_def.elements.element_begin_index,
                )
                .range(
                    buffer_def.elements.element_stride * buffer_def.elements.element_begin_index,
                );

            //TODO: Verify we support the format
            unsafe {
                Some(
                    device_context
                        .device()
                        .create_buffer_view(&*create_info, None)?,
                )
            }
        } else {
            None
        };

        let inner = RafxBufferVulkanInner {
            buffer,
            buffer_def: buffer_def.clone(),
            uniform_texel_view,
            storage_texel_view,
        };

        Ok(RafxBufferVulkan { inner })
    }
}

use crate::vulkan::RafxDeviceContextVulkan;
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;

#[derive(Copy, Clone, Debug)]
pub struct VkBufferRaw {
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
}

//TODO: Merge this into RafxBufferVulkan
/// Represents a vulkan buffer (vertex, index, image, etc.)
#[derive(Debug)]
pub struct VkBuffer {
    pub device_context: RafxDeviceContextVulkan,
    pub allocation_info: vk_mem::AllocationInfo,
    pub raw: Option<VkBufferRaw>,
    always_mapped: bool,
}

impl VkBuffer {
    pub fn device_context(&self) -> &RafxDeviceContextVulkan {
        &self.device_context
    }

    pub fn size(&self) -> vk::DeviceSize {
        self.allocation_info.get_size() as vk::DeviceSize
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        memory_usage: vk_mem::MemoryUsage,
        buffer_usage: vk::BufferUsageFlags,
        required_property_flags: vk::MemoryPropertyFlags,
        size: vk::DeviceSize,
        always_mapped: bool,
    ) -> VkResult<Self> {
        let mut flags = vk_mem::AllocationCreateFlags::NONE;
        if always_mapped {
            flags |= vk_mem::AllocationCreateFlags::MAPPED;
        }

        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            flags,
            required_flags: required_property_flags,
            preferred_flags: vk::MemoryPropertyFlags::empty(),
            memory_type_bits: 0, // Do not exclude any memory types
            pool: None,
            user_data: None,
        };

        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(buffer_usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        //TODO: Better way of handling allocator errors
        let (buffer, allocation, allocation_info) = device_context
            .allocator()
            .create_buffer(&buffer_info, &allocation_create_info)
            .map_err(|e| {
                log::error!("Error creating buffer {:?}", e);
                vk::Result::ERROR_UNKNOWN
            })?;

        let raw = VkBufferRaw { buffer, allocation };

        log::trace!(
            "Buffer {:?} crated with size {} (always mapped: {:?})",
            raw.buffer,
            buffer_info.size,
            always_mapped
        );

        Ok(VkBuffer {
            device_context: device_context.clone(),
            allocation_info,
            always_mapped,
            raw: Some(raw),
        })
    }

    pub fn buffer(&self) -> vk::Buffer {
        // Raw is only none if take_raw has not been called, and take_raw consumes the VkBuffer
        self.raw.unwrap().buffer
    }

    pub fn allocation(&self) -> vk_mem::Allocation {
        // Raw is only none if take_raw has not been called, and take_raw consumes the VkBuffer
        self.raw.unwrap().allocation
    }

    pub fn allocation_info(&self) -> &vk_mem::AllocationInfo {
        &self.allocation_info
    }

    pub fn take_raw(mut self) -> Option<VkBufferRaw> {
        let mut raw = None;
        std::mem::swap(&mut raw, &mut self.raw);
        raw
    }
}

impl Drop for VkBuffer {
    fn drop(&mut self) {
        log::trace!("destroying VkBuffer");

        if let Some(raw) = &self.raw {
            log::trace!(
                "Buffer {:?} destroying with size {} (always mapped: {:?})",
                self.buffer(),
                self.size(),
                self.always_mapped
            );
            if self.always_mapped {
                self.device_context
                    .allocator()
                    .unmap_memory(&self.allocation())
                    .unwrap();
            }

            self.device_context
                .allocator()
                .destroy_buffer(raw.buffer, &raw.allocation)
                .unwrap();
        }

        log::trace!("destroyed VkBuffer");
    }
}
