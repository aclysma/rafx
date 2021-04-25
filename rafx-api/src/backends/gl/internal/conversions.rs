use crate::gl::gles20;
use crate::gl::gles20::types::GLenum;
use crate::{
    RafxAddressMode, RafxBlendFactor, RafxBlendOp, RafxBlendState, RafxCompareOp, RafxCullMode,
    RafxDepthState, RafxFilterType, RafxFrontFace, RafxMemoryUsage, RafxPrimitiveTopology,
    RafxRasterizerState, RafxResult, RafxStencilOp,
};

impl RafxFilterType {
    pub fn gles2_filter_type(self) -> GLenum {
        match self {
            RafxFilterType::Nearest => gles20::NEAREST,
            RafxFilterType::Linear => gles20::LINEAR,
        }
    }
}

impl RafxAddressMode {
    pub fn gles2_address_mode(self) -> Option<GLenum> {
        match self {
            RafxAddressMode::Mirror => Some(gles20::MIRRORED_REPEAT),
            RafxAddressMode::Repeat => Some(gles20::REPEAT),
            RafxAddressMode::ClampToEdge => Some(gles20::CLAMP_TO_EDGE),
            // requires GL_OES_texture_border_clamp
            //RafxAddressMode::ClampToBorder => gles20::CLAMP_TO_BORDER,
            RafxAddressMode::ClampToBorder => None,
        }
    }
}

impl RafxPrimitiveTopology {
    pub fn gles2_topology(self) -> Option<GLenum> {
        match self {
            RafxPrimitiveTopology::PointList => Some(gles20::POINTS),
            RafxPrimitiveTopology::LineList => Some(gles20::LINES),
            RafxPrimitiveTopology::LineStrip => Some(gles20::LINE_STRIP),
            RafxPrimitiveTopology::TriangleList => Some(gles20::TRIANGLES),
            RafxPrimitiveTopology::TriangleStrip => Some(gles20::TRIANGLE_STRIP),
            RafxPrimitiveTopology::PatchList => None,
        }
    }
}

impl RafxMemoryUsage {
    pub fn gles2_usage(self) -> Option<GLenum> {
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
    pub fn gles2_cull_mode(self) -> GLenum {
        match self {
            RafxCullMode::None => gles20::NONE,
            RafxCullMode::Back => gles20::BACK,
            RafxCullMode::Front => gles20::FRONT,
        }
    }
}

impl RafxFrontFace {
    pub fn gles2_front_face(self) -> GLenum {
        match self {
            RafxFrontFace::CounterClockwise => gles20::CCW,
            RafxFrontFace::Clockwise => gles20::CW,
        }
    }
}

impl RafxCompareOp {
    pub fn gles2_compare_op(self) -> GLenum {
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
    pub fn gles2_stencil_op(self) -> GLenum {
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
pub struct Gles2RasterizerState {
    pub cull_mode: GLenum,
    pub front_face: GLenum,
    pub scissor_test: bool,
}

impl From<&RafxRasterizerState> for Gles2RasterizerState {
    fn from(state: &RafxRasterizerState) -> Self {
        Gles2RasterizerState {
            cull_mode: state.cull_mode.gles2_cull_mode(),
            front_face: state.front_face.gles2_front_face(),
            scissor_test: state.scissor,
        }
    }
}

//TODO: Some fields in RafxDepthState are not handled!
#[derive(Debug)]
pub struct Gles2DepthStencilState {
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

impl From<&RafxDepthState> for Gles2DepthStencilState {
    fn from(state: &RafxDepthState) -> Self {
        Gles2DepthStencilState {
            depth_test_enable: state.depth_test_enable,
            depth_write_enable: state.depth_write_enable,
            depth_compare_op: state.depth_compare_op.gles2_compare_op(),
            stencil_test_enable: state.stencil_test_enable,
            //stencil_read_mask: state.stencil_read_mask,
            stencil_write_mask: state.stencil_write_mask,
            front_depth_fail_op: state.front_depth_fail_op.gles2_stencil_op(),
            front_stencil_compare_op: state.front_stencil_compare_op.gles2_compare_op(),
            front_stencil_fail_op: state.front_stencil_fail_op.gles2_stencil_op(),
            front_stencil_pass_op: state.front_stencil_pass_op.gles2_stencil_op(),
            back_depth_fail_op: state.back_depth_fail_op.gles2_stencil_op(),
            back_stencil_compare_op: state.back_stencil_compare_op.gles2_compare_op(),
            back_stencil_fail_op: state.back_stencil_fail_op.gles2_stencil_op(),
            back_stencil_pass_op: state.back_stencil_pass_op.gles2_stencil_op(),
        }
    }
}

impl RafxBlendFactor {
    pub fn gles2_blend_factor(self) -> GLenum {
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
    pub fn gles2_blend_op(self) -> Option<GLenum> {
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

//TODO: Some fields in RafxBlendState are not handled!
#[derive(Debug)]
pub struct Gles2BlendState {
    pub enabled: bool,
    pub src_factor: GLenum,
    pub dst_factor: GLenum,
    pub src_factor_alpha: GLenum,
    pub dst_factor_alpha: GLenum,
    pub blend_op: GLenum,
    pub blend_op_alpha: GLenum,
}

impl RafxBlendState {
    pub fn gles2_blend_state(&self) -> RafxResult<Gles2BlendState> {
        if self.independent_blend {
            return Err("GL ES 2.0 does not support independent blend states")?;
        }

        let rt_state = self
            .render_target_blend_states
            .get(0)
            .ok_or("RafxBlendState has no render target blend states")?;
        let blend_state = Gles2BlendState {
            enabled: rt_state.blend_enabled(),
            src_factor: rt_state.src_factor.gles2_blend_factor(),
            dst_factor: rt_state.dst_factor.gles2_blend_factor(),
            src_factor_alpha: rt_state.src_factor_alpha.gles2_blend_factor(),
            dst_factor_alpha: rt_state.dst_factor_alpha.gles2_blend_factor(),
            blend_op: rt_state.blend_op.gles2_blend_op().ok_or_else(|| {
                format!(
                    "GL ES 2.0 does not support blend op {:?}",
                    rt_state.blend_op
                )
            })?,
            blend_op_alpha: rt_state.blend_op.gles2_blend_op().ok_or_else(|| {
                format!(
                    "GL ES 2.0 does not support blend op {:?}",
                    rt_state.blend_op
                )
            })?,
        };

        Ok(blend_state)
    }
}
