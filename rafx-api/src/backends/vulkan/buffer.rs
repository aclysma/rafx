use crate::vulkan::RafxDeviceContextVulkan;
use crate::*;
use ash::version::DeviceV1_0;
use ash::vk;
use ash::vk::Handle;

#[derive(Clone, Debug)]
pub struct RafxBufferRaw {
    pub buffer: vk::Buffer,
    pub allocation: gpu_allocator::SubAllocation,
}

#[derive(Debug)]
pub struct RafxBufferVulkan {
    device_context: RafxDeviceContextVulkan,
    buffer_raw: Option<RafxBufferRaw>,

    buffer_def: RafxBufferDef,
    uniform_texel_view: Option<vk::BufferView>,
    storage_texel_view: Option<vk::BufferView>,
}

impl RafxBufferVulkan {
    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer_raw.as_ref().unwrap().buffer
    }

    pub fn vk_uniform_texel_view(&self) -> Option<vk::BufferView> {
        self.uniform_texel_view
    }

    pub fn vk_storage_texel_view(&self) -> Option<vk::BufferView> {
        self.storage_texel_view
    }

    pub fn take_raw(mut self) -> Option<RafxBufferRaw> {
        let mut raw = None;
        std::mem::swap(&mut raw, &mut self.buffer_raw);
        raw
    }

    pub fn buffer_def(&self) -> &RafxBufferDef {
        &self.buffer_def
    }

    pub fn set_debug_name(
        &self,
        name: impl AsRef<str>,
    ) {
        if self.device_context.device_info().debug_names_enabled {
            if let Some(debug_reporter) = self.device_context.debug_reporter() {
                debug_reporter.set_debug_labels(
                    &self.device_context,
                    vk::ObjectType::BUFFER,
                    self.vk_buffer().as_raw(),
                    name,
                );
            }
        }
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        self.mapped_memory().ok_or(RafxError::StringError(
            "Tried to map a buffer that is not CPU-visible".to_string(),
        ))
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        Ok(())
    }

    pub fn mapped_memory(&self) -> Option<*mut u8> {
        self.buffer_raw
            .as_ref()
            .unwrap()
            .allocation
            .mapped_ptr()
            .map(|x| x.as_ptr() as *mut u8)
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

        unsafe {
            let dst = self.map_buffer()?.add(buffer_byte_offset as usize);
            assert_eq!(((dst as usize) % required_alignment), 0);
            std::ptr::copy_nonoverlapping(src, dst, data_size_in_bytes as usize);
        }

        self.unmap_buffer()?;

        Ok(())
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<Self> {
        buffer_def.verify();
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

        assert_ne!(allocation_size, 0);

        let buffer_info = vk::BufferCreateInfo::builder()
            .size(allocation_size)
            .usage(usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let device = device_context.device();

        let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation = device_context.allocator().lock().unwrap().allocate(
            &gpu_allocator::AllocationCreateDesc {
                name: "",
                linear: true,
                location: buffer_def.memory_usage.into(),
                requirements,
            },
        )?;

        unsafe {
            device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;
        }

        let buffer_raw = RafxBufferRaw { buffer, allocation };

        log::trace!(
            "Buffer {:?} crated with size {} (always mapped: {:?})",
            buffer_raw.buffer,
            buffer_info.size,
            buffer_def.always_mapped
        );

        // let mut buffer_offset = 0;
        // if buffer_def.resource_type.intersects(RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE) {
        //     buffer_offset = buffer_def.struct_stride * buffer_def.first_element;
        // }

        let uniform_texel_view = if usage_flags
            .intersects(vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER)
        {
            let create_info = vk::BufferViewCreateInfo::builder()
                .buffer(buffer_raw.buffer)
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
                .buffer(buffer_raw.buffer)
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

        Ok(RafxBufferVulkan {
            device_context: device_context.clone(),
            buffer_raw: Some(buffer_raw),
            buffer_def: buffer_def.clone(),
            uniform_texel_view,
            storage_texel_view,
        })
    }
}

impl Drop for RafxBufferVulkan {
    fn drop(&mut self) {
        log::trace!("destroying RafxBufferVulkanInner");
        let device = self.device_context.device();
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

        if let Some(buffer_raw) = self.buffer_raw.take() {
            log::trace!(
                "Buffer {:?} destroying with size {} (always mapped: {:?})",
                buffer_raw.buffer,
                self.buffer_def.size,
                self.buffer_def.always_mapped
            );

            unsafe {
                self.device_context
                    .device()
                    .destroy_buffer(buffer_raw.buffer, None);
            }
            self.device_context
                .allocator()
                .lock()
                .unwrap()
                .free(buffer_raw.allocation)
                .unwrap();
        }

        log::trace!("destroyed RafxBufferVulkanInner");
    }
}
