use super::d3d;
use super::d3d12;
use super::dxgi;
use crate::dx12::RafxDeviceContextDx12;
use crate::{
    RafxComputePipelineDef, RafxGraphicsPipelineDef, RafxPipelineType, RafxResult,
    RafxRootSignature, RafxShaderStageFlags, RafxVertexAttributeRate,
    MAX_RENDER_TARGET_ATTACHMENTS,
};
use std::ffi::CString;

#[derive(Debug)]
pub struct RafxPipelineDx12 {
    pipeline_type: RafxPipelineType,
    // It's a RafxRootSignatureDx12, but stored as RafxRootSignature so we can return refs to it
    root_signature: RafxRootSignature,
    pipeline: d3d12::ID3D12PipelineState,
    topology: d3d::D3D_PRIMITIVE_TOPOLOGY,
    vertex_buffer_strides: [u32; crate::MAX_VERTEX_INPUT_BINDINGS],
}

impl RafxPipelineDx12 {
    pub fn pipeline_type(&self) -> RafxPipelineType {
        self.pipeline_type
    }

    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn pipeline(&self) -> &d3d12::ID3D12PipelineState {
        &self.pipeline
    }

    pub fn topology(&self) -> d3d::D3D_PRIMITIVE_TOPOLOGY {
        self.topology
    }

    pub fn vertex_buffer_strides(&self) -> &[u32; crate::MAX_VERTEX_INPUT_BINDINGS] {
        &self.vertex_buffer_strides
    }

    pub fn set_debug_name(
        &self,
        name: impl AsRef<str>,
    ) {
        if self
            .root_signature
            .dx12_root_signature()
            .unwrap()
            .inner
            .device_context
            .device_info()
            .debug_names_enabled
        {
            unsafe {
                let name: &str = name.as_ref();
                let utf16: Vec<_> = name.encode_utf16().chain(std::iter::once(0)).collect();
                self.pipeline
                    .SetName(windows::core::PCWSTR::from_raw(utf16.as_ptr()))
                    .unwrap();
                //TODO: Also set on allocation, views, etc?
            }
        }
    }

