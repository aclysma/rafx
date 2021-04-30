use crate::gles2::gles2_bindings;
use crate::gles2::gles2_bindings::types::GLenum;
use crate::{
    RafxAddressMode, RafxBlendFactor, RafxBlendOp, RafxBlendState, RafxCompareOp, RafxCullMode,
    RafxDepthState, RafxFilterType, RafxFrontFace, RafxMemoryUsage, RafxPrimitiveTopology,
    RafxRasterizerState, RafxResult, RafxStencilOp,
};

impl RafxFilterType {
    pub fn gles2_filter_type(self) -> GLenum {
        match self {
            RafxFilterType::Nearest => gles2_bindings::NEAREST,
            RafxFilterType::Linear => gles2_bindings::LINEAR,
        }
    }
}

impl RafxAddressMode {
    pub fn gles2_address_mode(self) -> Option<GLenum> {
        match self {
            RafxAddressMode::Mirror => Some(gles2_bindings::MIRRORED_REPEAT),
            RafxAddressMode::Repeat => Some(gles2_bindings::REPEAT),
            RafxAddressMode::ClampToEdge => Some(gles2_bindings::CLAMP_TO_EDGE),
            //TODO: GL ES 3.0 support
            // requires GL_OES_texture_border_clamp
            //RafxAddressMode::ClampToBorder => gles20::CLAMP_TO_BORDER,
            RafxAddressMode::ClampToBorder => None,
        }
    }
}

impl RafxPrimitiveTopology {
    pub fn gles2_topology(self) -> Option<GLenum> {
        match self {
            RafxPrimitiveTopology::PointList => Some(gles2_bindings::POINTS),
            RafxPrimitiveTopology::LineList => Some(gles2_bindings::LINES),
            RafxPrimitiveTopology::LineStrip => Some(gles2_bindings::LINE_STRIP),
            RafxPrimitiveTopology::TriangleList => Some(gles2_bindings::TRIANGLES),
            RafxPrimitiveTopology::TriangleStrip => Some(gles2_bindings::TRIANGLE_STRIP),
            RafxPrimitiveTopology::PatchList => None,
        }
    }
}

impl RafxMemoryUsage {
    pub fn gles2_usage(self) -> Option<GLenum> {
        match self {
            RafxMemoryUsage::Unknown => None,
            RafxMemoryUsage::GpuOnly => Some(gles2_bindings::STATIC_DRAW),
            RafxMemoryUsage::CpuOnly => Some(gles2_bindings::NONE),
            RafxMemoryUsage::CpuToGpu => Some(gles2_bindings::DYNAMIC_DRAW),
            RafxMemoryUsage::GpuToCpu => Some(gles2_bindings::STREAM_DRAW),
        }
    }
}

impl RafxCullMode {
    pub fn gles2_cull_mode(self) -> GLenum {
        match self {
            RafxCullMode::None => gles2_bindings::NONE,
            RafxCullMode::Back => gles2_bindings::BACK,
            RafxCullMode::Front => gles2_bindings::FRONT,
        }
    }
}

impl RafxFrontFace {
    pub fn gles2_front_face(self) -> GLenum {
        match self {
            RafxFrontFace::CounterClockwise => gles2_bindings::CCW,
            RafxFrontFace::Clockwise => gles2_bindings::CW,
        }
    }
}

impl RafxCompareOp {
    pub fn gles2_compare_op(self) -> GLenum {
        match self {
            RafxCompareOp::Never => gles2_bindings::NEVER,
            RafxCompareOp::Less => gles2_bindings::LESS,
            RafxCompareOp::Equal => gles2_bindings::EQUAL,
            RafxCompareOp::LessOrEqual => gles2_bindings::LEQUAL,
            RafxCompareOp::Greater => gles2_bindings::GREATER,
            RafxCompareOp::NotEqual => gles2_bindings::NOTEQUAL,
            RafxCompareOp::GreaterOrEqual => gles2_bindings::GEQUAL,
            RafxCompareOp::Always => gles2_bindings::ALWAYS,
        }
    }
}

impl RafxStencilOp {
    pub fn gles2_stencil_op(self) -> GLenum {
        match self {
            RafxStencilOp::Keep => gles2_bindings::KEEP,
            RafxStencilOp::Zero => gles2_bindings::ZERO,
            RafxStencilOp::Replace => gles2_bindings::REPLACE,
            RafxStencilOp::IncrementAndClamp => gles2_bindings::INCR,
            RafxStencilOp::DecrementAndClamp => gles2_bindings::DECR,
            RafxStencilOp::Invert => gles2_bindings::INVERT,
            RafxStencilOp::IncrementAndWrap => gles2_bindings::INCR_WRAP,
            RafxStencilOp::DecrementAndWrap => gles2_bindings::INCR_WRAP,
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
            RafxBlendFactor::Zero => gles2_bindings::ZERO,
            RafxBlendFactor::One => gles2_bindings::ONE,
            RafxBlendFactor::SrcColor => gles2_bindings::SRC_COLOR,
            RafxBlendFactor::OneMinusSrcColor => gles2_bindings::ONE_MINUS_SRC_COLOR,
            RafxBlendFactor::DstColor => gles2_bindings::DST_COLOR,
            RafxBlendFactor::OneMinusDstColor => gles2_bindings::ONE_MINUS_DST_COLOR,
            RafxBlendFactor::SrcAlpha => gles2_bindings::SRC_ALPHA,
            RafxBlendFactor::OneMinusSrcAlpha => gles2_bindings::ONE_MINUS_SRC_ALPHA,
            RafxBlendFactor::DstAlpha => gles2_bindings::DST_ALPHA,
            RafxBlendFactor::OneMinusDstAlpha => gles2_bindings::ONE_MINUS_DST_ALPHA,
            RafxBlendFactor::SrcAlphaSaturate => gles2_bindings::SRC_ALPHA_SATURATE,
            RafxBlendFactor::ConstantColor => gles2_bindings::CONSTANT_COLOR,
            RafxBlendFactor::OneMinusConstantColor => gles2_bindings::ONE_MINUS_CONSTANT_COLOR,
        }
    }
}

impl RafxBlendOp {
    pub fn gles2_blend_op(self) -> Option<GLenum> {
        match self {
            RafxBlendOp::Add => Some(gles2_bindings::FUNC_ADD),
            RafxBlendOp::Subtract => Some(gles2_bindings::FUNC_SUBTRACT),
            RafxBlendOp::ReverseSubtract => Some(gles2_bindings::FUNC_REVERSE_SUBTRACT),

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
            unimplemented!("GL ES 2.0 does not support independent blend states");
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
