use crate::gl::conversions::{GlBlendState, GlDepthStencilState, GlRasterizerState};
use crate::gl::gles20::types::GLenum;
use crate::gl::reflection::FieldIndex;
use crate::gl::{LocationId, ProgramId, RafxDeviceContextGl, RafxRootSignatureGl, RafxShaderGl};
use crate::{
    RafxComputePipelineDef, RafxDescriptorIndex, RafxGraphicsPipelineDef, RafxPipelineType,
    RafxResult, RafxRootSignature, MAX_DESCRIPTOR_SET_LAYOUTS,
};
use rafx_base::trust_cell::TrustCell;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct GlAttribute {
    pub(crate) buffer_index: u32,
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
    pub(crate) gl_attributes: Vec<GlAttribute>,
    pub(crate) program_id: ProgramId,
    resource_locations: Vec<Option<LocationId>>,
    uniform_field_locations: Vec<Option<LocationId>>,
    pub(crate) root_signature: RafxRootSignatureGl,
    pub(crate) last_descriptor_updates: TrustCell<[u64; MAX_DESCRIPTOR_SET_LAYOUTS]>,
    pub(crate) last_bound_by_command_pool: TrustCell<u32>,
}

impl GlPipelineInfo {
    pub fn resource_location(
        &self,
        descriptor_index: RafxDescriptorIndex,
    ) -> &Option<LocationId> {
        &self.resource_locations[descriptor_index.0 as usize]
    }

    pub fn uniform_member_location(
        &self,
        field_index: FieldIndex,
    ) -> &Option<LocationId> {
        &self.uniform_field_locations[field_index.0 as usize]
    }
}

#[derive(Debug)]
pub struct RafxPipelineGl {
    pipeline_type: RafxPipelineType,
    // It's a RafxRootSignatureGl, but stored as RafxRootSignature so we can return refs to it
    root_signature: RafxRootSignature,
    shader: RafxShaderGl,
    gl_pipeline_info: Arc<GlPipelineInfo>,
}

impl Drop for RafxPipelineGl {
    fn drop(&mut self) {
        let device_context = self
            .root_signature
            .gl_root_signature()
            .unwrap()
            .device_context();
        device_context
            .gl_context()
            .gl_destroy_program(self.gl_pipeline_info.program_id)
            .unwrap();
    }
}

impl RafxPipelineGl {
    pub fn pipeline_type(&self) -> RafxPipelineType {
        self.pipeline_type
    }

    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn gl_program_id(&self) -> ProgramId {
        self.gl_pipeline_info.program_id
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

        // Create a new program so that we can customize the vertex attributes
        let program_id = gl_context.gl_create_program()?;
        gl_context.gl_attach_shader(program_id, shader.gl_vertex_shader().shader_id())?;
        gl_context.gl_attach_shader(program_id, shader.gl_fragment_shader().shader_id())?;

        let gl_root_signature = pipeline_def.root_signature.gl_root_signature().unwrap();

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
                program_id,
                attribute.location,
                attribute.gl_attribute_name.as_ref().unwrap(),
            )?;

            let gl_type = attribute
                .format
                .gl_type()
                .ok_or_else(|| format!("Unsupported format {:?}", attribute.format))?;

            let buffer = &pipeline_def.vertex_layout.buffers[attribute.buffer_index as usize];

            gl_attributes.push(GlAttribute {
                buffer_index: attribute.buffer_index,
                location: attribute.location,
                channel_count: attribute.format.channel_count(),
                gl_type,
                stride: buffer.stride,
                is_normalized: attribute.format.is_normalized(),
                byte_offset: attribute.byte_offset,
            });
        }

        gl_context.link_shader_program(program_id)?;

        let mut resource_locations = Vec::with_capacity(gl_root_signature.inner.descriptors.len());
        for resource in &gl_root_signature.inner.descriptors {
            resource_locations
                .push(gl_context.gl_get_uniform_location(program_id, &resource.gl_name)?);
        }

        let all_uniform_fields = gl_root_signature.inner.uniform_reflection.fields();
        let mut uniform_field_locations = Vec::with_capacity(all_uniform_fields.len());
        for field in all_uniform_fields {
            uniform_field_locations
                .push(gl_context.gl_get_uniform_location(program_id, &field.name)?);
        }

        //TODO: set up textures?
        //gl_context.gl_use_program(program)?;
        //gl_context.gl_use_program(NONE_PROGRAM)?;

        let gl_topology = pipeline_def
            .primitive_topology
            .gl_topology()
            .ok_or_else(|| {
                format!(
                    "GL ES 2.0 does not support topology {:?}",
                    pipeline_def.primitive_topology
                )
            })?;

        let gl_pipeline_info = GlPipelineInfo {
            last_bound_by_command_pool: TrustCell::new(0),
            gl_rasterizer_state: pipeline_def.rasterizer_state.into(),
            gl_depth_stencil_state: pipeline_def.depth_state.into(),
            gl_blend_state: pipeline_def.blend_state.gl_blend_state()?,
            gl_topology,
            gl_attributes,
            program_id,
            resource_locations,
            uniform_field_locations,
            root_signature: gl_root_signature.clone(),
            last_descriptor_updates: Default::default(),
        };

        Ok(RafxPipelineGl {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: RafxPipelineType::Graphics,
            shader: shader.clone(),
            gl_pipeline_info: Arc::new(gl_pipeline_info),
        })
    }

    pub fn new_compute_pipeline(
        _device_context: &RafxDeviceContextGl,
        _pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<Self> {
        unimplemented!("GL ES 2.0 does not support compute pipelines");
    }
}