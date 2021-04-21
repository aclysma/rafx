use crate::{RafxMemoryUsage, RafxCullMode, RafxRasterizerState, RafxFrontFace, RafxDepthState, RafxCompareOp, RafxStencilOp, RafxBlendState, RafxBlendStateRenderTarget, RafxBlendFactor, RafxBlendOp, RafxResult};
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

impl RafxCompareOp {
    pub fn gl_compare_op(self) -> GLenum {
        match self {
            RafxCompareOp::Never => gles20::NEVER,
            RafxCompareOp::Less => gles20::LESS,
            RafxCompareOp::Equal => gles20::EQUAL,
            RafxCompareOp::LessOrEqual => gles20::LEQUAL,
            RafxCompareOp::Greater => gles20::GREATER,
            RafxCompareOp::NotEqual => gles20::NOTEQUAL,
            RafxCompareOp::GreaterOrEqual => gles20::GEQUAL,
            RafxCompareOp::Always => gles20::ALWAYS,
        }
    }
}

impl RafxStencilOp {
    pub fn gl_stencil_op(self) -> GLenum {
        match self {
            RafxStencilOp::Keep => gles20::KEEP,
            RafxStencilOp::Zero => gles20::ZERO,
            RafxStencilOp::Replace => gles20::REPLACE,
            RafxStencilOp::IncrementAndClamp => gles20::INCR,
            RafxStencilOp::DecrementAndClamp => gles20::DECR,
            RafxStencilOp::Invert => gles20::INVERT,
            RafxStencilOp::IncrementAndWrap => gles20::INCR_WRAP,
            RafxStencilOp::DecrementAndWrap => gles20::INCR_WRAP,
        }
    }
}

//TODO: Some fields in RafxRasterizerState are not handled!
#[derive(Debug)]
pub struct GlRasterizerState {
    pub cull_mode: GLenum,
    pub front_face: GLenum,
    pub scissor_test: bool
}

impl From<&RafxRasterizerState> for GlRasterizerState {
    fn from(state: &RafxRasterizerState) -> Self {
        GlRasterizerState {
            cull_mode: state.cull_mode.gl_cull_mode(),
            front_face: state.front_face.gl_front_face(),
            scissor_test: state.scissor,
        }
    }
}

//TODO: Some fields in RafxDepthState are not handled!
#[derive(Debug)]
pub struct GlDepthStencilState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: GLenum,
    pub stencil_test_enable: bool,
    //pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
    pub front_depth_fail_op: GLenum,
    pub front_stencil_compare_op: GLenum,
    pub front_stencil_fail_op: GLenum,
    pub front_stencil_pass_op: GLenum,
    pub back_depth_fail_op: GLenum,
    pub back_stencil_compare_op: GLenum,
    pub back_stencil_fail_op: GLenum,
    pub back_stencil_pass_op: GLenum,
}

impl From<&RafxDepthState> for GlDepthStencilState {
    fn from(state: &RafxDepthState) -> Self {
        GlDepthStencilState {
            depth_test_enable: state.depth_test_enable,
            depth_write_enable: state.depth_write_enable,
            depth_compare_op: state.depth_compare_op.gl_compare_op(),
            stencil_test_enable: state.stencil_test_enable,
            //stencil_read_mask: state.stencil_read_mask,
            stencil_write_mask: state.stencil_write_mask,
            front_depth_fail_op: state.front_depth_fail_op.gl_stencil_op(),
            front_stencil_compare_op: state.front_stencil_compare_op.gl_compare_op(),
            front_stencil_fail_op: state.front_stencil_fail_op.gl_stencil_op(),
            front_stencil_pass_op: state.front_stencil_pass_op.gl_stencil_op(),
            back_depth_fail_op: state.back_depth_fail_op.gl_stencil_op(),
            back_stencil_compare_op: state.back_stencil_compare_op.gl_compare_op(),
            back_stencil_fail_op: state.back_stencil_fail_op.gl_stencil_op(),
            back_stencil_pass_op: state.back_stencil_pass_op.gl_stencil_op(),
        }
    }
}

