use super::d3d12;
use crate::dx12::RafxDeviceContextDx12;
use crate::{
    RafxComputePipelineDef, RafxImmutableSamplerKey, RafxImmutableSamplers, RafxPipeline,
    RafxResourceType, RafxResult, RafxRootSignature, RafxRootSignatureDef, RafxSampler,
    RafxSamplerDef, RafxShader, RafxShaderModule, RafxShaderModuleDefDx12, RafxShaderResource,
    RafxShaderStageDef, RafxShaderStageFlags, RafxShaderStageReflection,
};

pub struct Dx12MipmapResources {
    //pub shader: RafxShader,
    //pub root_signature: RafxRootSignature,
    //pub pipeline: RafxPipeline
    pub root_signature: d3d12::ID3D12RootSignature,
    pub pipeline: d3d12::ID3D12PipelineState,
}

impl Dx12MipmapResources {
    pub fn new(dx12_device_context: &RafxDeviceContextDx12) -> RafxResult<Self> {
        let generate_mips_shader_src = include_str!("../shaders/GenerateMipsCS.hlsl");

        //let d3d_device = dx12_device_context.d3d12_device();

        //
        // Compile the shader
        //
        let mut bytecode = hassle_rs::compile_hlsl(
            "GenerateMipsCS.hlsl",
            generate_mips_shader_src,
            "main",
            "cs_6_0",
            &["/Zi"],
            &[],
        )?;

        hassle_rs::fake_sign_dxil_in_place(&mut bytecode);

        let dxc = hassle_rs::Dxc::new(None).unwrap();
        let library = dxc.create_library().unwrap();
        let blob_encoding = library.create_blob_with_encoding(&bytecode).unwrap();
        let blob: hassle_rs::DxcBlob = blob_encoding.into();
        let root_signature: d3d12::ID3D12RootSignature = unsafe {
            dx12_device_context
                .d3d12_device()
                .CreateRootSignature(0, blob.as_ref())
                .unwrap()
        };
        unsafe {
            root_signature
                .SetName(windows::w!("MipmapRootSignature"))
                .unwrap();
        }

        let shader_bytecode = d3d12::D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytecode.as_ptr() as *const std::ffi::c_void,
            BytecodeLength: bytecode.len(),
        };

