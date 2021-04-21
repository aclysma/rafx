use crate::gl::{RafxDeviceContextGl, GlCompiledShader, ProgramId, RafxShaderGl, NONE_PROGRAM};
use crate::{RafxComputePipelineDef, RafxGraphicsPipelineDef, RafxPipelineType, RafxResult, RafxRootSignature, RafxShaderStageFlags, RafxRasterizerState};
use crate::gl::gles20;
use crate::gl::conversions::{GlRasterizerState, GlBlendState, GlDepthStencilState};
use crate::gl::gles20::types::GLenum;
use std::sync::Arc;
// fn gl_entry_point_name(name: &str) -> &str {
//     // "main" is not an allowed entry point name. spirv_cross adds a 0 to the end of any
//     // unallowed entry point names so do that here too
//     if name == "main" {
//         "main0"
//     } else {
//         name
//     }
// }

// #[derive(Debug)]
// enum GlPipelineState {
//     Graphics(gl_rs::RenderPipelineState),
//     Compute(gl_rs::ComputePipelineState),
// }

// #[derive(Debug)]
// pub(crate) struct PipelineComputeEncoderInfo {
//     pub compute_threads_per_group: [u32; 3],
// }
//
// #[derive(Debug)]
// pub(crate) struct PipelineRenderEncoderInfo {
//     // This is all set on the render encoder, so cache it now so we can set it later
//     pub(crate) mtl_cull_mode: gl_rs::MTLCullMode,
//     pub(crate) mtl_triangle_fill_mode: gl_rs::MTLTriangleFillMode,
//     pub(crate) mtl_front_facing_winding: gl_rs::MTLWinding,
//     pub(crate) mtl_depth_bias: f32,
//     pub(crate) mtl_depth_bias_slope_scaled: f32,
//     pub(crate) mtl_depth_clip_mode: gl_rs::MTLDepthClipMode,
//     pub(crate) mtl_depth_stencil_state: Option<gl_rs::DepthStencilState>,
//     pub(crate) mtl_primitive_type: gl_rs::MTLPrimitiveType,
// }



#[derive(Debug)]
pub(crate) struct GlAttribute {
    pub(crate) location: u32,
    pub(crate) channel_count: u32,
    pub(crate) gl_type: GLenum,
    pub(crate) stride: u32,
    pub(crate) is_normalized: bool,
    pub(crate) byte_offset: u32,
}

#[derive(Debug)]
pub(crate) struct GlPipelineInfo {
    pub(crate) gl_rasterizer_state: GlRasterizerState,
    pub(crate) gl_depth_stencil_state: GlDepthStencilState,
    pub(crate) gl_blend_state: GlBlendState,
    pub(crate) gl_topology: GLenum,
    pub(crate) vertex_buffer_stride: u32,
    pub(crate) gl_attributes: Vec<GlAttribute>
}

#[derive(Debug)]
pub struct RafxPipelineGl {
    pipeline_type: RafxPipelineType,
    // It's a RafxRootSignatureGl, but stored as RafxRootSignature so we can return refs to it
    root_signature: RafxRootSignature,
    shader: RafxShaderGl,

    gl_pipeline_info: Arc<GlPipelineInfo>,
}

impl RafxPipelineGl {
    pub fn pipeline_type(&self) -> RafxPipelineType {
        self.pipeline_type
    }

    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn gl_program_id(&self) -> ProgramId {
        self.shader.gl_program_id()
    }

    pub(crate) fn gl_pipeline_info(&self) -> &Arc<GlPipelineInfo> {
        &self.gl_pipeline_info
    }

    pub fn new_graphics_pipeline(
        device_context: &RafxDeviceContextGl,
        pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<Self> {
        let gl_context = device_context.gl_context();
        let shader = pipeline_def.shader.gl_shader().unwrap();
        let program = shader.gl_program_id();

        // Multiple buffers not currently supported
        assert!(pipeline_def.vertex_layout.buffers.len() <= 1);

        let vertex_buffer_stride = pipeline_def.vertex_layout.buffers[0].stride;
        let mut gl_attributes = Vec::with_capacity(pipeline_def.vertex_layout.attributes.len());

        for attribute in &pipeline_def.vertex_layout.attributes {
            if attribute.location >= device_context.device_info().max_vertex_attribute_count {
                Err(format!(
                    "Vertex attribute location {} exceeds max of {}",
                    attribute.location,
                    device_context.device_info().max_vertex_attribute_count
                ))?;
            }

            gl_context.gl_bind_attrib_location(
                program,
                attribute.location,
                attribute.gl_attribute_name.as_ref().unwrap()
            )?;

            let gl_type = attribute.format.gl_type().ok_or_else(|| format!("Unsupported format {:?}", attribute.format))?;

            gl_attributes.push(GlAttribute {
                location: attribute.location,
                channel_count: attribute.format.channel_count(),
                gl_type,
                stride: attribute.format.block_or_pixel_size_in_bytes(),
                is_normalized: attribute.format.is_normalized(),
                byte_offset: attribute.byte_offset
            });
        }

        if !pipeline_def.vertex_layout.attributes.is_empty() {
            gl_context.link_shader_program(program)?;
            //gl_context.validate_shader_program(program)?;
        }

        //TODO: set up textures?
        //gl_context.gl_use_program(program)?;
        //gl_context.gl_use_program(NONE_PROGRAM)?;

        let gl_topology = pipeline_def
            .primitive_topology
            .gl_topology()
            .ok_or_else(|| format!("GL ES 2.0 does not support topology {:?}", pipeline_def.primitive_topology))?;

        let gl_pipeline_info = GlPipelineInfo {
            gl_rasterizer_state: pipeline_def.rasterizer_state.into(),
            gl_depth_stencil_state: pipeline_def.depth_state.into(),
            gl_blend_state: pipeline_def.blend_state.gl_blend_state()?,
            gl_topology,
            vertex_buffer_stride,
            gl_attributes
        };

        Ok(RafxPipelineGl {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: RafxPipelineType::Graphics,
            shader: shader.clone(),
            gl_pipeline_info: Arc::new(gl_pipeline_info)
        })
    }

    pub fn new_compute_pipeline(
        _device_context: &RafxDeviceContextGl,
        _pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<Self> {
        unimplemented!("GL ES 2.0 does not support compute pipelines");
    }
}
