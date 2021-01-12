use crate::vulkan::RafxDeviceContextVulkan;
use crate::{RafxCompareOp, RafxMipMapMode, RafxResult, RafxSamplerDef};
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::Arc;

pub struct RafxSamplerVulkanInner {
    device_context: RafxDeviceContextVulkan,
    sampler: vk::Sampler,
}

impl Drop for RafxSamplerVulkanInner {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_sampler(self.sampler, None);
        }
    }
}

#[derive(Clone)]
pub struct RafxSamplerVulkan {
    inner: Arc<RafxSamplerVulkanInner>,
}

impl std::fmt::Debug for RafxSamplerVulkan {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("RafxSamplerVulkan")
            .field("sampler", &self.inner.sampler)
            .finish()
    }
}

impl RafxSamplerVulkan {
    pub fn vk_sampler(&self) -> vk::Sampler {
        self.inner.sampler
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerVulkan> {
        let max_lod = if sampler_def.mip_map_mode == RafxMipMapMode::Linear {
            f32::MAX
        } else {
            0.0
        };

        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(sampler_def.mag_filter.into())
            .min_filter(sampler_def.min_filter.into())
            .mipmap_mode(sampler_def.mip_map_mode.into())
            .address_mode_u(sampler_def.address_mode_u.into())
            .address_mode_v(sampler_def.address_mode_v.into())
            .address_mode_w(sampler_def.address_mode_w.into())
            .mip_lod_bias(sampler_def.mip_lod_bias)
            .anisotropy_enable(sampler_def.max_anisotropy > 0.0)
            .max_anisotropy(sampler_def.max_anisotropy)
            .compare_enable(sampler_def.compare_op != RafxCompareOp::Never)
            .compare_op(sampler_def.compare_op.into())
            .min_lod(sampler_def.mip_lod_bias)
            .max_lod(max_lod)
            .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK)
            .unnormalized_coordinates(false);

        let sampler = unsafe {
            device_context
                .device()
                .create_sampler(&*sampler_create_info, None)?
        };

        let inner = RafxSamplerVulkanInner {
            device_context: device_context.clone(),
            sampler,
        };

        Ok(RafxSamplerVulkan {
            inner: Arc::new(inner),
        })
    }
}
