use crate::dx12::{Dx12DescriptorId, RafxDeviceContextDx12};
use crate::{
    RafxBufferDef, RafxMemoryUsage, RafxQueueType, RafxResourceState, RafxResourceType, RafxResult,
};
use rafx_base::trust_cell::TrustCell;
use std::sync::atomic::{AtomicU32, Ordering};

use super::d3d12;

#[derive(Debug)]
pub struct RafxBufferRaw {
    pub resource: d3d12::ID3D12Resource,
    pub allocation: gpu_allocator::d3d12::Allocation,
}

#[derive(Debug)]
pub struct RafxBufferDx12 {
    device_context: RafxDeviceContextDx12,
    buffer_raw: Option<RafxBufferRaw>,

    buffer_def: RafxBufferDef,
    cbv: Option<Dx12DescriptorId>,
    srv: Option<Dx12DescriptorId>,
    uav: Option<Dx12DescriptorId>,
    mapped_ptr: TrustCell<Option<*mut u8>>,
    mapped_ref_count: AtomicU32,

    gpu_address: u64,
}

// for TrustCell<Option<*mut u8>>, which is a pointer to mapped buffer
unsafe impl Send for RafxBufferDx12 {}
unsafe impl Sync for RafxBufferDx12 {}

impl RafxBufferDx12 {
    pub fn cbv(&self) -> Option<Dx12DescriptorId> {
        self.cbv
    }

    pub fn srv(&self) -> Option<Dx12DescriptorId> {
        self.srv
    }

    pub fn uav(&self) -> Option<Dx12DescriptorId> {
        self.uav
    }

    pub fn dx12_resource(&self) -> &d3d12::ID3D12Resource {
        &self.buffer_raw.as_ref().unwrap().resource
    }