impl RafxBlendFactor {
    pub fn gl_blend_factor(self) -> GLenum {
        match self {
            RafxBlendFactor::Zero => gles20::ZERO,
            RafxBlendFactor::One => gles20::ONE,
            RafxBlendFactor::SrcColor => gles20::SRC_COLOR,
            RafxBlendFactor::OneMinusSrcColor => gles20::ONE_MINUS_SRC_COLOR,
            RafxBlendFactor::DstColor => gles20::DST_COLOR,
            RafxBlendFactor::OneMinusDstColor => gles20::ONE_MINUS_DST_COLOR,
            RafxBlendFactor::SrcAlpha => gles20::SRC_ALPHA,
            RafxBlendFactor::OneMinusSrcAlpha => gles20::ONE_MINUS_SRC_ALPHA,
            RafxBlendFactor::DstAlpha => gles20::DST_ALPHA,
            RafxBlendFactor::OneMinusDstAlpha => gles20::ONE_MINUS_DST_ALPHA,
            RafxBlendFactor::SrcAlphaSaturate => gles20::SRC_ALPHA_SATURATE,
            RafxBlendFactor::ConstantColor => gles20::CONSTANT_COLOR,
            RafxBlendFactor::OneMinusConstantColor => gles20::ONE_MINUS_CONSTANT_COLOR,
        }
    }
}

impl RafxBlendOp {
    pub fn gl_blend_op(self) -> Option<GLenum> {
        match self {
            RafxBlendOp::Add => Some(gles20::FUNC_ADD),
            RafxBlendOp::Subtract => Some(gles20::FUNC_SUBTRACT),
            RafxBlendOp::ReverseSubtract => Some(gles20::FUNC_REVERSE_SUBTRACT),

            // min/max are GLES 3.2
            RafxBlendOp::Min => None,
            RafxBlendOp::Max => None,
        }
    }
}

// pub struct GlBlendStateRenderTarget {
//     pub enabled: bool,
//     pub src_factor: GLenum,
//     pub dst_factor: GLenum,
//     pub src_factor_alpha: GLenum,
//     pub dst_factor_alpha: GLenum,
//     pub blend_op: GLenum,
//     pub blend_op_alpha: GLenum,
//     //pub masks: RafxColorFlags,
// }
//
// impl From<&RafxBlendStateRenderTarget> for GlBlendStateRenderTarget {
//     fn from(state: &RafxBlendStateRenderTarget) -> Self {
//         GlBlendStateRenderTarget {
//             enabled: state.blend_enabled(),
//             src_factor: state.src_factor.gl_blend_factor(),
//             dst_factor: state.dst_factor.gl_blend_factor(),
//             src_factor_alpha: state.src_factor_alpha.gl_blend_factor(),
//             dst_factor_alpha: state.dst_factor_alpha.gl_blend_factor(),
//             blend_op: state.blend_op.gl_blend_op().ok_or_else(|| format!("GL ES 2.0 does not support blend op {:?}", state.blend_op))?,
//             blend_op_alpha: state.blend_op.gl_blend_op().ok_or_else(|| format!("GL ES 2.0 does not support blend op {:?}", state.blend_op))?,
//         }
//     }
// }

//TODO: Some fields in RafxBlendState are not handled!
#[derive(Debug)]
pub struct GlBlendState {
    pub enabled: bool,
    pub src_factor: GLenum,
    pub dst_factor: GLenum,
    pub src_factor_alpha: GLenum,
    pub dst_factor_alpha: GLenum,
    pub blend_op: GLenum,
    pub blend_op_alpha: GLenum,
}

impl RafxBlendState {
    pub fn gl_blend_state(&self) -> RafxResult<GlBlendState> {
        if self.independent_blend {
            return Err("GL ES 2.0 does not support independent blend states")?;
        }

        let rt_state = self.render_target_blend_states.get(0).ok_or("RafxBlendState has no render target blend states")?;
        let blend_state = GlBlendState {
            enabled: rt_state.blend_enabled(),
            src_factor: rt_state.src_factor.gl_blend_factor(),
            dst_factor: rt_state.dst_factor.gl_blend_factor(),
            src_factor_alpha: rt_state.src_factor_alpha.gl_blend_factor(),
            dst_factor_alpha: rt_state.dst_factor_alpha.gl_blend_factor(),
            blend_op: rt_state.blend_op.gl_blend_op().ok_or_else(|| format!("GL ES 2.0 does not support blend op {:?}", rt_state.blend_op))?,
            blend_op_alpha: rt_state.blend_op.gl_blend_op().ok_or_else(|| format!("GL ES 2.0 does not support blend op {:?}", rt_state.blend_op))?,
        };

        Ok(blend_state)
    }
}
