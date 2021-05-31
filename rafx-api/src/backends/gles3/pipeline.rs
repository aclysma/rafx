use crate::gles3::conversions::{Gles3BlendState, Gles3DepthStencilState, Gles3RasterizerState};
use crate::gles3::gles3_bindings::types::GLenum;
use crate::gles3::{
    gles3_bindings, LocationId, ProgramId, RafxDeviceContextGles3, RafxRootSignatureGles3,
    RafxShaderGles3,
};
use crate::{
    RafxComputePipelineDef, RafxDescriptorIndex, RafxGraphicsPipelineDef, RafxPipelineType,
    RafxResult, RafxRootSignature, RafxVertexAttributeRate, MAX_DESCRIPTOR_SET_LAYOUTS,
};
use rafx_base::trust_cell::TrustCell;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct Gles3Attribute {
    pub(crate) buffer_index: u32,
    pub(crate) location: u32,
    pub(crate) channel_count: u32,
    pub(crate) gl_type: GLenum,
    pub(crate) stride: u32,
    pub(crate) divisor: u32,
    pub(crate) is_normalized: bool,
    pub(crate) byte_offset: u32,
}

#[derive(Debug)]
pub(crate) struct Gles3PipelineInfo {
    pub(crate) gl_rasterizer_state: Gles3RasterizerState,
    pub(crate) gl_depth_stencil_state: Gles3DepthStencilState,
    pub(crate) gl_blend_state: Gles3BlendState,
    pub(crate) gl_topology: GLenum,
    pub(crate) gl_attributes: Vec<Gles3Attribute>,
    pub(crate) program_id: ProgramId,
    resource_locations: Vec<Option<LocationId>>,
    pub(crate) uniform_block_sizes: Vec<Option<u32>>,
    pub(crate) root_signature: RafxRootSignatureGles3,
    pub(crate) last_descriptor_updates: TrustCell<[u64; MAX_DESCRIPTOR_SET_LAYOUTS]>,
    pub(crate) last_bound_by_command_pool: TrustCell<u32>,
}

impl Gles3PipelineInfo {
    pub fn resource_location(
        &self,
        descriptor_index: RafxDescriptorIndex,
        element_index: u32,
    ) -> &Option<LocationId> {
        let descriptor = self.root_signature.descriptor(descriptor_index).unwrap();
        &self.resource_locations
            [(descriptor.first_location_index.unwrap() + element_index) as usize]
    }
}

#[derive(Debug)]
pub struct RafxPipelineGles3 {
    pipeline_type: RafxPipelineType,
    // It's a RafxRootSignatureGles3, but stored as RafxRootSignature so we can return refs to it
    root_signature: RafxRootSignature,
    shader: RafxShaderGles3,
    gl_pipeline_info: Arc<Gles3PipelineInfo>,
}

impl Drop for RafxPipelineGles3 {
    fn drop(&mut self) {
        let device_context = self
            .root_signature
            .gles3_root_signature()
            .unwrap()
            .device_context();
        device_context
            .gl_context()
            .gl_destroy_program(self.gl_pipeline_info.program_id)
            .unwrap();
    }
}

impl RafxPipelineGles3 {
    pub fn pipeline_type(&self) -> RafxPipelineType {
        self.pipeline_type
    }

    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn gl_program_id(&self) -> ProgramId {
        self.gl_pipeline_info.program_id
    }

    pub(crate) fn gl_pipeline_info(&self) -> &Arc<Gles3PipelineInfo> {
        &self.gl_pipeline_info
    }

