use crate::metal::RafxDeviceContextMetal;
use crate::{
    RafxComputePipelineDef, RafxGraphicsPipelineDef, RafxPipelineType, RafxResult,
    RafxRootSignature, RafxShaderStageFlags,
};

fn metal_entry_point_name(name: &str) -> &str {
    // "main" is not an allowed entry point name. spirv_cross adds a 0 to the end of any
    // unallowed entry point names so do that here too
    if name == "main" {
        "main0"
    } else {
        name
    }
}

#[derive(Debug)]
enum MetalPipelineState {
    Graphics(metal_rs::RenderPipelineState),
    Compute(metal_rs::ComputePipelineState),
}

// for metal_rs::RenderPipelineState
unsafe impl Send for MetalPipelineState {}
unsafe impl Sync for MetalPipelineState {}

#[derive(Debug)]
pub(crate) struct PipelineComputeEncoderInfo {
    pub compute_threads_per_group: [u32; 3],
}

#[derive(Debug)]
pub(crate) struct PipelineRenderEncoderInfo {
    // This is all set on the render encoder, so cache it now so we can set it later
    pub(crate) mtl_cull_mode: metal_rs::MTLCullMode,
    pub(crate) mtl_triangle_fill_mode: metal_rs::MTLTriangleFillMode,
    pub(crate) mtl_front_facing_winding: metal_rs::MTLWinding,
    pub(crate) mtl_depth_bias: f32,
    pub(crate) mtl_depth_bias_slope_scaled: f32,
    pub(crate) mtl_depth_clip_mode: metal_rs::MTLDepthClipMode,
    pub(crate) mtl_depth_stencil_state: Option<metal_rs::DepthStencilState>,
    pub(crate) mtl_primitive_type: metal_rs::MTLPrimitiveType,
}

// for metal_rs::DepthStencilState
unsafe impl Send for PipelineRenderEncoderInfo {}
unsafe impl Sync for PipelineRenderEncoderInfo {}

#[derive(Debug)]
pub struct RafxPipelineMetal {
    pipeline_type: RafxPipelineType,
    // It's a RafxRootSignatureMetal, but stored as RafxRootSignature so we can return refs to it
    root_signature: RafxRootSignature,
    pipeline: MetalPipelineState,

    pub(crate) render_encoder_info: Option<PipelineRenderEncoderInfo>,
    pub(crate) compute_encoder_info: Option<PipelineComputeEncoderInfo>,
}

impl RafxPipelineMetal {
    pub fn pipeline_type(&self) -> RafxPipelineType {
        self.pipeline_type
    }

    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn metal_render_pipeline(&self) -> Option<&metal_rs::RenderPipelineStateRef> {
        match &self.pipeline {
            MetalPipelineState::Graphics(pipeline) => Some(pipeline.as_ref()),
            MetalPipelineState::Compute(_) => None,
        }
    }

    pub fn metal_compute_pipeline(&self) -> Option<&metal_rs::ComputePipelineStateRef> {
        match &self.pipeline {
            MetalPipelineState::Graphics(_) => None,
            MetalPipelineState::Compute(pipeline) => Some(pipeline.as_ref()),
        }
    }

    pub fn new_graphics_pipeline(
        device_context: &RafxDeviceContextMetal,
        pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<Self> {
        let pipeline = metal_rs::RenderPipelineDescriptor::new();

        let mut vertex_function = None;
        let mut fragment_function = None;

        for stage in pipeline_def.shader.metal_shader().unwrap().stages() {
            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::VERTEX)
            {
                let entry_point = metal_entry_point_name(&stage.reflection.entry_point_name);
                assert!(vertex_function.is_none());
                vertex_function = Some(
                    stage
                        .shader_module
                        .metal_shader_module()
                        .unwrap()
                        .library()
                        .get_function(entry_point, None)?,
                );
            }

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::FRAGMENT)
            {
                let entry_point = metal_entry_point_name(&stage.reflection.entry_point_name);
                assert!(fragment_function.is_none());
                fragment_function = Some(
                    stage
                        .shader_module
                        .metal_shader_module()
                        .unwrap()
                        .library()
                        .get_function(entry_point, None)?,
                );
            }
        }

        let vertex_function = vertex_function.ok_or("Could not find vertex function")?;

        pipeline.set_vertex_function(Some(vertex_function.as_ref()));
        pipeline.set_fragment_function(fragment_function.as_ref().map(|x| x.as_ref()));
        pipeline.set_sample_count(pipeline_def.sample_count.into());

        let vertex_descriptor = metal_rs::VertexDescriptor::new();
        for attribute in &pipeline_def.vertex_layout.attributes {
            let buffer_index =
                super::util::vertex_buffer_adjusted_buffer_index(attribute.buffer_index);
            let attribute_descriptor = vertex_descriptor
                .attributes()
                .object_at(attribute.location as _)
                .unwrap();
            attribute_descriptor.set_buffer_index(buffer_index);
            attribute_descriptor.set_format(attribute.format.into());
            attribute_descriptor.set_offset(attribute.offset as _);
        }

