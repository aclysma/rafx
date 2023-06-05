use super::d3d12;
use crate::dx12::RafxDeviceContextDx12;
use crate::{
    RafxDescriptorIndex, RafxPipelineType, RafxResourceType, RafxResult, RafxRootSignatureDef,
    RafxSampler, RafxShaderStageFlags, ALL_SHADER_STAGE_FLAGS, MAX_DESCRIPTOR_SET_LAYOUTS,
};
use fnv::FnvHashMap;
use std::sync::Arc;

//TODO: Could compact this down quite a bit
#[derive(Clone, Debug)]
pub(crate) struct DescriptorInfo {
    pub(crate) name: Option<String>,
    pub(crate) resource_type: RafxResourceType,
    // Only valid for textures
    //pub(crate) texture_dimensions: Option<RafxTextureDimension>,

    // Also the set layout
    pub(crate) set_index: u32,
    // Binding within the set
    pub(crate) binding: u32,
    // Used for arrays of textures, samplers, etc.
    pub(crate) element_count: u32,

    pub(crate) push_constant_size: u32,
    pub(crate) used_in_shader_stages: RafxShaderStageFlags,

    // Index into DescriptorSetLayoutInfo::descriptors list
    // NOT THE BINDING INDEX!!!
    //pub(crate) descriptor_index: RafxDescriptorIndex,

    // --- dx12-specific ---
    pub(crate) register_space: u32,
    pub(crate) register: u32,

    // The index to the first descriptor in the flattened list of all descriptors in the layout
    // none for immutable samplers, which have no update data. This aligns with and will be less than
    // cbv_srv_uav_table_descriptor_count/sampler_table_descriptor_count etc.
    //TODO: I think we don't create DescriptorInfo for immutable samplers? So this is never none.
    pub(crate) update_data_offset_in_set: Option<u32>,
    //pub(crate) is_root_descriptor: bool,
    pub(crate) root_param_index: Option<u32>,
}

#[derive(Default, Debug)]
pub(crate) struct DescriptorSetLayoutInfo {
    // Settable descriptors, immutable samplers are omitted
    pub(crate) descriptors: Vec<RafxDescriptorIndex>,
    // Indexes binding index to the descriptors list
    pub(crate) binding_to_descriptor_index: FnvHashMap<u32, RafxDescriptorIndex>,

    // --- dx12-specific ---
    pub(crate) cbv_srv_uav_table: Vec<RafxDescriptorIndex>,
    pub(crate) sampler_table: Vec<RafxDescriptorIndex>,
    pub(crate) root_descriptors_params: Vec<RafxDescriptorIndex>,
    //pub(crate) root_constants: Vec<RafxDescriptorIndex>,

    //pub(crate) update_data_count: u32,

    // Total number of descriptors within the given tables. (An array counts as N descriptors)
    pub(crate) cbv_srv_uav_table_descriptor_count: Option<u32>,
    pub(crate) sampler_table_descriptor_count: Option<u32>,

    // The index of the table within the root descriptor
    pub(crate) cbv_srv_uav_table_root_index: Option<u8>,
    pub(crate) sampler_table_root_index: Option<u8>,
    //descriptor index map? map descriptor index to original shader resource?
}

#[derive(Debug)]
pub(crate) struct RafxRootSignatureDx12Inner {
    pub(crate) device_context: RafxDeviceContextDx12,
    pub(crate) pipeline_type: RafxPipelineType,
    pub(crate) layouts: [DescriptorSetLayoutInfo; MAX_DESCRIPTOR_SET_LAYOUTS],
    pub(crate) descriptors: Vec<DescriptorInfo>,
    pub(crate) name_to_descriptor_index: FnvHashMap<String, RafxDescriptorIndex>,
    pub(crate) push_constant_descriptors:
        [Option<RafxDescriptorIndex>; ALL_SHADER_STAGE_FLAGS.len()],
    // Keeps them in scope so they don't drop
    _immutable_samplers: Vec<RafxSampler>,

    // --- dx12-specific ---
    dx12_root_signature: d3d12::ID3D12RootSignature,
}

// for metal_rs::ArgumentDescriptor
unsafe impl Send for RafxRootSignatureDx12Inner {}
unsafe impl Sync for RafxRootSignatureDx12Inner {}

#[derive(Clone, Debug)]
pub struct RafxRootSignatureDx12 {
    pub(crate) inner: Arc<RafxRootSignatureDx12Inner>,
}

impl RafxRootSignatureDx12 {
    pub fn device_context(&self) -> &RafxDeviceContextDx12 {
        &self.inner.device_context
    }