    pub fn gpu_address(&self) -> u64 {
        self.gpu_address
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
            unsafe {
                let name: &str = name.as_ref();
                let utf16: Vec<_> = name.encode_utf16().chain(std::iter::once(0)).collect();
                self.buffer_raw
                    .as_ref()
                    .unwrap()
                    .resource
                    .SetName(windows::core::PCWSTR::from_raw(utf16.as_ptr()))
                    .unwrap();
                //TODO: Also set on allocation, views, etc?
            }
        }
    }

    pub fn map_buffer(&self) -> RafxResult<*mut u8> {
        if self.buffer_def.memory_usage == RafxMemoryUsage::GpuOnly {
            return Err("Cannot map GPU-only buffer")?;
        }

        if !self.buffer_def.always_mapped {
            let mut mapped_ptr = std::ptr::null_mut::<std::ffi::c_void>();
            unsafe {
                self.buffer_raw
                    .as_ref()
                    .unwrap()
                    .resource
                    .Map(0, None, Some(&mut mapped_ptr))?;
                self.mapped_ref_count.fetch_add(1, Ordering::Relaxed);
            }

            *self.mapped_ptr.borrow_mut() = Some(mapped_ptr as *mut u8);
        }

        Ok(self.mapped_ptr.borrow().unwrap())
    }

    pub fn unmap_buffer(&self) -> RafxResult<()> {
        if self.buffer_def.memory_usage == RafxMemoryUsage::GpuOnly {
            return Err("Cannot map GPU-only buffer")?;
        }

        if !self.buffer_def.always_mapped {
            unsafe {
                self.buffer_raw.as_ref().unwrap().resource.Unmap(0, None);
                let old_count = self.mapped_ref_count.fetch_sub(1, Ordering::Relaxed);
                if old_count == 1 {
                    *self.mapped_ptr.borrow_mut() = None;
                }
            }
        }

        Ok(())
    }

    pub fn mapped_memory(&self) -> Option<*mut u8> {
        *self.mapped_ptr.borrow()
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
        device_context: &RafxDeviceContextDx12,
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
                device_context
                    .device_info()
                    .min_uniform_buffer_offset_alignment as u64,
            )
        }

        let mut desc = d3d12::D3D12_RESOURCE_DESC {
            Dimension: d3d12::D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: d3d12::D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as u64,
            Width: allocation_size,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: super::dxgi::Common::DXGI_FORMAT_UNKNOWN,
            SampleDesc: super::dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: d3d12::D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Flags: d3d12::D3D12_RESOURCE_FLAG_NONE,
        };

        if buffer_def
            .resource_type
            .intersects(RafxResourceType::BUFFER_READ_WRITE)
        {
            desc.Flags |= d3d12::D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
        }

        {
            let mut padded_size = 0;
            unsafe {
                device_context.d3d12_device().GetCopyableFootprints(
                    &desc,
                    0,
                    1,
                    0,
                    None,
                    None,
                    None,
                    Some(&mut padded_size),
                );
            }

            allocation_size = padded_size;
            desc.Width = padded_size;
        }

        let start_state = if buffer_def.queue_type == RafxQueueType::Transfer {
            //DX12 Docs:
            // The COPY flags (COPY_DEST and COPY_SOURCE) used as initial states represent states in
            // the 3D/Compute type class. To use a resource initially on a Copy queue it should
            // start in the COMMON state. The COMMON state can be used for all usages on a Copy
            // queue using the implicit state transitions.
            RafxResourceState::COMMON
        } else {
            match buffer_def.memory_usage {
                RafxMemoryUsage::CpuToGpu => RafxResourceState::GENERIC_READ,
                RafxMemoryUsage::CpuOnly => RafxResourceState::GENERIC_READ,
                RafxMemoryUsage::GpuToCpu => RafxResourceState::UNORDERED_ACCESS,
                RafxMemoryUsage::GpuOnly => RafxResourceState::COPY_DST,
                _ => unreachable!(),
            }
        };

        let res_states = start_state.into();

        let allocation = device_context.allocator().lock().unwrap().allocate(
            &gpu_allocator::d3d12::AllocationCreateDesc {
                name: "",
                location: buffer_def.memory_usage.into(),
                //linear: true,
                //requirements,
                size: allocation_size,
                alignment: d3d12::D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as u64,
                resource_category: gpu_allocator::d3d12::ResourceCategory::Buffer,
            },
        )?;

        let mut resource: Option<d3d12::ID3D12Resource> = None;
        unsafe {
            device_context.d3d12_device().CreatePlacedResource(
                allocation.heap(),
                allocation.offset(),
                &desc,
                res_states,
                None,
                &mut resource,
            )?;
        }
        let resource = resource.unwrap();

        let mut mapped_ptr = None;
        let mapped_ref_count = AtomicU32::new(0);
        if buffer_def.memory_usage != RafxMemoryUsage::GpuOnly && buffer_def.always_mapped {
            unsafe {
                let mut mapped_ptr_cvoid = std::ptr::null_mut::<std::ffi::c_void>();
                resource.Map(0, None, Some(&mut mapped_ptr_cvoid))?;
                mapped_ptr = Some(mapped_ptr_cvoid as *mut u8);
                mapped_ref_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        let gpu_address = unsafe { resource.GetGPUVirtualAddress() };

        // make CBV
        // We can't pre-allocate a full-buffer view if it is too large for the buffer to be bound
        let cbv = if buffer_def
            .resource_type
            .intersects(RafxResourceType::UNIFORM_BUFFER)
            && allocation_size < 65536
        {
            // make cbv
            let desc = d3d12::D3D12_CONSTANT_BUFFER_VIEW_DESC {
                BufferLocation: gpu_address,
                SizeInBytes: allocation_size as u32,
            };

            let descriptor_id = device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .allocate(device_context.d3d12_device(), 1)?;
            let cpu_handle = device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .id_to_cpu_handle(descriptor_id);
            unsafe {
                device_context
                    .d3d12_device()
                    .CreateConstantBufferView(Some(&desc), cpu_handle);
            }

            Some(descriptor_id)
        } else {
            None
        };

        // make srv
        let srv = if buffer_def
            .resource_type
            .intersects(RafxResourceType::BUFFER)
        {
            //println!("creating srv");
            let mut desc = d3d12::D3D12_SHADER_RESOURCE_VIEW_DESC::default();
            desc.Format = buffer_def.format.into();
            desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_BUFFER;
            desc.Shader4ComponentMapping = d3d12::D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
            desc.Anonymous.Buffer.FirstElement = buffer_def.elements.element_begin_index;
            desc.Anonymous.Buffer.NumElements = buffer_def.elements.element_count as _;
            desc.Anonymous.Buffer.StructureByteStride = buffer_def.elements.element_stride as _;
            desc.Anonymous.Buffer.Flags = d3d12::D3D12_BUFFER_SRV_FLAG_NONE;

            //TODO: RAW buffer support?

            //println!("format: {:?} stride {} count: {}", buffer_def.format, buffer_def.elements.element_stride, buffer_def.elements.element_count);

            // Can't create typed structured buffer,
            // see https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ns-d3d12-d3d12_buffer_srv
            if desc.Format != super::dxgi::Common::DXGI_FORMAT_UNKNOWN {
                desc.Anonymous.Buffer.StructureByteStride = 0;
            }

            if desc.Format == super::dxgi::Common::DXGI_FORMAT_UNKNOWN
                && buffer_def.elements.element_stride == 0
            {
                desc.Anonymous.Buffer.StructureByteStride = 4;
                desc.Anonymous.Buffer.NumElements = buffer_def.size as u32 / 4;
            }

            //assert!(buffer_def.elements.)

            let descriptor_id = device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .allocate(device_context.d3d12_device(), 1)?;
            let cpu_handle = device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .id_to_cpu_handle(descriptor_id);
            unsafe {
                device_context.d3d12_device().CreateShaderResourceView(
                    &resource,
                    Some(&desc),
                    cpu_handle,
                );
            }

            Some(descriptor_id)
        } else {
            None
        };

        // make uav
        let uav = if buffer_def
            .resource_type
            .intersects(RafxResourceType::BUFFER_READ_WRITE)
        {
            //println!("creating uav");
            let mut desc = d3d12::D3D12_UNORDERED_ACCESS_VIEW_DESC::default();
            desc.Format = buffer_def.format.into();
            desc.ViewDimension = d3d12::D3D12_UAV_DIMENSION_BUFFER;
            desc.Anonymous.Buffer.FirstElement = buffer_def.elements.element_begin_index;
            desc.Anonymous.Buffer.NumElements = buffer_def.elements.element_count as _;
            desc.Anonymous.Buffer.StructureByteStride = buffer_def.elements.element_stride as _;
            desc.Anonymous.Buffer.CounterOffsetInBytes = 0;
            desc.Anonymous.Buffer.Flags = d3d12::D3D12_BUFFER_UAV_FLAG_NONE;

            //TODO: RAW buffer support?
            //TODO: Validate format support?

            // Can't create typed structured buffer,
            // see https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ns-d3d12-d3d12_buffer_srv
            if desc.Format != super::dxgi::Common::DXGI_FORMAT_UNKNOWN {
                desc.Anonymous.Buffer.StructureByteStride = 0;
            }

            if desc.Format == super::dxgi::Common::DXGI_FORMAT_UNKNOWN
                && buffer_def.elements.element_stride == 0
            {
                desc.Anonymous.Buffer.StructureByteStride = 4;
                desc.Anonymous.Buffer.NumElements = buffer_def.size as u32 / 4;
            }

            //TODO: counter buffer support?
            let descriptor_id = device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .allocate(device_context.d3d12_device(), 1)?;
            let cpu_handle = device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .id_to_cpu_handle(descriptor_id);
            unsafe {
                device_context.d3d12_device().CreateUnorderedAccessView(
                    &resource,
                    None,
                    Some(&desc),
                    cpu_handle,
                );
            }

            Some(descriptor_id)
        } else {
            None
        };

        let buffer_raw = RafxBufferRaw {
            resource,
            allocation,
        };

        log::trace!(
            "Buffer {:?} crated with size {} (always mapped: {:?})",
            buffer_raw.resource,
            buffer_def.size,
            buffer_def.always_mapped
        );

        Ok(RafxBufferDx12 {
            device_context: device_context.clone(),
            buffer_raw: Some(buffer_raw),
            buffer_def: buffer_def.clone(),
            cbv,
            srv,
            uav,
            mapped_ptr: TrustCell::new(mapped_ptr),
            mapped_ref_count,
            gpu_address,
        })
    }
}

impl Drop for RafxBufferDx12 {
    fn drop(&mut self) {
        log::trace!("destroying RafxBufferDx12Inner");
        if let Some(cbv) = self.cbv {
            self.device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .free(cbv, 1);
        }

        if let Some(srv) = self.srv {
            self.device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .free(srv, 1);
        }

        if let Some(uav) = self.uav {
            self.device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .free(uav, 1);
        }

        if let Some(buffer_raw) = self.buffer_raw.take() {
            log::trace!(
                "Buffer {:?} destroying with size {} (always mapped: {:?})",
                buffer_raw.resource,
                self.buffer_def.size,
                self.buffer_def.always_mapped
            );

            drop(buffer_raw.resource);

            self.device_context
                .allocator()
                .lock()
                .unwrap()
                .free(buffer_raw.allocation)
                .unwrap();
        }

        log::trace!("destroyed RafxBufferDx12Inner");
    }
}
