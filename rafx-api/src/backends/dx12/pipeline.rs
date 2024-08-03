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
use windows::core::Vtable;

macro_rules! pipeline_state_stream_subobject {
    ($struct_name:ident, $constant:expr, $inner_type:ty) => {
        #[repr(C, align(8))]
        struct $struct_name
        {
            subobject_type: d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE,
            inner: $inner_type
        }

        impl Default for $struct_name
        {
            fn default() -> Self {
                Self {
                    subobject_type: $constant,
                    inner: <$inner_type>::default()
                }
            }
        }
    }
}

macro_rules! pipeline_state_stream_subobject_with_default {
    ($struct_name:ident, $constant:expr, $inner_type:ty, $default_value:expr) => {
        #[repr(C, align(8))]
        struct $struct_name
        {
            subobject_type: d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE,
            inner: $inner_type
        }

        impl Default for $struct_name
        {
            fn default() -> Self {
                Self {
                    subobject_type: $constant,
                    inner: $default_value
                }
            }
        }
    }
}

pipeline_state_stream_subobject!(PipelineStateStreamFlags, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_FLAGS, d3d12::D3D12_PIPELINE_STATE_FLAGS);
pipeline_state_stream_subobject!(PipelineStateStreamNodeMask, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_NODE_MASK, u32);
pipeline_state_stream_subobject_with_default!(PipelineStateStreamRootSignature, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_ROOT_SIGNATURE, *const d3d12::ID3D12RootSignature, std::ptr::null_mut());
pipeline_state_stream_subobject!(PipelineStateStreamInputLayout, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_INPUT_LAYOUT, d3d12::D3D12_INPUT_LAYOUT_DESC);
pipeline_state_stream_subobject!(PipelineStateStreamIbStripCutValue, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_IB_STRIP_CUT_VALUE, d3d12::D3D12_INDEX_BUFFER_STRIP_CUT_VALUE);
pipeline_state_stream_subobject!(PipelineStateStreamPrimitiveTopologyType, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PRIMITIVE_TOPOLOGY, d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE);
pipeline_state_stream_subobject!(PipelineStateStreamVS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_VS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamGS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_GS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamStreamOutput, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_STREAM_OUTPUT, d3d12::D3D12_STREAM_OUTPUT_DESC);
pipeline_state_stream_subobject!(PipelineStateStreamHS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_HS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamDS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamPS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamAS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_AS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamMS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_MS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamCS, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_CS, d3d12::D3D12_SHADER_BYTECODE);
pipeline_state_stream_subobject!(PipelineStateStreamBlendDesc, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_BLEND, d3d12::D3D12_BLEND_DESC);
pipeline_state_stream_subobject!(PipelineStateStreamDepthStencil, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL, d3d12::D3D12_DEPTH_STENCIL_DESC);
pipeline_state_stream_subobject!(PipelineStateStreamDepthStencil1, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL1, d3d12::D3D12_DEPTH_STENCIL_DESC1);
// if (D3D12_SDK_VERSION >= 606)
//pipeline_state_stream_subobject!(PipelineStateStreamDepthStencil2, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL2, d3d12::D3D12_DEPTH_STENCIL_DESC2);
pipeline_state_stream_subobject!(PipelineStateStreamDepthStencilFormat, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL_FORMAT, dxgi::Common::DXGI_FORMAT);
pipeline_state_stream_subobject!(PipelineStateStreamRasterizer, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RASTERIZER, d3d12::D3D12_RASTERIZER_DESC);
// if (D3D12_SDK_VERSION >= 608)
//pipeline_state_stream_subobject!(PipelineStateStreamRasterizer1, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RASTERIZER1, d3d12::D3D12_RASTERIZER_DESC1);
// if (D3D12_SDK_VERSION >= 610)
//pipeline_state_stream_subobject!(PipelineStateStreamRasterizer2, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RASTERIZER2, d3d12::D3D12_RASTERIZER_DESC2);
pipeline_state_stream_subobject!(PipelineStateStreamRenderTargetFormats, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RENDER_TARGET_FORMATS, d3d12::D3D12_RT_FORMAT_ARRAY);
pipeline_state_stream_subobject!(PipelineStateStreamSampleDesc, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_DESC, dxgi::Common::DXGI_SAMPLE_DESC);
pipeline_state_stream_subobject!(PipelineStateStreamSampleMask, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_MASK, u32);
pipeline_state_stream_subobject!(PipelineStateStreamCachedPso, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_CACHED_PSO, d3d12::D3D12_CACHED_PIPELINE_STATE);
pipeline_state_stream_subobject!(PipelineStateStreamViewInstancing, d3d12::D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_VIEW_INSTANCING, d3d12::D3D12_VIEW_INSTANCING_DESC);

