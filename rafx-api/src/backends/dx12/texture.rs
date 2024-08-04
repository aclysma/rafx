use crate::dx12::descriptor_heap::Dx12DescriptorId;
use crate::dx12::RafxDeviceContextDx12;
use crate::{
    RafxResourceState, RafxResourceType, RafxResult, RafxTextureDef, RafxTextureDimensions,
};
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

use super::d3d12;
use super::dxgi;

fn create_srv_desc(
    desc: &d3d12::D3D12_RESOURCE_DESC,
    format: DXGI_FORMAT,
    is_cube_map: bool,
) -> d3d12::D3D12_SHADER_RESOURCE_VIEW_DESC {
    //TODO: Convert format util_to_dx12_srv_format
    let mut srv_desc = d3d12::D3D12_SHADER_RESOURCE_VIEW_DESC::default();
    srv_desc.Format = super::internal::conversions::dxgi_to_srv_format(format);
    srv_desc.Shader4ComponentMapping = d3d12::D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;

    match desc.Dimension {
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE1D => {
            if desc.DepthOrArraySize > 1 {
                srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE1DARRAY;
                srv_desc.Anonymous.Texture1DArray.ArraySize = desc.DepthOrArraySize as _;
                srv_desc.Anonymous.Texture1DArray.FirstArraySlice = 0;
                srv_desc.Anonymous.Texture1DArray.MipLevels = desc.MipLevels as _;
                srv_desc.Anonymous.Texture1DArray.MostDetailedMip = 0;
                srv_desc.Anonymous.Texture1DArray.ResourceMinLODClamp = 0.0;
            } else {
                srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE1D;
                srv_desc.Anonymous.Texture1D.MipLevels = desc.MipLevels as _;
                srv_desc.Anonymous.Texture1D.MostDetailedMip = 0;
                srv_desc.Anonymous.Texture1D.ResourceMinLODClamp = 0.0;
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE2D => {
            if is_cube_map {
                assert_eq!(desc.DepthOrArraySize % 6, 0);

                if desc.DepthOrArraySize > 6 {
                    srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURECUBEARRAY;
                    srv_desc.Anonymous.TextureCubeArray.First2DArrayFace = 0;
                    srv_desc.Anonymous.TextureCubeArray.MipLevels = desc.MipLevels as _;
                    srv_desc.Anonymous.TextureCubeArray.MostDetailedMip = 0;
                    srv_desc.Anonymous.TextureCubeArray.ResourceMinLODClamp = 0.0;
                    srv_desc.Anonymous.TextureCubeArray.NumCubes = desc.DepthOrArraySize as u32 / 6;
                } else {
                    srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURECUBE;
                    srv_desc.Anonymous.TextureCube.MipLevels = desc.MipLevels as _;
                    srv_desc.Anonymous.TextureCube.MostDetailedMip = 0;
                    srv_desc.Anonymous.TextureCube.ResourceMinLODClamp = 0.0;
                }
            } else {
                if desc.DepthOrArraySize > 1 {
                    if desc.SampleDesc.Count > 1 {
                        srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2DMSARRAY;
                        srv_desc.Anonymous.Texture2DMSArray.ArraySize = desc.DepthOrArraySize as _;
                        srv_desc.Anonymous.Texture2DMSArray.FirstArraySlice = 0;
                    } else {
                        srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2DARRAY;
                        srv_desc.Anonymous.Texture2DArray.ArraySize = desc.DepthOrArraySize as _;
                        srv_desc.Anonymous.Texture2DArray.FirstArraySlice = 0;
                        srv_desc.Anonymous.Texture2DArray.MipLevels = desc.MipLevels as _;
                        srv_desc.Anonymous.Texture2DArray.MostDetailedMip = 0;
                        srv_desc.Anonymous.Texture2DArray.ResourceMinLODClamp = 0.0;
                        srv_desc.Anonymous.Texture2DArray.PlaneSlice = 0;
                    }
                } else {
                    if desc.SampleDesc.Count > 1 {
                        srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2DMS;
                        // Nothing to set
                    } else {
                        srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2D;
                        srv_desc.Anonymous.Texture2D.MipLevels = desc.MipLevels as _;
                        srv_desc.Anonymous.Texture2D.MostDetailedMip = 0;
                        srv_desc.Anonymous.Texture2D.ResourceMinLODClamp = 0.0;
                        srv_desc.Anonymous.Texture2D.PlaneSlice = 0;
                    }
                }
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE3D => {
            srv_desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE3D;
            srv_desc.Anonymous.Texture3D.MipLevels = desc.MipLevels as _;
            srv_desc.Anonymous.Texture3D.MostDetailedMip = 0;
            srv_desc.Anonymous.Texture3D.ResourceMinLODClamp = 0.0;
        }
        _ => panic!("Unexpected dimension in add_srv()"),
    }

    srv_desc
}

fn add_srv(
    context: &RafxDeviceContextDx12,
    resource: &d3d12::ID3D12Resource,
    srv_desc: &d3d12::D3D12_SHADER_RESOURCE_VIEW_DESC,
    descriptor_id: Dx12DescriptorId,
) -> RafxResult<Dx12DescriptorId> {
    // let descriptor_id = if let Some(descriptor_id) = descriptor_id {
    //     descriptor_id
    // } else {
    //     context.inner.heaps.cbv_srv_uav_heap.allocate(context.d3d12_device(), 1)?
    // };

    let descriptor_handle = context
        .inner
        .heaps
        .cbv_srv_uav_heap
        .id_to_cpu_handle(descriptor_id);
    unsafe {
        context.d3d12_device().CreateShaderResourceView(
            resource,
            Some(srv_desc),
            descriptor_handle,
        );
    }

    Ok(descriptor_id)
}

fn create_uav_desc(
    desc: &d3d12::D3D12_RESOURCE_DESC,
    format: DXGI_FORMAT,
    is_cube_map: bool,
) -> d3d12::D3D12_UNORDERED_ACCESS_VIEW_DESC {
    let mut uav_desc = d3d12::D3D12_UNORDERED_ACCESS_VIEW_DESC::default();
    uav_desc.Format = super::internal::conversions::dxgi_to_uav_format(format);

    match desc.Dimension {
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE1D => {
            if desc.DepthOrArraySize > 1 {
                uav_desc.ViewDimension = d3d12::D3D12_UAV_DIMENSION_TEXTURE1D;
                uav_desc.Anonymous.Texture1DArray.ArraySize = desc.DepthOrArraySize as _;
                uav_desc.Anonymous.Texture1DArray.FirstArraySlice = 0;
                uav_desc.Anonymous.Texture1DArray.MipSlice = 0;
            } else {
                uav_desc.ViewDimension = d3d12::D3D12_UAV_DIMENSION_TEXTURE1D;
                uav_desc.Anonymous.Texture1D.MipSlice = 0;
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE2D => {
            if is_cube_map {
                assert_eq!(desc.DepthOrArraySize % 6, 0);

                uav_desc.ViewDimension = d3d12::D3D12_UAV_DIMENSION_TEXTURE2DARRAY;
                uav_desc.Anonymous.Texture2DArray.ArraySize = desc.DepthOrArraySize as _;
                uav_desc.Anonymous.Texture2DArray.FirstArraySlice = 0;
                uav_desc.Anonymous.Texture2DArray.MipSlice = 0;
                uav_desc.Anonymous.Texture2DArray.PlaneSlice = 0;
            } else {
                if desc.SampleDesc.Count > 1 {
                    // can't create multisampled uav
                    unimplemented!()
                }

                if desc.DepthOrArraySize > 1 {
                    uav_desc.ViewDimension = d3d12::D3D12_UAV_DIMENSION_TEXTURE2DARRAY;
                    uav_desc.Anonymous.Texture2DArray.ArraySize = desc.DepthOrArraySize as _;
                    uav_desc.Anonymous.Texture2DArray.FirstArraySlice = 0;
                    uav_desc.Anonymous.Texture2DArray.MipSlice = 0;
                    uav_desc.Anonymous.Texture2DArray.PlaneSlice = 0;
                } else {
                    uav_desc.ViewDimension = d3d12::D3D12_UAV_DIMENSION_TEXTURE2D;
                    uav_desc.Anonymous.Texture2D.MipSlice = 0;
                    uav_desc.Anonymous.Texture2D.PlaneSlice = 0;
                }
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE3D => {
            uav_desc.ViewDimension = d3d12::D3D12_UAV_DIMENSION_TEXTURE3D;
            uav_desc.Anonymous.Texture3D.MipSlice = 0;
            uav_desc.Anonymous.Texture3D.FirstWSlice = 0;
            uav_desc.Anonymous.Texture3D.WSize = desc.DepthOrArraySize as _;
        }
        _ => panic!("Unexpected dimension in create_uav_desc()"),
    }

    uav_desc
}

fn add_uav(
    context: &RafxDeviceContextDx12,
    resource: &d3d12::ID3D12Resource,
    uav_desc: &d3d12::D3D12_UNORDERED_ACCESS_VIEW_DESC,
    descriptor_id: Dx12DescriptorId,
) -> RafxResult<Dx12DescriptorId> {
    let descriptor_handle = context
        .inner
        .heaps
        .cbv_srv_uav_heap
        .id_to_cpu_handle(descriptor_id);
    unsafe {
        context.d3d12_device().CreateUnorderedAccessView(
            resource,
            None,
            Some(uav_desc),
            descriptor_handle,
        );
    }

    Ok(descriptor_id)
}

fn add_rtv(
    context: &RafxDeviceContextDx12,
    resource: &d3d12::ID3D12Resource,
    desc: &d3d12::D3D12_RESOURCE_DESC,
    format: DXGI_FORMAT,
    mip_level: u32,
    array_slice: Option<u32>,
    descriptor_id: Dx12DescriptorId,
) -> RafxResult<Dx12DescriptorId> {
    let mut rtv_desc = d3d12::D3D12_RENDER_TARGET_VIEW_DESC::default();
    rtv_desc.Format = format;

    match desc.Dimension {
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE1D => {
            if desc.DepthOrArraySize > 1 {
                rtv_desc.ViewDimension = d3d12::D3D12_RTV_DIMENSION_TEXTURE1DARRAY;
                rtv_desc.Anonymous.Texture1DArray.MipSlice = mip_level;
                if let Some(array_slice) = array_slice {
                    rtv_desc.Anonymous.Texture1DArray.ArraySize = 1;
                    rtv_desc.Anonymous.Texture1DArray.FirstArraySlice = array_slice as u32;
                } else {
                    rtv_desc.Anonymous.Texture1DArray.ArraySize = desc.DepthOrArraySize as u32;
                }
            } else {
                rtv_desc.ViewDimension = d3d12::D3D12_RTV_DIMENSION_TEXTURE1D;
                rtv_desc.Anonymous.Texture1D.MipSlice = mip_level;
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE2D => {
            if desc.SampleDesc.Count > 1 {
                if desc.DepthOrArraySize > 1 {
                    rtv_desc.ViewDimension = d3d12::D3D12_RTV_DIMENSION_TEXTURE2DMSARRAY;
                    if let Some(array_slice) = array_slice {
                        rtv_desc.Anonymous.Texture2DMSArray.ArraySize = 1;
                        rtv_desc.Anonymous.Texture2DMSArray.FirstArraySlice = array_slice as u32;
                    } else {
                        rtv_desc.Anonymous.Texture2DMSArray.ArraySize =
                            desc.DepthOrArraySize as u32;
                    }
                } else {
                    rtv_desc.ViewDimension = d3d12::D3D12_RTV_DIMENSION_TEXTURE2DMS;
                }
            } else {
                if desc.DepthOrArraySize > 1 {
                    rtv_desc.ViewDimension = d3d12::D3D12_RTV_DIMENSION_TEXTURE2DARRAY;
                    rtv_desc.Anonymous.Texture2DArray.MipSlice = mip_level;
                    if let Some(array_slice) = array_slice {
                        rtv_desc.Anonymous.Texture2DArray.ArraySize = 1;
                        rtv_desc.Anonymous.Texture2DArray.FirstArraySlice = array_slice as u32;
                    } else {
                        rtv_desc.Anonymous.Texture2DArray.ArraySize = desc.DepthOrArraySize as u32;
                    }
                } else {
                    rtv_desc.ViewDimension = d3d12::D3D12_RTV_DIMENSION_TEXTURE2D;
                    rtv_desc.Anonymous.Texture2D.MipSlice = mip_level;
                }
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE3D => {
            rtv_desc.ViewDimension = d3d12::D3D12_RTV_DIMENSION_TEXTURE3D;
            rtv_desc.Anonymous.Texture3D.MipSlice = mip_level;
            if let Some(array_slice) = array_slice {
                rtv_desc.Anonymous.Texture3D.WSize = 1;
                rtv_desc.Anonymous.Texture3D.FirstWSlice = array_slice as u32;
            } else {
                rtv_desc.Anonymous.Texture3D.WSize = desc.DepthOrArraySize as u32;
            }
        }
        _ => panic!("Unexpected dimension in add_rtv()"),
    }
    let descriptor_handle = context.inner.heaps.rtv_heap.id_to_cpu_handle(descriptor_id);
    unsafe {
        context
            .d3d12_device()
            .CreateRenderTargetView(resource, Some(&rtv_desc), descriptor_handle);
    }

    Ok(descriptor_id)
}

fn add_dsv(
    context: &RafxDeviceContextDx12,
    resource: &d3d12::ID3D12Resource,
    desc: &d3d12::D3D12_RESOURCE_DESC,
    format: DXGI_FORMAT,
    mip_level: u32,
    array_slice: Option<u32>,
    descriptor_id: Dx12DescriptorId,
) -> RafxResult<Dx12DescriptorId> {
    let mut dsv_desc = d3d12::D3D12_DEPTH_STENCIL_VIEW_DESC::default();
    dsv_desc.Format = format;

    match desc.Dimension {
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE1D => {
            if desc.DepthOrArraySize > 1 {
                dsv_desc.ViewDimension = d3d12::D3D12_DSV_DIMENSION_TEXTURE1DARRAY;
                dsv_desc.Anonymous.Texture1DArray.MipSlice = mip_level;

                if let Some(array_slice) = array_slice {
                    dsv_desc.Anonymous.Texture1DArray.ArraySize = 1;
                    dsv_desc.Anonymous.Texture1DArray.FirstArraySlice = array_slice as _;
                } else {
                    dsv_desc.Anonymous.Texture1DArray.ArraySize = desc.DepthOrArraySize as _;
                    dsv_desc.Anonymous.Texture1DArray.FirstArraySlice = 0;
                }
            } else {
                dsv_desc.ViewDimension = d3d12::D3D12_DSV_DIMENSION_TEXTURE1D;
                dsv_desc.Anonymous.Texture1D.MipSlice = mip_level;
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE2D => {
            if desc.SampleDesc.Count > 1 {
                if desc.DepthOrArraySize > 1 {
                    dsv_desc.ViewDimension = d3d12::D3D12_DSV_DIMENSION_TEXTURE2DMSARRAY;
                    if let Some(array_slice) = array_slice {
                        dsv_desc.Anonymous.Texture2DMSArray.ArraySize = 1;
                        dsv_desc.Anonymous.Texture2DMSArray.FirstArraySlice = array_slice as _;
                    } else {
                        dsv_desc.Anonymous.Texture2DMSArray.ArraySize = desc.DepthOrArraySize as _;
                        dsv_desc.Anonymous.Texture2DMSArray.FirstArraySlice = 0;
                    }
                } else {
                    dsv_desc.ViewDimension = d3d12::D3D12_DSV_DIMENSION_TEXTURE2DMS;
                    // Nothing to set
                }
            } else {
                if desc.DepthOrArraySize > 1 {
                    dsv_desc.ViewDimension = d3d12::D3D12_DSV_DIMENSION_TEXTURE2DARRAY;
                    dsv_desc.Anonymous.Texture2DArray.MipSlice = mip_level;
                    if let Some(array_slice) = array_slice {
                        dsv_desc.Anonymous.Texture2DArray.ArraySize = 1;
                        dsv_desc.Anonymous.Texture2DArray.FirstArraySlice = array_slice as _;
                    } else {
                        dsv_desc.Anonymous.Texture2DArray.ArraySize = desc.DepthOrArraySize as _;
                        dsv_desc.Anonymous.Texture2DArray.FirstArraySlice = 0;
                    }
                } else {
                    dsv_desc.ViewDimension = d3d12::D3D12_DSV_DIMENSION_TEXTURE2D;
                    dsv_desc.Anonymous.Texture2D.MipSlice = mip_level;
                }
            }
        }
        d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE3D => {
            // Not supported
            panic!("3d depth stencil not supported")
        }
        _ => panic!("Unexpected dimension in add_dsv()"),
    }

    let descriptor_handle = context.inner.heaps.dsv_heap.id_to_cpu_handle(descriptor_id);
    unsafe {
        context
            .d3d12_device()
            .CreateDepthStencilView(resource, Some(&dsv_desc), descriptor_handle);
    }

    Ok(descriptor_id)
}

fn mip_level_array_index_to_slice_index(
    texture_def: &RafxTextureDef,
    mip_level: u32,
    array_slice: u32,
) -> u32 {
    if texture_def
        .resource_type
        .intersects(RafxResourceType::RENDER_TARGET_ARRAY_SLICES)
        || texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_DEPTH_SLICES)
    {
        debug_assert!(mip_level < texture_def.mip_count);
        debug_assert!(array_slice < texture_def.array_length);
        texture_def.array_length * mip_level + array_slice
    } else {
        debug_assert!(mip_level < texture_def.mip_count);
        debug_assert_eq!(array_slice, 0);
        mip_level
    }
}

#[derive(Debug)]
pub struct RafxRawImageDx12 {
    pub image: super::d3d12::ID3D12Resource,
    pub allocation: Option<gpu_allocator::d3d12::Allocation>,
}

impl RafxRawImageDx12 {
    fn destroy_image(
        &mut self,
        device_context: &RafxDeviceContextDx12,
    ) {
        if let Some(allocation) = self.allocation.take() {
            log::trace!("destroying RafxRawImageDx12");

            device_context
                .allocator()
                .lock()
                .unwrap()
                .free(allocation)
                .unwrap();

            log::trace!("destroyed RafxRawImageDx12");
        } else {
            log::trace!(
                "RafxImageVulkan has no allocation associated with it, not destroying image"
            );
        }
    }
}

impl Drop for RafxRawImageDx12 {
    fn drop(&mut self) {
        assert!(self.allocation.is_none())
    }
}

#[derive(Debug)]
pub struct RafxTextureDx12Inner {
    device_context: RafxDeviceContextDx12,
    texture_def: RafxTextureDef,
    image: RafxRawImageDx12,
    //mip_level_uav_views: Vec<metal_rs::Texture>,
    texture_id: u32,

    srv_uav_handles: Option<Dx12DescriptorId>,
    rtv_handles: Option<Dx12DescriptorId>,
    dsv_handles: Option<Dx12DescriptorId>,

    srv_uav_handle_count: u32,
    rtv_handle_count: u32,
    dsv_handle_count: u32,

    //NOTE: If we care about saving memory, we can derive index from handles above using texture_def

    // One per texture if RafxResourceType::TEXTURE is set
    srv: Option<Dx12DescriptorId>,
    // One per mip level if RafxResourceType::TEXTURE_READ_WRITE is set
    first_uav: Option<Dx12DescriptorId>,
    // 1 + mip_count if RafxResourceType::RENDER_TARGET_COLOR is set
    // Unless RENDER_TARGET_ARRAY_SLICES is also set, in which case 1 + mip_count * array_length
    rtv: Option<Dx12DescriptorId>,
    first_rtv_slice: Option<Dx12DescriptorId>,
    // 1 + mip_count if RafxResourceType::RENDER_TARGET_DEPTH_STENCIL is set
    // Unless RENDER_TARGET_DEPTH_SLICES is also set, in which case 1 + mip_count * array_length
    dsv: Option<Dx12DescriptorId>,
    first_dsv_slice: Option<Dx12DescriptorId>,
}

impl Drop for RafxTextureDx12Inner {
    fn drop(&mut self) {
        if let Some(srv_uav_handles) = self.srv_uav_handles {
            self.device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .free(srv_uav_handles, self.srv_uav_handle_count);
        }

        if let Some(rtv_handles) = self.rtv_handles {
            self.device_context
                .inner
                .heaps
                .rtv_heap
                .free(rtv_handles, self.rtv_handle_count);
        }

        if let Some(dsv_handles) = self.dsv_handles {
            self.device_context
                .inner
                .heaps
                .dsv_heap
                .free(dsv_handles, self.dsv_handle_count);
        }

        self.image.destroy_image(&self.device_context);
    }
}

/// Holds the vk::Image and allocation as well as a few vk::ImageViews depending on the
/// provided RafxResourceType in the texture_def.
#[derive(Clone, Debug)]
pub struct RafxTextureDx12 {
    inner: Arc<RafxTextureDx12Inner>,
}

// for metal_rs::Texture
unsafe impl Send for RafxTextureDx12 {}
unsafe impl Sync for RafxTextureDx12 {}

impl PartialEq for RafxTextureDx12 {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.inner.texture_id == other.inner.texture_id
    }
}

impl Eq for RafxTextureDx12 {}

impl Hash for RafxTextureDx12 {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.inner.texture_id.hash(state);
    }
}

impl RafxTextureDx12 {
    pub fn texture_def(&self) -> &RafxTextureDef {
        &self.inner.texture_def
    }

    pub fn dx12_resource(&self) -> &super::d3d12::ID3D12Resource {
        &self.inner.image.image
    }

    pub fn srv(&self) -> Option<Dx12DescriptorId> {
        debug_assert!(self
            .inner
            .texture_def
            .resource_type
            .intersects(RafxResourceType::TEXTURE));
        self.inner.srv
    }

    pub fn srv_handle(&self) -> Option<d3d12::D3D12_CPU_DESCRIPTOR_HANDLE> {
        let handle = self.srv();
        handle.map(|x| {
            self.inner
                .device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .id_to_cpu_handle(x)
        })
    }

    // TODO: Supprt arrays? Right now the handles are created foreach array element, foreach mip.. this is the opposite of RTV
    // MIPMAP CODE IS ASSUMING THIS
    pub fn uav(
        &self,
        mip_level: u32,
    ) -> Option<Dx12DescriptorId> {
        // debug_assert!(self
        //     .inner
        //     .texture_def
        //     .resource_type
        //     .intersects(RafxResourceType::TEXTURE_READ_WRITE));
        debug_assert!(
            mip_level < self.inner.texture_def.array_length * self.inner.texture_def.mip_count
        );
        self.inner.first_uav.map(|x| x.add_offset(mip_level))
    }

    pub fn uav_handle(
        &self,
        mip_level: u32,
    ) -> Option<d3d12::D3D12_CPU_DESCRIPTOR_HANDLE> {
        let handle = self.uav(mip_level);
        handle.map(|x| {
            self.inner
                .device_context
                .inner
                .heaps
                .cbv_srv_uav_heap
                .id_to_cpu_handle(x)
        })
    }

    pub fn rtv(&self) -> Option<Dx12DescriptorId> {
        debug_assert!(self
            .inner
            .texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_COLOR));
        self.inner.rtv
    }

    pub fn rtv_handle(&self) -> Option<d3d12::D3D12_CPU_DESCRIPTOR_HANDLE> {
        let handle = self.rtv();
        handle.map(|x| {
            self.inner
                .device_context
                .inner
                .heaps
                .rtv_heap
                .id_to_cpu_handle(x)
        })
    }

    pub fn dsv(&self) -> Option<Dx12DescriptorId> {
        debug_assert!(self
            .inner
            .texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL));
        self.inner.dsv
    }

    pub fn dsv_handle(&self) -> Option<d3d12::D3D12_CPU_DESCRIPTOR_HANDLE> {
        let handle = self.dsv();
        handle.map(|x| {
            self.inner
                .device_context
                .inner
                .heaps
                .dsv_heap
                .id_to_cpu_handle(x)
        })
    }

    pub fn rtv_slice(
        &self,
        mip_level: u32,
        array_slice: u32,
    ) -> Option<Dx12DescriptorId> {
        self.inner.first_rtv_slice.map(|x| {
            x.add_offset(mip_level_array_index_to_slice_index(
                &self.inner.texture_def,
                mip_level,
                array_slice,
            ))
        })
    }

    pub fn rtv_slice_handle(
        &self,
        mip_level: u32,
        array_slice: u32,
    ) -> Option<d3d12::D3D12_CPU_DESCRIPTOR_HANDLE> {
        let handle = self.rtv_slice(mip_level, array_slice);
        handle.map(|x| {
            self.inner
                .device_context
                .inner
                .heaps
                .dsv_heap
                .id_to_cpu_handle(x)
        })
    }

    pub fn dsv_slice(
        &self,
        mip_level: u32,
        array_slice: u32,
    ) -> Option<Dx12DescriptorId> {
        self.inner.first_dsv_slice.map(|x| {
            x.add_offset(mip_level_array_index_to_slice_index(
                &self.inner.texture_def,
                mip_level,
                array_slice,
            ))
        })
    }

    pub fn dsv_slice_handle(
        &self,
        mip_level: u32,
        array_slice: u32,
    ) -> Option<d3d12::D3D12_CPU_DESCRIPTOR_HANDLE> {
        let handle = self.dsv_slice(mip_level, array_slice);
        handle.map(|x| {
            self.inner
                .device_context
                .inner
                .heaps
                .dsv_heap
                .id_to_cpu_handle(x)
        })
    }

    pub fn set_debug_name(
        &self,
        name: impl AsRef<str>,
    ) {
        if self.inner.device_context.device_info().debug_names_enabled {
            unsafe {
                let name: &str = name.as_ref();
                let utf16: Vec<_> = name.encode_utf16().chain(std::iter::once(0)).collect();
                self.inner
                    .image
                    .image
                    .SetName(windows::core::PCWSTR::from_raw(utf16.as_ptr()))
                    .unwrap();
                //TODO: Also set on allocation, views, etc?
            }
        }
    }

    pub fn new(
        device_context: &RafxDeviceContextDx12,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureDx12> {
        Self::from_existing(device_context, None, texture_def)
    }

    // This path is mostly so we can wrap a provided swapchain image
    pub fn from_existing(
        device_context: &RafxDeviceContextDx12,
        existing_image: Option<RafxRawImageDx12>,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureDx12> {
        texture_def.verify();

        let is_depth = texture_def.format.has_depth();

        let dimensions = texture_def
            .dimensions
            .determine_dimensions(texture_def.extents);

        let create_uav_chain = texture_def
            .resource_type
            .intersects(RafxResourceType::TEXTURE_READ_WRITE)
            || (texture_def.mip_count > 1 && !texture_def.format.is_compressed());

        //
        // Create the resource if it wasn't provided
        //
        let image = if let Some(existing_image) = existing_image {
            existing_image
        } else {
            let d3d12_dimension = match dimensions {
                RafxTextureDimensions::Dim1D => d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE1D,
                RafxTextureDimensions::Dim2D => d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                RafxTextureDimensions::Dim3D => d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE3D,
                _ => unreachable!(),
            };

            let dxgi_format = texture_def.format.into();
            let typeless_format = super::internal::conversions::dxgi_to_typeless(dxgi_format);

            let (extents_width, extents_height) = if texture_def.format.is_compressed() {
                let w = rafx_base::memory::round_size_up_to_alignment_u32(
                    texture_def.extents.width,
                    texture_def.format.block_width_in_pixels(),
                );
                let h = rafx_base::memory::round_size_up_to_alignment_u32(
                    texture_def.extents.height,
                    texture_def.format.block_height_in_pixels(),
                );
                (w, h)
            } else {
                (texture_def.extents.width, texture_def.extents.height)
            };

            let mut desc = d3d12::D3D12_RESOURCE_DESC {
                Dimension: d3d12_dimension,
                // From docs: If Alignment is set to 0, the runtime will use 4MB for MSAA textures and 64KB for everything else.
                Alignment: 0,
                Width: extents_width as u64,
                Height: extents_height,
                DepthOrArraySize: if texture_def.array_length != 1 {
                    texture_def.array_length
                } else {
                    texture_def.extents.depth
                } as u16,
                MipLevels: texture_def.mip_count as u16,
                Format: typeless_format, //TODO: typeless or not?
                SampleDesc: dxgi::Common::DXGI_SAMPLE_DESC {
                    Count: texture_def.sample_count.as_u32(),
                    Quality: 0,
                },
                Layout: d3d12::D3D12_TEXTURE_LAYOUT_UNKNOWN,
                Flags: d3d12::D3D12_RESOURCE_FLAG_NONE,
            };

            let resource_states = RafxResourceState::UNDEFINED;

            if create_uav_chain {
                desc.Flags |= d3d12::D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
            }

            if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_COLOR)
            {
                desc.Flags |= d3d12::D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
            }

            if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
            {
                desc.Flags |= d3d12::D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL;
            }

            if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_ARRAY_SLICES)
                || texture_def
                    .resource_type
                    .intersects(RafxResourceType::RENDER_TARGET_DEPTH_SLICES)
            {
                if is_depth {
                    desc.Flags |= d3d12::D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL;
                } else {
                    desc.Flags |= d3d12::D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
                }
            }

            let mut resource_category = gpu_allocator::d3d12::ResourceCategory::OtherTexture;
            let mut d3d_clear_value = d3d12::D3D12_CLEAR_VALUE::default();
            let clear_value: Option<*const d3d12::D3D12_CLEAR_VALUE> = if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
            {
                resource_category = gpu_allocator::d3d12::ResourceCategory::RtvDsvTexture;
                d3d_clear_value.Format = dxgi_format;
                d3d_clear_value.Anonymous.DepthStencil.Depth = 0.0;
                d3d_clear_value.Anonymous.DepthStencil.Stencil = 0;
                Some(&d3d_clear_value)
            } else if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_COLOR)
            {
                resource_category = gpu_allocator::d3d12::ResourceCategory::RtvDsvTexture;
                d3d_clear_value.Format = dxgi_format;
                d3d_clear_value.Anonymous.Color = [0.0, 0.0, 0.0, 0.0];
                Some(&d3d_clear_value)
            } else {
                None
            };

            let d3d12_resource_states: d3d12::D3D12_RESOURCE_STATES = resource_states.into();

            let allocation_info = unsafe {
                device_context
                    .d3d12_device()
                    .GetResourceAllocationInfo(0, &[desc])
            };

            let allocation = device_context.allocator().lock().unwrap().allocate(
                &gpu_allocator::d3d12::AllocationCreateDesc {
                    name: "",
                    location: gpu_allocator::MemoryLocation::GpuOnly,
                    size: allocation_info.SizeInBytes,
                    alignment: allocation_info.Alignment,
                    resource_category,
                },
            )?;

            let mut resource: Option<d3d12::ID3D12Resource> = None;
            unsafe {
                device_context.d3d12_device().CreatePlacedResource(
                    allocation.heap(),
                    allocation.offset(),
                    &desc,
                    d3d12_resource_states,
                    clear_value,
                    &mut resource,
                )?;
            }
            let image = resource.unwrap();

            RafxRawImageDx12 {
                image,
                allocation: Some(allocation),
            }
        };

        let pixel_format = texture_def.format.into();

        //
        // Determine SRV/UAV descriptor handle count
        //
        let mut srv_uav_handle_count = 0;
        let mut uav_first_index = 0;

        // One per texture
        if texture_def
            .resource_type
            .intersects(RafxResourceType::TEXTURE)
        {
            srv_uav_handle_count += 1;
        }

        uav_first_index = srv_uav_handle_count;

        // One per mip count
        if create_uav_chain {
            srv_uav_handle_count += texture_def.mip_count;
        }
        let resource_desc = unsafe { image.image.GetDesc() };

        let is_cube_map = texture_def
            .resource_type
            .contains(RafxResourceType::TEXTURE_CUBE);

        let srv_uav_handles = if srv_uav_handle_count > 0 {
            Some(
                device_context
                    .inner
                    .heaps
                    .cbv_srv_uav_heap
                    .allocate(device_context.d3d12_device(), srv_uav_handle_count)?,
            )
        } else {
            None
        };

        let mut srv = None;
        let mut first_uav = None;
        if let Some(srv_uav_handles) = srv_uav_handles {
            let mut next_srv_uav_handle = srv_uav_handles;
            if texture_def
                .resource_type
                .intersects(RafxResourceType::TEXTURE)
            {
                let srv_desc = create_srv_desc(&resource_desc, pixel_format, is_cube_map);
                add_srv(device_context, &image.image, &srv_desc, next_srv_uav_handle)?;
                srv = Some(next_srv_uav_handle);
                next_srv_uav_handle = next_srv_uav_handle.add_offset(1);
            }

            if create_uav_chain {
                first_uav = Some(next_srv_uav_handle);
                let mut uav_desc = create_uav_desc(&resource_desc, pixel_format, is_cube_map);
                for mip_slice in 0..texture_def.mip_count {
                    // Adjust the uav_desc to target the correct mip slice
                    // WARNING: This is technically unsound, it assumes union members have the first element as MipSlice.
                    uav_desc.Anonymous.Texture1DArray.MipSlice = mip_slice;
                    if resource_desc.Dimension == d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE3D {
                        uav_desc.Anonymous.Texture3D.WSize =
                            resource_desc.DepthOrArraySize as u32 / 2_u32.pow(mip_slice);
                    }

                    add_uav(device_context, &image.image, &uav_desc, next_srv_uav_handle)?;
                    next_srv_uav_handle = next_srv_uav_handle.add_offset(1);
                }
            }

            assert_eq!(
                next_srv_uav_handle.0 - srv_uav_handles.0,
                srv_uav_handle_count
            );
        }

        let mut render_target_count = if texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_ARRAY_SLICES)
            || texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_DEPTH_SLICES)
        {
            texture_def.mip_count * texture_def.array_length
        } else if texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_COLOR)
            || texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
        {
            texture_def.mip_count
        } else {
            0
        };

        if texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_COLOR)
            || texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
        {
            render_target_count += 1;
        }

        let mut rtv_handle_count = 0;
        let mut dsv_handle_count = 0;

        if is_depth {
            dsv_handle_count = render_target_count;
        } else {
            rtv_handle_count = render_target_count;
        }

        let rtv_handles = if rtv_handle_count > 0 {
            Some(
                device_context
                    .inner
                    .heaps
                    .rtv_heap
                    .allocate(device_context.d3d12_device(), rtv_handle_count)?,
            )
        } else {
            None
        };
        let mut next_rtv_handle = rtv_handles;

        let dsv_handles = if dsv_handle_count > 0 {
            Some(
                device_context
                    .inner
                    .heaps
                    .dsv_heap
                    .allocate(device_context.d3d12_device(), dsv_handle_count)?,
            )
        } else {
            None
        };
        let mut next_dsv_handle = dsv_handles;

        let mut rtv = None;
        if texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_COLOR)
        {
            add_rtv(
                device_context,
                &image.image,
                &resource_desc,
                pixel_format,
                0,
                None,
                next_rtv_handle.unwrap(),
            )?;
            rtv = next_rtv_handle;
            next_rtv_handle = Some(next_rtv_handle.unwrap().add_offset(1));
        }

        let mut dsv = None;
        if texture_def
            .resource_type
            .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
        {
            add_dsv(
                device_context,
                &image.image,
                &resource_desc,
                pixel_format,
                0,
                None,
                next_dsv_handle.unwrap(),
            )?;
            dsv = next_dsv_handle;
            next_dsv_handle = Some(next_dsv_handle.unwrap().add_offset(1));
        }

        let first_rtv_slice = next_rtv_handle;
        let first_dsv_slice = next_dsv_handle;

        for mip_level in 0..texture_def.mip_count {
            if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_ARRAY_SLICES)
                || texture_def
                    .resource_type
                    .intersects(RafxResourceType::RENDER_TARGET_DEPTH_SLICES)
            {
                for array_slice in 0..texture_def.array_length {
                    let handle_offset =
                        mip_level_array_index_to_slice_index(texture_def, mip_level, array_slice);
                    if is_depth {
                        add_dsv(
                            device_context,
                            &image.image,
                            &resource_desc,
                            pixel_format,
                            mip_level,
                            Some(array_slice),
                            first_dsv_slice.unwrap().add_offset(handle_offset),
                        )?;
                    } else {
                        add_rtv(
                            device_context,
                            &image.image,
                            &resource_desc,
                            pixel_format,
                            mip_level,
                            Some(array_slice),
                            first_rtv_slice.unwrap().add_offset(handle_offset),
                        )?;
                    };
                }
            } else if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_COLOR)
                || texture_def
                    .resource_type
                    .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
            {
                //TODO: check for mip slice index?
                let handle_offset = mip_level_array_index_to_slice_index(texture_def, mip_level, 0);
                if is_depth {
                    add_dsv(
                        device_context,
                        &image.image,
                        &resource_desc,
                        pixel_format,
                        mip_level,
                        None,
                        first_dsv_slice.unwrap().add_offset(handle_offset),
                    )?;
                } else {
                    add_rtv(
                        device_context,
                        &image.image,
                        &resource_desc,
                        pixel_format,
                        mip_level,
                        None,
                        first_rtv_slice.unwrap().add_offset(handle_offset),
                    )?;
                }
            }
        }

        let texture_id = crate::internal_shared::NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed);

        let inner = RafxTextureDx12Inner {
            texture_def: texture_def.clone(),
            device_context: device_context.clone(),
            image,
            texture_id,
            srv_uav_handles,
            rtv_handles,
            dsv_handles,
            srv_uav_handle_count,
            rtv_handle_count,
            dsv_handle_count,
            srv,
            first_uav,
            rtv,
            first_rtv_slice,
            dsv,
            first_dsv_slice,
        };

        Ok(RafxTextureDx12 {
            inner: Arc::new(inner),
        })
    }
}