    pub fn pipeline_type(&self) -> RafxPipelineType {
        self.inner.pipeline_type
    }

    pub fn dx12_root_signature(&self) -> &d3d12::ID3D12RootSignature {
        &self.inner.dx12_root_signature
    }

    pub fn find_descriptor_by_name(
        &self,
        name: &str,
    ) -> Option<RafxDescriptorIndex> {
        self.inner.name_to_descriptor_index.get(name).copied()
    }

    pub fn find_descriptor_by_binding(
        &self,
        set_index: u32,
        binding: u32,
    ) -> Option<RafxDescriptorIndex> {
        self.inner
            .layouts
            .get(set_index as usize)
            .and_then(|x| x.binding_to_descriptor_index.get(&binding))
            .copied()
    }

    pub fn find_push_constant_descriptor(
        &self,
        stage: RafxShaderStageFlags,
    ) -> Option<RafxDescriptorIndex> {
        let mut found_descriptor = None;
        println!("root constants {:?}", self.inner.push_constant_descriptors);
        for (stage_index, s) in ALL_SHADER_STAGE_FLAGS.iter().enumerate() {
            if s.intersects(stage) {
                let s_descriptor_index = self.inner.push_constant_descriptors[stage_index];
                if s_descriptor_index.is_some() {
                    if let Some(found_descriptor) = found_descriptor {
                        if found_descriptor != s_descriptor_index {
                            println!(
                                "Stages don't agree {:?} {:?}",
                                found_descriptor, s_descriptor_index
                            );
                            // The caller passed multiple stages and they do not use the same push constant descriptor
                            return None;
                        }
                    } else {
                        found_descriptor = Some(s_descriptor_index);
                    }
                }
            }
        }

        return found_descriptor.flatten();
    }

    pub(crate) fn descriptor(
        &self,
        descriptor_index: RafxDescriptorIndex,
    ) -> Option<&DescriptorInfo> {
        self.inner.descriptors.get(descriptor_index.0 as usize)
    }

