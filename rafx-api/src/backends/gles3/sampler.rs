use crate::gles3::gles3_bindings;
use crate::gles3::gles3_bindings::types::GLenum;
use crate::gles3::RafxDeviceContextGles3;
use crate::{RafxFilterType, RafxMipMapMode, RafxResult, RafxSamplerDef};
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxSamplerGles3Inner {
    pub(crate) device_context: RafxDeviceContextGles3,
    pub(crate) gl_mip_map_mode: GLenum,
    pub(crate) gl_min_filter: GLenum,
    pub(crate) gl_mag_filter: GLenum,
    pub(crate) gl_address_mode_s: GLenum,
    pub(crate) gl_address_mode_t: GLenum,
    pub(crate) gl_compare_op: GLenum,
}

#[derive(Debug, Clone)]
pub struct RafxSamplerGles3 {
    pub(crate) inner: Arc<RafxSamplerGles3Inner>,
}

impl RafxSamplerGles3 {
    pub fn new(
        device_context: &RafxDeviceContextGles3,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerGles3> {
        let gl_mip_map_mode = match sampler_def.min_filter {
            RafxFilterType::Nearest => match sampler_def.mip_map_mode {
                RafxMipMapMode::Nearest => gles3_bindings::NEAREST_MIPMAP_NEAREST,
                RafxMipMapMode::Linear => gles3_bindings::NEAREST_MIPMAP_LINEAR,
            },
            RafxFilterType::Linear => match sampler_def.mip_map_mode {
                RafxMipMapMode::Nearest => gles3_bindings::LINEAR_MIPMAP_NEAREST,
                RafxMipMapMode::Linear => gles3_bindings::LINEAR_MIPMAP_LINEAR,
            },
        };

        let gl_min_filter = sampler_def.min_filter.gles3_filter_type();
        let gl_mag_filter = sampler_def.mag_filter.gles3_filter_type();

        let gl_address_mode_s =
            sampler_def
                .address_mode_u
                .gles3_address_mode()
                .ok_or_else(|| {
                    format!(
                        "Address mode {:?} not supported in GL ES 2.0",
                        sampler_def.address_mode_u
                    )
                })?;
        let gl_address_mode_t =
            sampler_def
                .address_mode_v
                .gles3_address_mode()
                .ok_or_else(|| {
                    format!(
                        "Address mode {:?} not supported in GL ES 2.0",
                        sampler_def.address_mode_v
                    )
                })?;
        let gl_compare_op = sampler_def.compare_op.gles3_compare_op();

        //TODO: address_mode_w, mip_lod_bias, max_anisotropy, ClampToBorder
        //TODO: sampler objects (ES3 only)

        let inner = RafxSamplerGles3Inner {
            device_context: device_context.clone(),
            gl_mip_map_mode,
            gl_min_filter,
            gl_mag_filter,
            gl_address_mode_s,
            gl_address_mode_t,
            gl_compare_op,
        };

        Ok(RafxSamplerGles3 {
            inner: Arc::new(inner),
        })
    }
}