#[derive(Default)]
#[repr(C)]
struct PipelineStreamObjectMesh {
    flags: PipelineStateStreamFlags,
    node_mask: PipelineStateStreamNodeMask,
    root_signature: PipelineStateStreamRootSignature,
    //input_layout: PipelineStateStreamInputLayout,
    //ib_strip_cut_value: PipelineStateStreamIbStripCutValue,
    primitive_topology_type: PipelineStateStreamPrimitiveTopologyType,
    //vs: PipelineStateStreamVS,
    //gs: PipelineStateStreamGS,
    stream_output: PipelineStateStreamStreamOutput,
    //hs: PipelineStateStreamHS,
    //ds: PipelineStateStreamDS,
    ps: PipelineStateStreamPS,
    r#as: PipelineStateStreamAS,
    ms: PipelineStateStreamMS,
    //cs: PipelineStateStreamCS,
    blend: PipelineStateStreamBlendDesc,
    depth_stencil: PipelineStateStreamDepthStencil,
    //depth_stencil1: PipelineStateStreamDepthStencil1,
    //depth_stencil2: PipelineStateStreamDepthStencil2,
    dsv_format: PipelineStateStreamDepthStencilFormat,
    rasterizer: PipelineStateStreamRasterizer,
    //rasterizer1: PipelineStateStreamRasterizer1,
    //rasterizer2: PipelineStateStreamRasterizer2,
    rtv_formats: PipelineStateStreamRenderTargetFormats,
    sample_desc: PipelineStateStreamSampleDesc,
    sample_mask: PipelineStateStreamSampleMask,
    cached_pso: PipelineStateStreamCachedPso,
    view_instancing: PipelineStateStreamViewInstancing,
}


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
        let mut ms_bytecode = None;

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

            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::MESH)
            {
                ms_bytecode = Some(
                    module.get_or_compile_bytecode(&stage.reflection.entry_point_name, "ms_6_5")?,
                );
            }
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

        let blend_state = super::internal::conversions::blend_state_blend_state_desc(
            pipeline_def.blend_state,
            render_target_count,
        );

        let rasterizer_state = super::internal::conversions::rasterizer_state_rasterizer_desc(
            pipeline_def.rasterizer_state,
        );

        let pipeline_state = if ms_bytecode.is_some() {
            // Treat as a graphics pipeline using mesh shaders
            use windows::core::Interface;
            let device2 = device_context.d3d12_device().cast::<d3d12::ID3D12Device2>().unwrap();

            //let dx12_root_sig = pipeline_def.root_signature.dx12_root_signature().unwrap().dx12_root_signature();
            let root_sig_ptr = pipeline_def.root_signature.dx12_root_signature().unwrap().dx12_root_signature().as_raw();

            let mut pipeline_stream_object = PipelineStreamObjectMesh::default();

            pipeline_stream_object.root_signature.inner = root_sig_ptr as *const d3d12::ID3D12RootSignature;
            pipeline_stream_object.ms.inner = ms_bytecode.map(|x| *x.bytecode()).unwrap_or_default();
            pipeline_stream_object.ps.inner = ps_bytecode.map(|x| *x.bytecode()).unwrap_or_default();
            pipeline_stream_object.blend.inner = blend_state;
            pipeline_stream_object.sample_mask.inner = u32::MAX;
            pipeline_stream_object.rasterizer.inner = rasterizer_state;
            pipeline_stream_object.depth_stencil.inner = depth_stencil_desc;
            pipeline_stream_object.primitive_topology_type.inner = pipeline_def.primitive_topology.into();
            pipeline_stream_object.rtv_formats.inner.NumRenderTargets = render_target_count as u32;
            pipeline_stream_object.rtv_formats.inner.RTFormats = rtv_formats;
            pipeline_stream_object.dsv_format.inner = pipeline_def
                .depth_stencil_format
                .map(|x| x.into())
                .unwrap_or(dxgi::Common::DXGI_FORMAT_UNKNOWN);
            pipeline_stream_object.sample_desc.inner = sample_desc;
            pipeline_stream_object.cached_pso.inner = cached_pipeline_state;
            pipeline_stream_object.flags.inner = d3d12::D3D12_PIPELINE_STATE_FLAG_NONE;
            pipeline_stream_object.node_mask.inner = 0;




            //pipeline_stream_object.vs.inner = vs_bytecode.map(|x| *x.bytecode()).as_ref().unwrap_or_default();
            let pipeline_state_desc = d3d12::D3D12_PIPELINE_STATE_STREAM_DESC {
                SizeInBytes: std::mem::size_of::<PipelineStreamObjectMesh>(),
                pPipelineStateSubobjectStream: ((&mut pipeline_stream_object) as *mut PipelineStreamObjectMesh) as *mut std::ffi::c_void
            };
            let pipeline_state: d3d12::ID3D12PipelineState = unsafe {
                device2.CreatePipelineState(
                    &pipeline_state_desc as * const d3d12::D3D12_PIPELINE_STATE_STREAM_DESC
                ).unwrap()
            };

            pipeline_state
        } else {
            // Treat as a standard graphics pipeline

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
                BlendState: blend_state,
                SampleMask: u32::MAX,
                RasterizerState: rasterizer_state,
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
            let pipeline_state: d3d12::ID3D12PipelineState = unsafe {
                device_context
                    .d3d12_device()
                    .CreateGraphicsPipelineState(&pipeline_state_desc)?
            };

            pipeline_state
        };



        let topology = pipeline_def.primitive_topology.into();

        let pipeline = RafxPipelineDx12 {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: pipeline_def.root_signature.pipeline_type(),
            pipeline: pipeline_state,
            topology,
            vertex_buffer_strides,
        };

        if let Some(debug_name) = pipeline_def.debug_name {
            pipeline.set_debug_name(debug_name)
        }

        Ok(pipeline)
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

        let pipeline = RafxPipelineDx12 {
            root_signature: pipeline_def.root_signature.clone(),
            pipeline_type: pipeline_def.root_signature.pipeline_type(),
            pipeline,
            topology: d3d::D3D_PRIMITIVE_TOPOLOGY_UNDEFINED,
            vertex_buffer_strides: [0; crate::MAX_VERTEX_INPUT_BINDINGS],
        };

        if let Some(debug_name) = pipeline_def.debug_name {
            pipeline.set_debug_name(debug_name)
        }

        Ok(pipeline)
    }
}
