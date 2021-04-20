use crate::{RafxMemoryUsage, RafxCullMode, RafxRasterizerState, RafxFrontFace};
use crate::gl::gles20::types::GLenum;
use crate::gl::gles20;

impl RafxMemoryUsage {
    pub fn gl_usage(self) -> Option<GLenum> {
        match self {
            RafxMemoryUsage::Unknown => None,
            RafxMemoryUsage::GpuOnly => Some(gles20::STATIC_DRAW),
            RafxMemoryUsage::CpuOnly => Some(gles20::NONE),
            RafxMemoryUsage::CpuToGpu => Some(gles20::DYNAMIC_DRAW),
            RafxMemoryUsage::GpuToCpu => Some(gles20::STREAM_DRAW),
        }
    }
}

impl RafxCullMode {
    pub fn gl_cull_mode(self) -> GLenum {
        match self {
            RafxCullMode::None => gles20::NONE,
            RafxCullMode::Back => gles20::BACK,
            RafxCullMode::Front => gles20::FRONT,
        }
    }
}

impl RafxFrontFace {
    pub fn gl_front_face(self) -> GLenum {
        match self {
            RafxFrontFace::CounterClockwise => gles20::CCW,
            RafxFrontFace::Clockwise => gles20::CW,
        }
    }
}

//TODO: Some fields are not handled!
pub struct GlRasterizerState {
    pub cull_mode: gles20::GLenum,
    pub front_face: gles20::GLenum,
    pub scissor_test: bool
}

impl From<&RafxRasterizerState> for GlRasterizerState {
    fn from(rasterizer_state: &RafxRasterizerState) -> Self {
        GlRasterizerState {
            cull_mode: rasterizer_state.cull_mode.gl_cull_mode(),
            front_face: rasterizer_state.front_face.gl_front_face(),
            scissor_test: rasterizer_state.scissor,
        }
    }
}