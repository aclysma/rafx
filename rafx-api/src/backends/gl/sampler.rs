use crate::gl::RafxDeviceContextGl;
use crate::{RafxResult, RafxSamplerDef};
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxSamplerGlInner {
    device_context: RafxDeviceContextGl,
    //sampler: gl_rs::SamplerState,
}

#[derive(Debug, Clone)]
pub struct RafxSamplerGl {
    inner: Arc<RafxSamplerGlInner>,
}

impl RafxSamplerGl {
    // pub fn gl_sampler(&self) -> &gl_rs::SamplerStateRef {
    //     self.inner.sampler.as_ref()
    // }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerGl> {
        unimplemented!();
        // let descriptor = gl_rs::SamplerDescriptor::new();
        // descriptor.set_min_filter(sampler_def.min_filter.into());
        // descriptor.set_mag_filter(sampler_def.mag_filter.into());
        // descriptor.set_mip_filter(sampler_def.mip_map_mode.into());
        // if sampler_def.max_anisotropy == 0.0 {
        //     descriptor.set_max_anisotropy(1);
        // } else {
        //     descriptor.set_max_anisotropy(sampler_def.max_anisotropy as _);
        // }
        // let device_info = device_context.device_info();
        // descriptor.set_address_mode_s(super::util::address_mode_mtl_sampler_address_mode(
        //     sampler_def.address_mode_u,
        //     device_info,
        // ));
        // descriptor.set_address_mode_t(super::util::address_mode_mtl_sampler_address_mode(
        //     sampler_def.address_mode_v,
        //     device_info,
        // ));
        // descriptor.set_address_mode_r(super::util::address_mode_mtl_sampler_address_mode(
        //     sampler_def.address_mode_w,
        //     device_info,
        // ));
        // descriptor.set_compare_function(sampler_def.compare_op.into());
        // descriptor.set_support_argument_buffers(true);
        // let sampler = device_context.device().new_sampler(&descriptor);
        //
        // let inner = RafxSamplerGlInner {
        //     device_context: device_context.clone(),
        //     sampler,
        // };
        //
        // Ok(RafxSamplerGl {
        //     inner: Arc::new(inner),
        // })
    }
}
