use super::d3d12;
use crate::dx12::{Dx12DescriptorId, RafxDeviceContextDx12};
use crate::{RafxCompareOp, RafxFilterType, RafxMipMapMode, RafxResult, RafxSamplerDef};
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxSamplerDx12Inner {
    device_context: RafxDeviceContextDx12,
    sampler_desc: d3d12::D3D12_SAMPLER_DESC,
    sampler_descriptor: Dx12DescriptorId,
}

impl Drop for RafxSamplerDx12Inner {
    fn drop(&mut self) {
        self.device_context
            .inner
            .heaps
            .sampler_heap
            .free(self.sampler_descriptor, 1);
    }
}

#[derive(Debug, Clone)]
pub struct RafxSamplerDx12 {
    inner: Arc<RafxSamplerDx12Inner>,
}

impl RafxSamplerDx12 {
    pub fn dx12_sampler_descriptor(&self) -> Dx12DescriptorId {
        self.inner.sampler_descriptor
    }

    pub fn dx12_sampler_desc(&self) -> &d3d12::D3D12_SAMPLER_DESC {
        &self.inner.sampler_desc
    }

    pub fn new(
        device_context: &RafxDeviceContextDx12,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerDx12> {
        let mut filter: d3d12::D3D12_FILTER = d3d12::D3D12_FILTER(0);

        // Assume defines from https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ne-d3d12-d3d12_filter
        if sampler_def.max_anisotropy > 0.0 {
            // d3d12 defines assume trilinear if anisotropy is set
            filter.0 |= d3d12::D3D12_FILTER_ANISOTROPIC.0;
        } else {
            if sampler_def.min_filter == RafxFilterType::Linear {
                filter.0 |= 0x1 << 4;
            }

            if sampler_def.mag_filter == RafxFilterType::Linear {
                filter.0 |= 0x1 << 2;
            }

            if sampler_def.mip_map_mode == RafxMipMapMode::Linear {
                filter.0 |= 0x1 << 0;
            }
        }

        if sampler_def.compare_op != RafxCompareOp::Never {
            filter.0 |= 0x1 << 7;
        }

        let max_lod = match sampler_def.mip_map_mode {
            RafxMipMapMode::Nearest => 0.0,
            RafxMipMapMode::Linear => f32::MAX,
        };

        let sampler_desc = d3d12::D3D12_SAMPLER_DESC {
            Filter: filter,
            AddressU: sampler_def.address_mode_u.into(),
            AddressV: sampler_def.address_mode_v.into(),
            AddressW: sampler_def.address_mode_w.into(),
            MipLODBias: sampler_def.mip_lod_bias,
            MaxAnisotropy: (sampler_def.max_anisotropy as u32).max(1),
            ComparisonFunc: sampler_def.compare_op.into(),
            BorderColor: [0.0, 0.0, 0.0, 0.0],
            MinLOD: 0.0,
            MaxLOD: max_lod,
        };

        let sampler_heap = &device_context.inner.heaps.sampler_heap;
        let sampler_descriptor = device_context
            .inner
            .heaps
            .sampler_heap
            .allocate(device_context.d3d12_device(), 1)?;
        unsafe {
            device_context.d3d12_device().CreateSampler(
                &sampler_desc,
                sampler_heap.id_to_cpu_handle(sampler_descriptor),
            )
        };

        let inner = RafxSamplerDx12Inner {
            sampler_descriptor,
            sampler_desc,
            device_context: device_context.clone(),
        };

        Ok(RafxSamplerDx12 {
            inner: Arc::new(inner),
        })
    }
}