        for (index, binding) in pipeline_def.vertex_layout.buffers.iter().enumerate() {
            let buffer_index = super::util::vertex_buffer_adjusted_buffer_index(index as u32);
            let layout_descriptor = vertex_descriptor.layouts().object_at(buffer_index).unwrap();
            layout_descriptor.set_stride(binding.stride as _);
            layout_descriptor.set_step_function(binding.rate.into());
            layout_descriptor.set_step_rate(1);
        }
        pipeline.set_vertex_descriptor(Some(vertex_descriptor));

        pipeline.set_input_primitive_topology(pipeline_def.primitive_topology.into());

        //TODO: Pass in number of color attachments?
        super::util::blend_def_to_attachment(
            pipeline_def.blend_state,
            &mut pipeline.color_attachments(),
            pipeline_def.color_formats.len(),
        );

        for (index, &color_format) in pipeline_def.color_formats.iter().enumerate() {
            pipeline
                .color_attachments()
                .object_at(index as _)
                .unwrap()
                .set_pixel_format(color_format.into());
        }

        if let Some(depth_format) = pipeline_def.depth_stencil_format {
            if depth_format.has_depth() {
                pipeline.set_depth_attachment_pixel_format(depth_format.into());
            }

            if depth_format.has_stencil() {
                pipeline.set_stencil_attachment_pixel_format(depth_format.into());
            }
        }

        let pipeline = device_context
            .device()
            .new_render_pipeline_state(pipeline.as_ref())?;

        let mtl_cull_mode = pipeline_def.rasterizer_state.cull_mode.into();
        let mtl_triangle_fill_mode = pipeline_def.rasterizer_state.fill_mode.into();
        let mtl_front_facing_winding = pipeline_def.rasterizer_state.front_face.into();
        let mtl_depth_bias = pipeline_def.rasterizer_state.depth_bias as f32;
        let mtl_depth_bias_slope_scaled =
            pipeline_def.rasterizer_state.depth_bias_slope_scaled as f32;
        let mtl_depth_clip_mode = if pipeline_def.rasterizer_state.depth_clamp_enable {
            metal_rs::MTLDepthClipMode::Clamp
        } else {
            metal_rs::MTLDepthClipMode::Clip
        };
        let mtl_primitive_type = pipeline_def.primitive_topology.into();

        let depth_stencil_descriptor =
            super::util::depth_state_to_descriptor(&pipeline_def.depth_state);
        let mtl_depth_stencil_state = if pipeline_def.depth_stencil_format.is_some() {
            Some(
                device_context
                    .device()
                    .new_depth_stencil_state(depth_stencil_descriptor.as_ref()),
            )
        } else {
            None
        };

        let render_encoder_info = PipelineRenderEncoderInfo {
            mtl_cull_mode,
            mtl_triangle_fill_mode,
            mtl_front_facing_winding,
            mtl_depth_bias,
            mtl_depth_bias_slope_scaled,
            mtl_depth_clip_mode,
            mtl_depth_stencil_state,
            mtl_primitive_type,
        };

        Ok(RafxPipelineMetal {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: pipeline_def.root_signature.pipeline_type(),
            pipeline: MetalPipelineState::Graphics(pipeline),
            render_encoder_info: Some(render_encoder_info),
            compute_encoder_info: None,
        })
    }

    pub fn new_compute_pipeline(
        device_context: &RafxDeviceContextMetal,
        pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<Self> {
        let mut compute_function = None;
        let mut compute_threads_per_group = None;

        for stage in pipeline_def.shader.metal_shader().unwrap().stages() {
            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::COMPUTE)
            {
                let entry_point = metal_entry_point_name(&stage.reflection.entry_point_name);

                assert!(compute_function.is_none());
                compute_function = Some(
                    stage
                        .shader_module
                        .metal_shader_module()
                        .unwrap()
                        .library()
                        .get_function(entry_point, None)?,
                );

                compute_threads_per_group = stage.reflection.compute_threads_per_group;
            }
        }

        let compute_function = compute_function.ok_or("Could not find compute function")?;

        let pipeline = device_context
            .device()
            .new_compute_pipeline_state_with_function(compute_function.as_ref())?;

        let compute_encoder_info = PipelineComputeEncoderInfo {
            compute_threads_per_group: compute_threads_per_group.unwrap(),
        };

        Ok(RafxPipelineMetal {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: pipeline_def.root_signature.pipeline_type(),
            pipeline: MetalPipelineState::Compute(pipeline),
            render_encoder_info: None,
            compute_encoder_info: Some(compute_encoder_info),
        })
    }
}