        let pipeline_state_desc = d3d12::D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: ::windows::core::ManuallyDrop::none(),
            CS: shader_bytecode,
            CachedPSO: Default::default(),
            Flags: d3d12::D3D12_PIPELINE_STATE_FLAG_NONE,
            NodeMask: 0,
        };

        let pipeline: d3d12::ID3D12PipelineState = unsafe {
            dx12_device_context
                .d3d12_device()
                .CreateComputePipelineState(&pipeline_state_desc)?
        };
        unsafe {
            pipeline.SetName(windows::w!("MipmapPipeline")).unwrap();
        }

        //let dxc = hassle_rs::Dxc::new(None).unwrap();
        //let library = dxc.create_library().unwrap();
        // let blob_encoding = library.create_blob_with_encoding_from_str(generate_mips_shader_src).unwrap();
        // let blob: hassle_rs::DxcBlob = blob_encoding.into();
        // let root_signature: d3d12::ID3D12RootSignature = unsafe {
        //     dx12_device_context.d3d12_device().CreateRootSignature(0, blob.as_ref()).unwrap()
        // };

        //pipeline.

        //
        // Create the sampler
        //
        // let sampler_desc = d3d12::D3D12_SAMPLER_DESC {
        //     Filter: d3d12::D3D12_FILTER_COMPARISON_MIN_MAG_MIP_LINEAR,
        //     AddressU: d3d12::D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
        //     AddressV: d3d12::D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
        //     AddressW: d3d12::D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
        //     MipLODBias: 0.0,
        //     MaxAnisotropy: 1,
        //     ComparisonFunc: d3d12::D3D12_COMPARISON_FUNC_NEVER,
        //     BorderColor: [0.0, 0.0, 0.0, 0.0],
        //     MinLOD: 0.0,
        //     MaxLOD: f32::MAX,
        // };

        // let sampler_heap = &dx12_device_context.inner.heaps.sampler_heap;
        // let sampler_descriptor = dx12_device_context
        //     .inner
        //     .heaps
        //     .sampler_heap
        //     .allocate(dx12_device_context.d3d12_device(), 1)?;
        // unsafe {
        //     dx12_device_context.d3d12_device().CreateSampler(
        //         &sampler_desc,
        //         sampler_heap.id_to_cpu_handle(sampler_descriptor),
        //     )
        // };

        //
        // Create the root signature
        //

        //
        // Set up immutable descriptors
        //

        //
        // Root params
        //
        /*
        let mut root_params = Vec::default();

        let mut root_param = d3d12::D3D12_ROOT_PARAMETER1::default();
        root_param.ParameterType = d3d12::D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;
        root_param.ShaderVisibility = resource.used_in_shader_stages.into();
        root_param.Anonymous.Constants.RegisterSpace = resource.dx12_space.unwrap();
        root_param.Anonymous.Constants.ShaderRegister = resource.dx12_reg.unwrap();
        root_param.Anonymous.Constants.Num32BitValues =
            rafx_base::memory::round_size_up_to_alignment_u32(resource.size_in_bytes, 4)
                / 4;
        root_params.push(root_param);

        //
        // Make the root signature
        //
        let mut root_sig_desc = d3d12::D3D12_VERSIONED_ROOT_SIGNATURE_DESC::default();
        root_sig_desc.Version = d3d12::D3D_ROOT_SIGNATURE_VERSION_1_1;
        if !root_params.is_empty() {
            root_sig_desc.Anonymous.Desc_1_1.NumParameters = root_params.len() as u32;
            root_sig_desc.Anonymous.Desc_1_1.pParameters = &root_params[0];
        }
        if !static_samplers.is_empty() {
            root_sig_desc.Anonymous.Desc_1_1.NumStaticSamplers = static_samplers.len() as u32;
            root_sig_desc.Anonymous.Desc_1_1.pStaticSamplers = &static_samplers[0];
        }
        root_sig_desc.Anonymous.Desc_1_1.Flags = root_signature_flags;

        let mut root_sig_string = None;
        let mut root_sig_error = None;
        let dx12_root_signature: d3d12::ID3D12RootSignature = unsafe {
            let result = d3d12::D3D12SerializeVersionedRootSignature(
                &root_sig_desc,
                &mut root_sig_string,
                Some(&mut root_sig_error),
            );

            if let Some(root_sig_error) = &root_sig_error {
                let str_slice = std::slice::from_raw_parts(
                    root_sig_error.GetBufferPointer() as *const u8,
                    root_sig_error.GetBufferSize(),
                );
                let str = String::from_utf8_lossy(str_slice);
                println!("root sig error {}", str);
                Err(str.to_string())?;
            }

            result?;

            let root_sig_string = root_sig_string.unwrap();
            let sig_string: &[u8] = std::slice::from_raw_parts(
                root_sig_string.GetBufferPointer() as *const u8,
                root_sig_string.GetBufferSize(),
            );
            let str = String::from_utf8_lossy(sig_string);
            println!("root sig {}", str);

            device_context
                .d3d12_device()
                .CreateRootSignature(0, sig_string)?
        };

        */

        //
        // Create the pipeline
        //

        //let mut root_sig_disc = super::d3d12::D3D12_ROOT_SIGNATURE_DESC::default();
        //root_sig_disc.

        //d3d_device.CreateCommandSignature()

        /*
                const GENERATE_MIPS_SAMPLER_BINDING: u32 = 0;
                const GENERATE_MIPS_SRC_MIP_BINDING: u32 = 1;
                const GENERATE_MIPS_DST_MIP_BINDING: u32 = 2;

                let shader_module = RafxShaderModule::Dx12(dx12_device_context.create_shader_module(RafxShaderModuleDefDx12::HlslSrc(generate_mips_shader_src))?);
                let shader = RafxShader::Dx12(dx12_device_context.create_shader(vec![RafxShaderStageDef {
                    shader_module,
                    reflection: RafxShaderStageReflection {
                        entry_point_name: "main".to_string(),
                        shader_stage: RafxShaderStageFlags::COMPUTE,
                        compute_threads_per_group: Some([8, 8, 1]),
                        resources: vec![
                            RafxShaderResource {
                                name: Some("CBuffer".to_string()),
                                resource_type: RafxResourceType::ROOT_CONSTANT,
                                set_index: u32::MAX,
                                binding: u32::MAX,
                                element_count: 0,
                                size_in_bytes: 16,
                                used_in_shader_stages: RafxShaderStageFlags::COMPUTE,
                                dx12_reg: Some(0),
                                dx12_space: Some(0),
                                ..Default::default()
                            },
                            RafxShaderResource {
                                name: Some("BilinearClamp".to_string()),
                                resource_type: RafxResourceType::SAMPLER,
                                set_index: 0,
                                binding: GENERATE_MIPS_SAMPLER_BINDING,
                                element_count: 1,
                                size_in_bytes: 0,
                                used_in_shader_stages: RafxShaderStageFlags::COMPUTE,
                                dx12_reg: Some(0),
                                dx12_space: Some(0),
                                ..Default::default()
                            },
                            RafxShaderResource {
                                name: Some("SrcMip".to_string()),
                                resource_type: RafxResourceType::TEXTURE,
                                set_index: 0,
                                binding: GENERATE_MIPS_SRC_MIP_BINDING,
                                element_count: 1,
                                size_in_bytes: 0,
                                used_in_shader_stages: RafxShaderStageFlags::COMPUTE,
                                dx12_reg: Some(0),
                                dx12_space: Some(0),
                                ..Default::default()
                            },
                            RafxShaderResource {
                                name: Some("OutMip1".to_string()),
                                resource_type: RafxResourceType::TEXTURE_READ_WRITE,
                                set_index: 0,
                                binding: GENERATE_MIPS_DST_MIP_BINDING,
                                element_count: 1,
                                size_in_bytes: 0,
                                used_in_shader_stages: RafxShaderStageFlags::COMPUTE,
                                dx12_reg: Some(0),
                                dx12_space: Some(0),
                                ..Default::default()
                            },
                            RafxShaderResource {
                                name: Some("OutMip2".to_string()),
                                resource_type: RafxResourceType::TEXTURE_READ_WRITE,
                                set_index: 0,
                                binding: GENERATE_MIPS_DST_MIP_BINDING + 1,
                                element_count: 1,
                                size_in_bytes: 0,
                                used_in_shader_stages: RafxShaderStageFlags::COMPUTE,
                                dx12_reg: Some(1),
                                dx12_space: Some(0),
                                ..Default::default()
                            },
                            RafxShaderResource {
                                name: Some("OutMip3".to_string()),
                                resource_type: RafxResourceType::TEXTURE_READ_WRITE,
                                set_index: 0,
                                binding: GENERATE_MIPS_DST_MIP_BINDING + 2,
                                element_count: 1,
                                size_in_bytes: 0,
                                used_in_shader_stages: RafxShaderStageFlags::COMPUTE,
                                dx12_reg: Some(2),
                                dx12_space: Some(0),
                                ..Default::default()
                            },
                            RafxShaderResource {
                                name: Some("OutMip4".to_string()),
                                resource_type: RafxResourceType::TEXTURE_READ_WRITE,
                                set_index: 0,
                                binding: GENERATE_MIPS_DST_MIP_BINDING + 3,
                                element_count: 1,
                                size_in_bytes: 0,
                                used_in_shader_stages: RafxShaderStageFlags::COMPUTE,
                                dx12_reg: Some(3),
                                dx12_space: Some(0),
                                ..Default::default()
                            },
                        ],
                    }
                }])?);

                let sampler = RafxSampler::Dx12(dx12_device_context.create_sampler(&RafxSamplerDef {
                    min_filter: Default::default(),
                    mag_filter: Default::default(),
                    mip_map_mode: Default::default(),
                    address_mode_u: Default::default(),
                    address_mode_v: Default::default(),
                    address_mode_w: Default::default(),
                    mip_lod_bias: 0.0,
                    max_anisotropy: 0.0,
                    compare_op: Default::default(),
                })?);

                let root_signature = RafxRootSignature::Dx12(dx12_device_context.create_root_signature(&RafxRootSignatureDef {
                    immutable_samplers: &[
                        RafxImmutableSamplers {
                            key: RafxImmutableSamplerKey::Binding(0, GENERATE_MIPS_SAMPLER_BINDING),
                            samplers: &[sampler]
                        }
                    ],
                    shaders: &[shader.clone()]
                })?);

                let pipeline = RafxPipeline::Dx12(dx12_device_context.create_compute_pipeline(&RafxComputePipelineDef {
                    shader: &shader,
                    root_signature: &root_signature,
                })?);
        */
        Ok(Dx12MipmapResources {
            //shader,
            //root_signature,
            root_signature,
            pipeline,
        })
    }
}
