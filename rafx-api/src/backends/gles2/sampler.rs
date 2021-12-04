use crate::gles2::gles2_bindings;
use crate::gles2::gles2_bindings::types::GLenum;
use crate::gles2::RafxDeviceContextGles2;
use crate::{RafxCompareOp, RafxFilterType, RafxMipMapMode, RafxResult, RafxSamplerDef};
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxSamplerGles2Inner {
    pub(crate) _device_context: RafxDeviceContextGles2,
    pub(crate) gl_mip_map_mode: GLenum,
    pub(crate) gl_min_filter: GLenum,
    pub(crate) gl_mag_filter: GLenum,
    pub(crate) gl_address_mode_s: GLenum,
    pub(crate) gl_address_mode_t: GLenum,
}

#[derive(Debug, Clone)]
pub struct RafxSamplerGles2 {
    pub(crate) inner: Arc<RafxSamplerGles2Inner>,
}

impl RafxSamplerGles2 {
    pub fn new(
        device_context: &RafxDeviceContextGles2,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerGles2> {
        let gl_mip_map_mode = match sampler_def.min_filter {
            RafxFilterType::Nearest => match sampler_def.mip_map_mode {
                RafxMipMapMode::Nearest => gles2_bindings::NEAREST_MIPMAP_NEAREST,
                RafxMipMapMode::Linear => gles2_bindings::NEAREST_MIPMAP_LINEAR,
            },
            RafxFilterType::Linear => match sampler_def.mip_map_mode {
                RafxMipMapMode::Nearest => gles2_bindings::LINEAR_MIPMAP_NEAREST,
                RafxMipMapMode::Linear => gles2_bindings::LINEAR_MIPMAP_LINEAR,
            },
        };

        let gl_min_filter = sampler_def.min_filter.gles2_filter_type();
        let gl_mag_filter = sampler_def.mag_filter.gles2_filter_type();

        let gl_address_mode_s =
            sampler_def
                .address_mode_u
                .gles2_address_mode()
                .ok_or_else(|| {
                    format!(
                        "Address mode {:?} not supported in GL ES 2.0",
                        sampler_def.address_mode_u
                    )
                })?;
        let gl_address_mode_t =
            sampler_def
                .address_mode_v
                .gles2_address_mode()
                .ok_or_else(|| {
                    format!(
                        "Address mode {:?} not supported in GL ES 2.0",
                        sampler_def.address_mode_v
                    )
                })?;

        if sampler_def.compare_op != RafxCompareOp::Never
            && sampler_def.compare_op != RafxCompareOp::Always
        {
            unimplemented!("GLES 2.0 does not support sampler compare ops");
        }

        //TODO: address_mode_w, mip_lod_bias, max_anisotropy, ClampToBorder
        //TODO: sampler objects (ES3 only)

        let inner = RafxSamplerGles2Inner {
            _device_context: device_context.clone(),
            gl_mip_map_mode,
            gl_min_filter,
            gl_mag_filter,
            gl_address_mode_s,
            gl_address_mode_t,
        };

        Ok(RafxSamplerGles2 {
            inner: Arc::new(inner),
        })
    }
}
