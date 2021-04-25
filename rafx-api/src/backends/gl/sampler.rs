use crate::gl::gles20;
use crate::gl::gles20::types::GLenum;
use crate::gl::RafxDeviceContextGl;
use crate::{RafxFilterType, RafxMipMapMode, RafxResult, RafxSamplerDef};
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxSamplerGlInner {
    pub(crate) device_context: RafxDeviceContextGl,
    pub(crate) gl_mip_map_mode: GLenum,
    pub(crate) gl_min_filter: GLenum,
    pub(crate) gl_mag_filter: GLenum,
    pub(crate) gl_address_mode_s: GLenum,
    pub(crate) gl_address_mode_t: GLenum,
    pub(crate) gl_compare_op: GLenum,
}

#[derive(Debug, Clone)]
pub struct RafxSamplerGl {
    pub(crate) inner: Arc<RafxSamplerGlInner>,
}

impl RafxSamplerGl {
    pub fn new(
        device_context: &RafxDeviceContextGl,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerGl> {
        let gl_mip_map_mode = match sampler_def.min_filter {
            RafxFilterType::Nearest => match sampler_def.mip_map_mode {
                RafxMipMapMode::Nearest => gles20::NEAREST_MIPMAP_NEAREST,
                RafxMipMapMode::Linear => gles20::NEAREST_MIPMAP_LINEAR,
            },
            RafxFilterType::Linear => match sampler_def.mip_map_mode {
                RafxMipMapMode::Nearest => gles20::LINEAR_MIPMAP_NEAREST,
                RafxMipMapMode::Linear => gles20::LINEAR_MIPMAP_LINEAR,
            },
        };

        let gl_min_filter = sampler_def.min_filter.gl_filter_type();
        let gl_mag_filter = sampler_def.mag_filter.gl_filter_type();

        let gl_address_mode_s = sampler_def
            .address_mode_u
            .gl_address_mode()
            .ok_or_else(|| {
                format!(
                    "Address mode {:?} not supported in GL ES 2.0",
                    sampler_def.address_mode_u
                )
            })?;
        let gl_address_mode_t = sampler_def
            .address_mode_v
            .gl_address_mode()
            .ok_or_else(|| {
                format!(
                    "Address mode {:?} not supported in GL ES 2.0",
                    sampler_def.address_mode_v
                )
            })?;
        let gl_compare_op = sampler_def.compare_op.gl_compare_op();

        //TODO: address_mode_w, mip_lod_bias, max_anisotropy, ClampToBorder
        //TODO: sampler objects

        let inner = RafxSamplerGlInner {
            device_context: device_context.clone(),
            gl_mip_map_mode,
            gl_min_filter,
            gl_mag_filter,
            gl_address_mode_s,
            gl_address_mode_t,
            gl_compare_op,
        };

        Ok(RafxSamplerGl {
            inner: Arc::new(inner),
        })
    }
}