    pub fn new_graphics_pipeline(
        device_context: &RafxDeviceContextDx12,
        pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<Self> {
        //TODO: ID3D12PipelineLibrary?

        let mut vs_bytecode = None;
        let mut ps_bytecode = None;
        let mut ds_bytecode = None;
        let mut hs_bytecode = None;
        let mut gs_bytecode = None;

        for stage in pipeline_def.shader.dx12_shader().unwrap().stages() {
            let module = stage.shader_module.dx12_shader_module().unwrap();

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::VERTEX)
            {
                vs_bytecode = Some(
                    module.get_or_compile_bytecode(&stage.reflection.entry_point_name, "vs_6_0")?,
                );
            }

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::FRAGMENT)
            {
                ps_bytecode = Some(
                    module.get_or_compile_bytecode(&stage.reflection.entry_point_name, "ps_6_0")?,
                );
            }

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::TESSELLATION_EVALUATION)
            {
                ds_bytecode = Some(
                    module.get_or_compile_bytecode(&stage.reflection.entry_point_name, "ds_6_0")?,
                );
            }

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::TESSELLATION_CONTROL)
            {
                hs_bytecode = Some(
                    module.get_or_compile_bytecode(&stage.reflection.entry_point_name, "hs_6_0")?,
                );
            }

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::GEOMETRY)
            {
                gs_bytecode = Some(
                    module.get_or_compile_bytecode(&stage.reflection.entry_point_name, "gs_6_0")?,
                );
            }
            //stage.reflection.shader_stage;
            // somehow get bytecode? reflection defines entry point and type of shader
            // probably query the shader module, it compiles and caches. we could have pre-compiled
            // and look it up as well
        }

        // can leave everything zero'd out
        let stream_out_desc = d3d12::D3D12_STREAM_OUTPUT_DESC::default();

        let depth_stencil_desc = if pipeline_def.depth_stencil_format.is_some() {
            super::internal::conversions::depth_state_depth_stencil_desc(pipeline_def.depth_state)
        } else {
            d3d12::D3D12_DEPTH_STENCIL_DESC::default()
        };

        // These are parallel arrays
        let mut input_elements = Vec::<d3d12::D3D12_INPUT_ELEMENT_DESC>::with_capacity(16);
        let mut semantic_name_cstrings = Vec::default();

        let mut vertex_buffer_strides = [0; crate::MAX_VERTEX_INPUT_BINDINGS];
        for (i, vertex_buffer_binding) in pipeline_def.vertex_layout.buffers.iter().enumerate() {
            vertex_buffer_strides[i] = vertex_buffer_binding.stride;
        }

        //TODO: Handle no attributes? We can turn IA off
        for vertex_attribute in &pipeline_def.vertex_layout.attributes {
            // Kind of dumb, we need to convert e.g. TEXCOORD1 to "TEXCOORD" and 1
            let mut ending_digit_count = 0;
            for char in vertex_attribute.hlsl_semantic.chars().rev() {
                if char.is_ascii_digit() {
                    ending_digit_count += 1;
                }
            }

            let (semantic_name, semantic_index) = if ending_digit_count > 0 {
                let name_chars = &vertex_attribute.hlsl_semantic
                    [0..(vertex_attribute.hlsl_semantic.len() - ending_digit_count)];
                let number_chars =
                    &vertex_attribute.hlsl_semantic[(vertex_attribute.hlsl_semantic.len()
                        - ending_digit_count)
                        ..vertex_attribute.hlsl_semantic.len()];

                let ending_number = number_chars.parse::<u32>().unwrap();

                (CString::new(name_chars).unwrap(), ending_number)
            } else {
                (CString::new(&*vertex_attribute.hlsl_semantic).unwrap(), 0)
            };

            let input_format = vertex_attribute.format.into();
            log::trace!(
                "pushing input element {:?}/{} format {:?}={:?}",
                semantic_name,
                semantic_index,
                vertex_attribute.format,
                input_format
            );

            // Allocate a null-terminated ascii version of the string.. we use the pointer here so
            // must keep it allocated until the pointer is no longer needed
            let semantic_name_ptr = semantic_name.as_ptr();
            semantic_name_cstrings.push(semantic_name);

            //for attribute in vertex_layout.
            let mut input_element = d3d12::D3D12_INPUT_ELEMENT_DESC {
                SemanticName: windows::core::PCSTR::from_raw(semantic_name_ptr as *const u8),
                //TODO: Might need to change this for SEMANTIC_TEXCOORD1, SEMANTIC_TEXCOORD2, etc.
                SemanticIndex: semantic_index,
                Format: input_format,
                InputSlot: vertex_attribute.buffer_index,
                AlignedByteOffset: vertex_attribute.byte_offset,
                InputSlotClass: d3d12::D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            };

            match pipeline_def.vertex_layout.buffers[vertex_attribute.buffer_index as usize].rate {
                RafxVertexAttributeRate::Vertex => {
                    input_element.InputSlotClass =
                        d3d12::D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA;
                    input_element.InstanceDataStepRate = 0;
                }
                RafxVertexAttributeRate::Instance => {
                    input_element.InputSlotClass =
                        d3d12::D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA;
                    input_element.InstanceDataStepRate = 1;
                }
            }

            // may need to cache these inputs if we use a pipeline cache
            input_elements.push(input_element);
        }

        let input_layout_desc = if input_elements.is_empty() {
            d3d12::D3D12_INPUT_LAYOUT_DESC::default()
        } else {
            d3d12::D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: input_elements.as_ptr(),
                NumElements: input_elements.len() as u32,
            }
        };

        let sample_desc = dxgi::Common::DXGI_SAMPLE_DESC {
            Count: pipeline_def.sample_count.as_u32(),
            Quality: 0, // TODO: Expose quality?
        };

        let cached_pipeline_state = d3d12::D3D12_CACHED_PIPELINE_STATE::default();

        let render_target_count = pipeline_def
            .color_formats
            .len()
            .min(MAX_RENDER_TARGET_ATTACHMENTS);

        let mut rtv_formats = [dxgi::Common::DXGI_FORMAT_UNKNOWN; MAX_RENDER_TARGET_ATTACHMENTS];
        for i in 0..render_target_count {
            rtv_formats[i] = pipeline_def.color_formats[i].into();
        }

        let pipeline_state_desc = d3d12::D3D12_GRAPHICS_PIPELINE_STATE_DESC {
            pRootSignature: ::windows::core::ManuallyDrop::new(
                &pipeline_def
                    .root_signature
                    .dx12_root_signature()
                    .unwrap()
                    .dx12_root_signature()
                    .clone(),
            ),
            VS: vs_bytecode.map(|x| *x.bytecode()).unwrap_or_default(),
            PS: ps_bytecode.map(|x| *x.bytecode()).unwrap_or_default(),
            DS: ds_bytecode.map(|x| *x.bytecode()).unwrap_or_default(),
            GS: gs_bytecode.map(|x| *x.bytecode()).unwrap_or_default(),
            HS: hs_bytecode.map(|x| *x.bytecode()).unwrap_or_default(),
            StreamOutput: stream_out_desc,
            BlendState: super::internal::conversions::blend_state_blend_state_desc(
                pipeline_def.blend_state,
                render_target_count,
            ),
            SampleMask: u32::MAX,
            RasterizerState: super::internal::conversions::rasterizer_state_rasterizer_desc(
                pipeline_def.rasterizer_state,
            ),
            DepthStencilState: depth_stencil_desc, //super::internal::conversions::depth_state_depth_stencil_desc(pipeline_def.depth_state),
            InputLayout: input_layout_desc,
            IBStripCutValue: d3d12::D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
            PrimitiveTopologyType: pipeline_def.primitive_topology.into(),
            NumRenderTargets: render_target_count as u32,
            RTVFormats: rtv_formats,
            DSVFormat: pipeline_def
                .depth_stencil_format
                .map(|x| x.into())
                .unwrap_or(dxgi::Common::DXGI_FORMAT_UNKNOWN),
            SampleDesc: sample_desc,
            CachedPSO: cached_pipeline_state,
            Flags: d3d12::D3D12_PIPELINE_STATE_FLAG_NONE,
            NodeMask: 0,
        };

        //TODO: More hashing required if using PSO cache

        //TODO: Try to find cached PSO

        // If we didn't have it cached, build it
        let pipeline: d3d12::ID3D12PipelineState = unsafe {
            device_context
                .d3d12_device()
                .CreateGraphicsPipelineState(&pipeline_state_desc)?
        };

        let topology = pipeline_def.primitive_topology.into();

        Ok(RafxPipelineDx12 {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: pipeline_def.root_signature.pipeline_type(),
            pipeline,
            topology,
            vertex_buffer_strides,
        })
    }

    pub fn new_compute_pipeline(
        device_context: &RafxDeviceContextDx12,
        pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<Self> {
        let mut cs_bytecode = None;

        for stage in pipeline_def.shader.dx12_shader().unwrap().stages() {
            let module = stage.shader_module.dx12_shader_module().unwrap();

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::COMPUTE)
            {
                assert!(cs_bytecode.is_none());
                cs_bytecode = Some(
                    module.get_or_compile_bytecode(&stage.reflection.entry_point_name, "cs_6_0")?,
                );
            } else {
                Err("Tried to create compute pipeline with a non-compute shader stage specified")?;
            }
        }

        let cached_pipeline_state = d3d12::D3D12_CACHED_PIPELINE_STATE::default();

        log::info!(
            "creating pipeline with root descriptor {:?}",
            pipeline_def
                .root_signature
                .dx12_root_signature()
                .unwrap()
                .dx12_root_signature()
        );

        let pipeline_state_desc = d3d12::D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: ::windows::core::ManuallyDrop::new(
                &pipeline_def
                    .root_signature
                    .dx12_root_signature()
                    .unwrap()
                    .dx12_root_signature()
                    .clone(),
            ),
            CS: cs_bytecode.map(|x| *x.bytecode()).unwrap_or_default(),
            CachedPSO: cached_pipeline_state,
            Flags: d3d12::D3D12_PIPELINE_STATE_FLAG_NONE,
            NodeMask: 0,
        };

        //TODO: More hashing required if using PSO cache

        //TODO: Try to find cached PSO

        // If we didn't have it cached, build it
        let pipeline: d3d12::ID3D12PipelineState = unsafe {
            device_context
                .d3d12_device()
                .CreateComputePipelineState(&pipeline_state_desc)?
        };

        Ok(RafxPipelineDx12 {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: pipeline_def.root_signature.pipeline_type(),
            pipeline,
            topology: d3d::D3D_PRIMITIVE_TOPOLOGY_UNDEFINED,
            vertex_buffer_strides: [0; crate::MAX_VERTEX_INPUT_BINDINGS],
        })
    }
}