    pub fn new(
        device_context: &RafxDeviceContextDx12,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<Self> {
        log::trace!("Create RafxRootSignatureDx12");

        // If we update this constant, update the arrays in this function
        assert_eq!(MAX_DESCRIPTOR_SET_LAYOUTS, 4);

        // Add a reference to all immutable samplers, we hold these so they aren't destroyed
        let mut immutable_samplers = vec![];
        for sampler_list in root_signature_def.immutable_samplers {
            for sampler in sampler_list.samplers {
                immutable_samplers.push(sampler.clone());
            }
        }

        // Make sure all shaders are compatible/build lookup of shared data from them
        let (pipeline_type, merged_resources, _merged_resources_name_index_map) =
            crate::internal_shared::merge_resources(root_signature_def)?;

        // merged_resources.sort_by(|lhs, rhs| {
        //     lhs.binding.cmp(&rhs.binding)
        // });

        let mut layouts = [
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
        ];

        let mut descriptors = Vec::with_capacity(merged_resources.len());
        let mut name_to_descriptor_index = FnvHashMap::default();

        let mut push_constant_descriptors = [None; ALL_SHADER_STAGE_FLAGS.len()];

        let mut static_samplers = vec![];

        let mut all_used_shader_stage = RafxShaderStageFlags::empty();
        let mut root_params = vec![];

        //
        // Iterate the resources and decide where they will go in the root signature
        // - push constants stored as values
        // - some uniform data can be promoted to root CBVs
        // - most data is assigned to a referenced table
        //
        for resource in &merged_resources {
            resource.validate()?;

            all_used_shader_stage |= resource.used_in_shader_stages;

            // Verify set index is valid
            let immutable_sampler = crate::internal_shared::find_immutable_sampler_index(
                root_signature_def.immutable_samplers,
                &resource.name,
                resource.set_index,
                resource.binding,
            );

            // Check that if an immutable sampler is set, the array size matches the resource element count
            if let Some(immutable_sampler_index) = immutable_sampler {
                if resource.element_count_normalized() as usize
                    != root_signature_def.immutable_samplers[immutable_sampler_index]
                        .samplers
                        .len()
                {
                    Err(format!(
                        "Descriptor (set={:?} binding={:?}) named {:?} specifies {} elements but the count of provided immutable samplers ({}) did not match",
                        resource.set_index,
                        resource.binding,
                        resource.name,
                        resource.element_count_normalized(),
                        root_signature_def.immutable_samplers[immutable_sampler_index].samplers.len()
                    ))?;
                }

                for sampler in
                    root_signature_def.immutable_samplers[immutable_sampler_index].samplers
                {
                    let d = sampler.dx12_sampler().unwrap().dx12_sampler_desc();
                    static_samplers.push(d3d12::D3D12_STATIC_SAMPLER_DESC {
                        Filter: d.Filter,
                        AddressU: d.AddressV,
                        AddressV: d.AddressV,
                        AddressW: d.AddressW,
                        MipLODBias: d.MipLODBias,
                        MaxAnisotropy: d.MaxAnisotropy,
                        ComparisonFunc: d.ComparisonFunc,
                        BorderColor: d3d12::D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK,
                        MinLOD: d.MinLOD,
                        MaxLOD: d.MaxLOD,
                        ShaderRegister: resource.dx12_reg.unwrap(),
                        RegisterSpace: resource.dx12_space.unwrap(),
                        ShaderVisibility: resource.used_in_shader_stages.into(),
                    });
                }

                // We don't need to do anything further with immutable samplers
                // Immutable samplers are omitted
                if !resource
                    .resource_type
                    .intersects(RafxResourceType::COMBINED_IMAGE_SAMPLER)
                {
                    continue;
                }
            } else {
                // combined image samplers are only supported with immutable samplers
                if resource
                    .resource_type
                    .intersects(RafxResourceType::COMBINED_IMAGE_SAMPLER)
                {
                    Err(format!(
                        "Descriptor (set={:?} binding={:?}) named {:?} is a combined image sampler but the sampler is NOT immutable. This is not supported. Use separate sampler/image bindings",
                        resource.set_index,
                        resource.binding,
                        resource.name
                    ))?;
                }
            }

            //TODO: Add some additional override? Or we just assume even though HLSL has no way to
            // indicate root constant, end-user has passed a resource type ROOT_CONSTANT
            // This may check being a uniform buffer and element count = 1
            let treat_as_root_constant = resource.resource_type == RafxResourceType::ROOT_CONSTANT;
            //TODO: Add some additional override? Make it a resource type?
            // This may check being a uniform buffer and element count = 1
            let treat_as_root_descriptor = false;

            if !treat_as_root_constant {
                let layout: &mut DescriptorSetLayoutInfo =
                    &mut layouts[resource.set_index as usize];

                let descriptor_index = RafxDescriptorIndex(descriptors.len() as u32);

                // let argument_buffer_id = next_argument_buffer_id[resource.set_index as usize];
                // next_argument_buffer_id[resource.set_index as usize] +=
                //     resource.element_count_normalized();

                //let update_data_offset_in_set = Some(layout.update_data_count_per_set);

                // Add it to the descriptor list
                descriptors.push(DescriptorInfo {
                    name: resource.name.clone(),
                    resource_type: resource.resource_type,
                    //texture_dimensions: resource.texture_dimensions,
                    set_index: resource.set_index,
                    binding: resource.binding,
                    element_count: resource.element_count_normalized(),
                    push_constant_size: 0,
                    used_in_shader_stages: resource.used_in_shader_stages,
                    register: resource.dx12_reg.unwrap(),
                    register_space: resource.dx12_space.unwrap(),
                    //shader_visibility: d3d12::D3D12_SHADER_VISIBILITY_ALL, //TODO: fix
                    update_data_offset_in_set: None, // gets set later
                    //is_root_descriptor: false,
                    root_param_index: None,
                });

                if let Some(name) = resource.name.as_ref() {
                    name_to_descriptor_index.insert(name.clone(), descriptor_index);
                }

                layout.descriptors.push(descriptor_index);
                layout
                    .binding_to_descriptor_index
                    .insert(resource.binding, descriptor_index);

                if resource.resource_type.intersects(RafxResourceType::SAMPLER) {
                    layout.sampler_table.push(descriptor_index);
                } else {
                    if treat_as_root_descriptor {
                        layout.root_descriptors_params.push(descriptor_index);
                    } else {
                        layout.cbv_srv_uav_table.push(descriptor_index);
                    }
                }
            } else {
                let descriptor_index = RafxDescriptorIndex(descriptors.len() as u32);
                descriptors.push(DescriptorInfo {
                    name: resource.name.clone(),
                    resource_type: resource.resource_type,
                    //texture_dimensions: resource.texture_dimensions,
                    set_index: u32::MAX,
                    binding: u32::MAX,
                    element_count: 0,
                    push_constant_size: resource.size_in_bytes,
                    used_in_shader_stages: resource.used_in_shader_stages,
                    register: u32::MAX,
                    register_space: u32::MAX,
                    //shader_visibility: d3d12::D3D12_SHADER_VISIBILITY_ALL, //TODO: fix
                    update_data_offset_in_set: None, // gets set later
                    //is_root_descriptor: true,
                    root_param_index: Some(root_params.len() as u32),
                });

                if let Some(name) = resource.name.as_ref() {
                    name_to_descriptor_index.insert(name.clone(), descriptor_index);
                }

                for (i, stage) in ALL_SHADER_STAGE_FLAGS.iter().enumerate() {
                    if stage.intersects(resource.used_in_shader_stages) {
                        push_constant_descriptors[i] = Some(descriptor_index);
                    }
                }

                let mut root_param = d3d12::D3D12_ROOT_PARAMETER1::default();
                root_param.ParameterType = d3d12::D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;
                root_param.ShaderVisibility = resource.used_in_shader_stages.into();
                root_param.Anonymous.Constants.RegisterSpace = resource.dx12_space.unwrap();
                root_param.Anonymous.Constants.ShaderRegister = resource.dx12_reg.unwrap();
                root_param.Anonymous.Constants.Num32BitValues =
                    rafx_base::memory::round_size_up_to_alignment_u32(resource.size_in_bytes, 4)
                        / 4;
                root_params.push(root_param);

                //TODO: Support promoting resources to be root constants
                // let layout: &mut DescriptorSetLayoutInfo =
                //     &mut layouts[resource.set_index as usize];
                //
                // layout.root_constants.push(descriptor_index);
            }
        }

        //
        // Set up immutable descriptors
        //
        let mut immutable_sampler_descs = Vec::with_capacity(immutable_samplers.len());
        for s in &immutable_samplers {
            immutable_sampler_descs.push(s.dx12_sampler().unwrap().dx12_sampler_desc());
        }

        // let mut root_param_count = 0;
        //
        // // One param per table
        // for i in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
        //     if !cbv_srv_uav_tables[i].is_empty() {
        //         root_param_count += 1;
        //     }
        //
        //     if !sampler_tables[i].is_empty() {
        //         root_param_count += 1;
        //     }
        // }

        //TODO: Add root descriptors and root constants

        // Preallocate rootParams, array of D3D12_ROOT_PARAMETER1, max number of root params supported
        // Preallocate cbvSrvUavRange, samplerRange, a block of D3D12_DESCRIPTOR_RANGE1 per layout
        // We already have immutable samplers prepared

        // array of D3D12_ROOT_PARAMETER1
        //let mut root_parameters = Vec::default();

        //
        // Add root descriptors to root_parameters
        //

        //
        // Add root constants to root_parameters
        //

        let mut cbv_srv_uav_ranges = [vec![], vec![], vec![], vec![]];
        let mut sampler_ranges = [vec![], vec![], vec![], vec![]];

        fn add_descriptor_table(
            descriptor_indices: &[RafxDescriptorIndex],
            descriptors: &mut [DescriptorInfo],
            ranges: &mut Vec<d3d12::D3D12_DESCRIPTOR_RANGE1>,
            shader_stages: &mut RafxShaderStageFlags,
        ) -> u32 {
            let mut total_descriptor_count = 0;
            for descriptor_index in descriptor_indices {
                let descriptor = &mut descriptors[descriptor_index.0 as usize];

                let mut descriptor_range = d3d12::D3D12_DESCRIPTOR_RANGE1::default();
                descriptor_range.BaseShaderRegister = descriptor.register;
                descriptor_range.RegisterSpace = descriptor.register_space;
                descriptor_range.Flags = d3d12::D3D12_DESCRIPTOR_RANGE_FLAG_NONE;
                descriptor_range.NumDescriptors = descriptor.element_count;
                descriptor_range.OffsetInDescriptorsFromTableStart =
                    d3d12::D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND;
                descriptor_range.RangeType =
                    super::internal::conversions::resource_type_descriptor_range_type(
                        descriptor.resource_type,
                    )
                    .unwrap();

                log::info!(
                    "  descriptor {:?}: space {} reg {} count {} shader stages {:?} resource type {:?} range type {:?}",
                    descriptor.name,
                    descriptor.register_space,
                    descriptor.register,
                    descriptor.element_count,
                    descriptor.used_in_shader_stages,
                    descriptor.resource_type,
                    descriptor_range.RangeType
                );

                ranges.push(descriptor_range);

                *shader_stages |= descriptor.used_in_shader_stages;

                descriptor.update_data_offset_in_set = Some(total_descriptor_count);
                // Already initialized to false
                //descriptor.is_root_descriptor = false;
                total_descriptor_count += descriptor.element_count;
            }

            total_descriptor_count
        }

        //
        // Add descriptor tables
        // - CbvSrvUav/Sampler tables are separate, each layout may make up to 2 tables
        //
        for layout_index in (0..MAX_DESCRIPTOR_SET_LAYOUTS).rev() {
            let layout = &mut layouts[layout_index];
            if !layout.cbv_srv_uav_table.is_empty() {
                log::info!(
                    "cbv_srv_uav_table for layout {} root index {}",
                    layout_index,
                    root_params.len() as u8
                );

                // create single D3D12_ROOT_PARAMETER1 for this table
                // create N D3D12_DESCRIPTOR_RANGE1 (could pre-allocate static array of them)
                let mut shader_stages = RafxShaderStageFlags::empty();
                // This will update descriptor's update_data_offset_in_set
                let total_descriptor_count = add_descriptor_table(
                    &layout.cbv_srv_uav_table,
                    &mut descriptors,
                    &mut cbv_srv_uav_ranges[layout_index],
                    &mut shader_stages,
                );

                let mut root_param = d3d12::D3D12_ROOT_PARAMETER1::default();
                root_param.ParameterType = d3d12::D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE;
                root_param.ShaderVisibility = shader_stages.into();
                root_param.Anonymous.DescriptorTable.pDescriptorRanges =
                    cbv_srv_uav_ranges[layout_index].as_ptr();
                root_param.Anonymous.DescriptorTable.NumDescriptorRanges =
                    cbv_srv_uav_ranges[layout_index].len() as u32;

                layout.cbv_srv_uav_table_descriptor_count = Some(total_descriptor_count);
                layout.cbv_srv_uav_table_root_index = Some(root_params.len() as u8);

                root_params.push(root_param);
            }

            if !layout.sampler_table.is_empty() {
                log::info!(
                    "sampler_table for layout {} root index {}",
                    layout_index,
                    root_params.len() as u8
                );

                // create single D3D12_ROOT_PARAMETER1 for this table
                // create N D3D12_DESCRIPTOR_RANGE1 (could pre-allocate static array of them)
                let mut shader_stages = RafxShaderStageFlags::empty();
                // This will update descriptor's update_data_offset_in_set
                let total_descriptor_count = add_descriptor_table(
                    &layout.cbv_srv_uav_table,
                    &mut descriptors,
                    &mut sampler_ranges[layout_index],
                    &mut shader_stages,
                );

                let mut root_param = d3d12::D3D12_ROOT_PARAMETER1::default();
                root_param.ParameterType = d3d12::D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE;
                root_param.ShaderVisibility = shader_stages.into();
                root_param.Anonymous.DescriptorTable.pDescriptorRanges =
                    sampler_ranges[layout_index].as_ptr();
                root_param.Anonymous.DescriptorTable.NumDescriptorRanges =
                    sampler_ranges[layout_index].len() as u32;

                layout.sampler_table_descriptor_count = Some(total_descriptor_count);
                layout.sampler_table_root_index = Some(root_params.len() as u8);

                root_params.push(root_param);
            }
        }

        let mut root_signature_flags = d3d12::D3D12_ROOT_SIGNATURE_FLAGS::default();
        root_signature_flags |= d3d12::D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
        if !all_used_shader_stage.intersects(RafxShaderStageFlags::VERTEX) {
            root_signature_flags |= d3d12::D3D12_ROOT_SIGNATURE_FLAG_DENY_VERTEX_SHADER_ROOT_ACCESS;
        }
        if !all_used_shader_stage.intersects(RafxShaderStageFlags::TESSELLATION_CONTROL) {
            root_signature_flags |= d3d12::D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS;
        }
        if !all_used_shader_stage.intersects(RafxShaderStageFlags::TESSELLATION_EVALUATION) {
            root_signature_flags |= d3d12::D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS;
        }
        if !all_used_shader_stage.intersects(RafxShaderStageFlags::GEOMETRY) {
            root_signature_flags |=
                d3d12::D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS;
        }
        if !all_used_shader_stage.intersects(RafxShaderStageFlags::FRAGMENT) {
            root_signature_flags |= d3d12::D3D12_ROOT_SIGNATURE_FLAG_DENY_PIXEL_SHADER_ROOT_ACCESS;
        }
        // There are other deny flags we could use?

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

        log::info!("created root signature {:?}", dx12_root_signature);

        let inner = RafxRootSignatureDx12Inner {
            device_context: device_context.clone(),
            pipeline_type,
            layouts,
            descriptors,
            name_to_descriptor_index,
            push_constant_descriptors,
            dx12_root_signature,
            _immutable_samplers: immutable_samplers,
        };

        Ok(RafxRootSignatureDx12 {
            inner: Arc::new(inner),
        })
    }
}