    pub fn new_graphics_pipeline(
        device_context: &RafxDeviceContextGles3,
        pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<Self> {
        let gl_context = device_context.gl_context();
        let shader = pipeline_def.shader.gles3_shader().unwrap();

        // Create a new program so that we can customize the vertex attributes
        let program_id = gl_context.gl_create_program()?;
        gl_context.gl_attach_shader(program_id, shader.gl_vertex_shader().shader_id())?;
        gl_context.gl_attach_shader(program_id, shader.gl_fragment_shader().shader_id())?;

        let gl_root_signature = pipeline_def.root_signature.gles3_root_signature().unwrap();

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
                .gles3_type()
                .ok_or_else(|| format!("Unsupported format {:?}", attribute.format))?;

            let buffer = &pipeline_def.vertex_layout.buffers[attribute.buffer_index as usize];

            let divisor = match buffer.rate {
                RafxVertexAttributeRate::Vertex => 0,
                RafxVertexAttributeRate::Instance => 1,
            };

            gl_attributes.push(Gles3Attribute {
                buffer_index: attribute.buffer_index,
                location: attribute.location,
                channel_count: attribute.format.channel_count(),
                gl_type,
                stride: buffer.stride,
                divisor,
                is_normalized: attribute.format.is_normalized(),
                byte_offset: attribute.byte_offset,
            });
        }

        gl_context.link_shader_program(program_id)?;

        if device_context.inner.validate_shaders {
            gl_context.validate_shader_program(program_id)?;
        }

        let mut resource_locations = Vec::with_capacity(gl_root_signature.inner.descriptors.len());
        for resource in &gl_root_signature.inner.descriptors {
            if let Some(first_location_index) = resource.first_location_index {
                for i in 0..resource.element_count {
                    let location_name = &gl_root_signature.inner.location_names
                        [(first_location_index + i) as usize];
                    resource_locations
                        .push(gl_context.gl_get_uniform_location(program_id, location_name)?);
                }
            }
        }

        let mut uniform_block_sizes =
            Vec::with_capacity(gl_root_signature.inner.uniform_block_descriptors.len());
        for &descriptor_index in &gl_root_signature.inner.uniform_block_descriptors {
            let descriptor = gl_root_signature.descriptor(descriptor_index).unwrap();
            let uniform_block_binding = descriptor.uniform_block_binding.unwrap();

            let uniform_block_index =
                gl_context.gl_get_uniform_block_index(program_id, &descriptor.gl_name)?;
            // The uniform block might not be active in this program
            if let Some(uniform_block_index) = uniform_block_index {
                let size = gl_context.gl_get_active_uniform_blockiv(
                    program_id,
                    uniform_block_index,
                    gles3_bindings::UNIFORM_BLOCK_DATA_SIZE,
                )?;
                gl_context.gl_uniform_block_binding(
                    program_id,
                    uniform_block_index,
                    uniform_block_binding,
                )?;
                uniform_block_sizes.push(Some(size as u32));
            } else {
                uniform_block_sizes.push(None);
            }
        }

        let gl_topology = pipeline_def
            .primitive_topology
            .gles3_topology()
            .ok_or_else(|| {
                format!(
                    "GL ES 2.0 does not support topology {:?}",
                    pipeline_def.primitive_topology
                )
            })?;

        let mut gl_pipeline_info = Gles3PipelineInfo {
            last_bound_by_command_pool: TrustCell::new(0),
            gl_rasterizer_state: pipeline_def.rasterizer_state.into(),
            gl_depth_stencil_state: pipeline_def.depth_state.into(),
            gl_blend_state: pipeline_def.blend_state.gles3_blend_state()?,
            gl_topology,
            gl_attributes,
            program_id,
            resource_locations,
            uniform_block_sizes,
            root_signature: gl_root_signature.clone(),
            last_descriptor_updates: Default::default(),
        };

        // Front face needs to be reversed because we render GL with a flipped Y axis:
        // - rafx-shader-processor uses spirv-cross to patch vertex shaders to flip the Y axis
        // - textures can be sampled using V+ in down direction (the way that DX, vulkan, metal work)
        // - we flip the image right-side-up when we do the final present
        gl_pipeline_info.gl_rasterizer_state.front_face = pipeline_def
            .rasterizer_state
            .front_face
            .gles3_front_face(true);

        Ok(RafxPipelineGles3 {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: RafxPipelineType::Graphics,
            shader: shader.clone(),
            gl_pipeline_info: Arc::new(gl_pipeline_info),
        })
    }

    pub fn new_compute_pipeline(
        _device_context: &RafxDeviceContextGles3,
        _pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<Self> {
        unimplemented!("GL ES 2.0 does not support compute pipelines");
    }
}
