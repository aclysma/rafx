use rafx_framework::cooked_shader::{
    ReflectedDescriptorSetLayout, ReflectedDescriptorSetLayoutBinding, ReflectedEntryPoint,
    ReflectedVertexInput,
};

use crate::shader_types::{
    element_count, generate_struct, MemoryLayout, TypeAlignmentInfo, UserType,
};
use fnv::FnvHashMap;
use rafx_api::{
    RafxAddressMode, RafxCompareOp, RafxFilterType, RafxGlUniformMember, RafxMipMapMode,
    RafxResourceType, RafxResult, RafxSamplerDef, RafxShaderResource, RafxShaderStageFlags,
    RafxShaderStageReflection, MAX_DESCRIPTOR_SET_LAYOUTS,
};
use spirv_cross::msl::{ResourceBinding, ResourceBindingLocation, SamplerData, SamplerLocation};
use spirv_cross::spirv::{ExecutionModel, Type};
use std::collections::BTreeMap;

fn get_descriptor_count_from_type<TargetT>(
    ast: &spirv_cross::spirv::Ast<TargetT>,
    ty: u32,
) -> RafxResult<u32>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    fn count_elements(a: &[u32]) -> u32 {
        let mut count = 1;
        for x in a {
            count *= x;
        }

        count
    }

    Ok(
        match ast
            .get_type(ty)
            .map_err(|_x| "could not get type from reflection data")?
        {
            Type::Unknown => 0,
            Type::Void => 0,
            Type::Boolean { array, .. } => count_elements(&array),
            Type::Char { array, .. } => count_elements(&array),
            Type::Int { array, .. } => count_elements(&array),
            Type::UInt { array, .. } => count_elements(&array),
            Type::Int64 { array, .. } => count_elements(&array),
            Type::UInt64 { array, .. } => count_elements(&array),
            Type::AtomicCounter { array, .. } => count_elements(&array),
            Type::Half { array, .. } => count_elements(&array),
            Type::Float { array, .. } => count_elements(&array),
            Type::Double { array, .. } => count_elements(&array),
            Type::Struct { array, .. } => count_elements(&array),
            Type::Image { array, .. } => count_elements(&array),
            Type::SampledImage { array, .. } => count_elements(&array),
            Type::Sampler { array, .. } => count_elements(&array),
            Type::SByte { array, .. } => count_elements(&array),
            Type::UByte { array, .. } => count_elements(&array),
            Type::Short { array, .. } => count_elements(&array),
            Type::UShort { array, .. } => count_elements(&array),
            Type::ControlPointArray => 1,
            Type::AccelerationStructure => 1,
            Type::RayQuery => 0,
            _ => unimplemented!(),
        },
    )
}

fn get_descriptor_size_from_resource_rafx<TargetT>(
    ast: &spirv_cross::spirv::Ast<TargetT>,
    resource: &spirv_cross::spirv::Resource,
    resource_type: RafxResourceType,
) -> RafxResult<u32>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    Ok(
        if resource_type.intersects(
            RafxResourceType::UNIFORM_BUFFER
                | RafxResourceType::BUFFER
                | RafxResourceType::BUFFER_READ_WRITE,
        ) {
            (ast.get_declared_struct_size(resource.type_id)
                .map_err(|_x| "could not get size from reflection data")?
                + 15)
                / 16
                * 16
        } else {
            0
        },
    )
}

fn get_rafx_resource<TargetT>(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    resource: &spirv_cross::spirv::Resource,
    resource_type: RafxResourceType,
    stage_flags: RafxShaderStageFlags,
) -> RafxResult<RafxShaderResource>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    let set = ast
        .get_decoration(resource.id, spirv_cross::spirv::Decoration::DescriptorSet)
        .map_err(|_x| "could not get descriptor set index from reflection data")?;
    let binding = ast
        .get_decoration(resource.id, spirv_cross::spirv::Decoration::Binding)
        .map_err(|_x| "could not get descriptor binding index from reflection data")?;
    let element_count = get_descriptor_count_from_type(ast, resource.type_id)?;

    let parsed_binding = declarations.bindings.iter().find(|x| x.parsed.layout_parts.binding == Some(binding as usize) && x.parsed.layout_parts.set == Some(set as usize))
        .or_else(|| declarations.bindings.iter().find(|x| x.parsed.instance_name == *resource.name))
        .ok_or_else(|| format!("A resource named {} in spirv reflection data was not matched up to a resource scanned in source code.", resource.name))?;

    let slot_name = if let Some(annotation) = &parsed_binding.annotations.slot_name {
        Some(annotation.0.clone())
    } else {
        None
    };

    let mut gl_uniform_members = Vec::<RafxGlUniformMember>::default();
    if resource_type == RafxResourceType::UNIFORM_BUFFER {
        generate_gl_uniform_members(
            &builtin_types,
            &user_types,
            &parsed_binding.parsed.type_name,
            parsed_binding.parsed.type_name.clone(),
            0,
            &mut gl_uniform_members,
        )?;
    }

    let gles_name = if resource_type == RafxResourceType::UNIFORM_BUFFER {
        parsed_binding.parsed.type_name.clone()
    } else {
        parsed_binding.parsed.instance_name.clone()
    };

    let resource = RafxShaderResource {
        resource_type,
        set_index: set,
        binding,
        element_count,
        size_in_bytes: 0,
        used_in_shader_stages: stage_flags,
        name: Some(slot_name.unwrap_or_else(|| resource.name.clone())),
        gles_name: Some(gles_name),
        gles_sampler_name: None, // This is set later if necessary when we cross compile GLES 2.0 src by set_gl_sampler_name
        gles2_uniform_members: gl_uniform_members,
    };

    resource.validate()?;

    Ok(resource)
}

fn get_reflected_binding<TargetT>(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    resource: &spirv_cross::spirv::Resource,
    resource_type: RafxResourceType,
    stage_flags: RafxShaderStageFlags,
) -> RafxResult<ReflectedDescriptorSetLayoutBinding>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    let name = &resource.name;
    let rafx_resource = get_rafx_resource(
        builtin_types,
        user_types,
        ast,
        declarations,
        resource,
        resource_type,
        stage_flags,
    )?;
    let set = rafx_resource.set_index;
    let binding = rafx_resource.binding;

    let parsed_binding = declarations.bindings.iter().find(|x| x.parsed.layout_parts.binding == Some(binding as usize) && x.parsed.layout_parts.set == Some(set as usize))
        .or_else(|| declarations.bindings.iter().find(|x| x.parsed.instance_name == *name))
        .ok_or_else(|| format!("A resource named {} in spirv reflection data was not matched up to a resource scanned in source code.", resource.name))?;

    let size = get_descriptor_size_from_resource_rafx(ast, resource, resource_type)
        .map_err(|_x| "could not get size from reflection data")?;

    let internal_buffer_per_descriptor_size =
        if parsed_binding.annotations.use_internal_buffer.is_some() {
            Some(size)
        } else {
            None
        };

    let immutable_samplers =
        if let Some(annotation) = &parsed_binding.annotations.immutable_samplers {
            Some(annotation.0.clone())
        } else {
            None
        };

    Ok(ReflectedDescriptorSetLayoutBinding {
        resource: rafx_resource,
        internal_buffer_per_descriptor_size,
        immutable_samplers,
    })
}

fn get_reflected_bindings<TargetT>(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    descriptors: &mut Vec<ReflectedDescriptorSetLayoutBinding>,
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    resources: &[spirv_cross::spirv::Resource],
    resource_type: RafxResourceType,
    stage_flags: RafxShaderStageFlags,
) -> RafxResult<()>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    for resource in resources {
        descriptors.push(get_reflected_binding(
            builtin_types,
            user_types,
            ast,
            declarations,
            resource,
            resource_type,
            stage_flags,
        )?);
    }

    Ok(())
}

fn get_all_reflected_bindings<TargetT>(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    shader_resources: &spirv_cross::spirv::ShaderResources,
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    stage_flags: RafxShaderStageFlags,
) -> RafxResult<Vec<ReflectedDescriptorSetLayoutBinding>>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    let mut bindings = Vec::default();
    get_reflected_bindings(
        builtin_types,
        user_types,
        &mut bindings,
        ast,
        declarations,
        &shader_resources.uniform_buffers,
        RafxResourceType::UNIFORM_BUFFER,
        stage_flags,
    )?;
    get_reflected_bindings(
        builtin_types,
        user_types,
        &mut bindings,
        ast,
        declarations,
        &shader_resources.storage_buffers,
        RafxResourceType::BUFFER,
        stage_flags,
    )?;
    get_reflected_bindings(
        builtin_types,
        user_types,
        &mut bindings,
        ast,
        declarations,
        &shader_resources.storage_images,
        RafxResourceType::TEXTURE_READ_WRITE,
        stage_flags,
    )?;
    get_reflected_bindings(
        builtin_types,
        user_types,
        &mut bindings,
        ast,
        declarations,
        &shader_resources.sampled_images,
        RafxResourceType::COMBINED_IMAGE_SAMPLER,
        stage_flags,
    )?;
    get_reflected_bindings(
        builtin_types,
        user_types,
        &mut bindings,
        ast,
        declarations,
        &shader_resources.separate_images,
        RafxResourceType::TEXTURE,
        stage_flags,
    )?;
    get_reflected_bindings(
        builtin_types,
        user_types,
        &mut bindings,
        ast,
        declarations,
        &shader_resources.separate_samplers,
        RafxResourceType::SAMPLER,
        stage_flags,
    )?;

    Ok(bindings)
}

//TODO: Exclude MSL constexpr samplers?
pub(crate) fn msl_assign_argument_buffer_ids(
    entry_points: &[ReflectedEntryPoint]
) -> RafxResult<BTreeMap<ResourceBindingLocation, ResourceBinding>> {
    let mut all_resources_lookup = FnvHashMap::<(u32, u32), RafxShaderResource>::default();
    for entry_point in entry_points {
        for resource in &entry_point.rafx_api_reflection.resources {
            let key = (resource.set_index, resource.binding);
            if let Some(old) = all_resources_lookup.get_mut(&key) {
                if resource.resource_type != old.resource_type {
                    Err(format!(
                        "Shaders with same set and binding {:?} have mismatching resource types {:?} and {:?}",
                        key,
                        resource.resource_type,
                        old.resource_type
                    ))?;
                }

                if resource.element_count_normalized() != old.element_count_normalized() {
                    Err(format!(
                        "Shaders with same set and binding {:?} have mismatching element counts {:?} and {:?}",
                        key,
                        resource.element_count_normalized(),
                        old.element_count_normalized()
                    ))?;
                }

                old.used_in_shader_stages |= resource.used_in_shader_stages;
            } else {
                all_resources_lookup.insert(key, resource.clone());
            }
        }
    }

    let mut resources: Vec<_> = all_resources_lookup.values().collect();
    resources.sort_by(|lhs, rhs| lhs.binding.cmp(&rhs.binding));

    // If we update this constant, update the arrays in this function
    assert_eq!(MAX_DESCRIPTOR_SET_LAYOUTS, 4);

    let mut next_msl_argument_buffer_id = [0, 0, 0, 0];

    let mut argument_buffer_assignments =
        BTreeMap::<ResourceBindingLocation, ResourceBinding>::default();

    for resource in resources {
        let msl_argument_buffer_id = next_msl_argument_buffer_id[resource.set_index as usize];

        let location = ResourceBindingLocation {
            // We'll overwrite the stage as needed when we insert into the map
            stage: spirv_cross::spirv::ExecutionModel::TessellationEvaluation,
            desc_set: resource.set_index,
            binding: resource.binding,
        };

        let new_binding = ResourceBinding {
            buffer_id: msl_argument_buffer_id,
            texture_id: msl_argument_buffer_id,
            sampler_id: msl_argument_buffer_id,
            count: resource.element_count_normalized(),
        };

        if resource
            .used_in_shader_stages
            .intersects(RafxShaderStageFlags::VERTEX)
        {
            let mut location = location.clone();
            location.stage = ExecutionModel::Vertex;
            argument_buffer_assignments.insert(location, new_binding.clone());
        }

        if resource
            .used_in_shader_stages
            .intersects(RafxShaderStageFlags::FRAGMENT)
        {
            let mut location = location.clone();
            location.stage = ExecutionModel::Fragment;
            argument_buffer_assignments.insert(location, new_binding.clone());
        }

        if resource
            .used_in_shader_stages
            .intersects(RafxShaderStageFlags::COMPUTE)
        {
            let mut location = location.clone();
            location.stage = ExecutionModel::Kernel;
            argument_buffer_assignments.insert(location, new_binding.clone());
        }

        if resource
            .used_in_shader_stages
            .intersects(RafxShaderStageFlags::TESSELLATION_CONTROL)
        {
            let mut location = location.clone();
            location.stage = ExecutionModel::TessellationControl;
            argument_buffer_assignments.insert(location, new_binding.clone());
        }

        if resource
            .used_in_shader_stages
            .intersects(RafxShaderStageFlags::TESSELLATION_EVALUATION)
        {
            let mut location = location.clone();
            location.stage = ExecutionModel::TessellationEvaluation;
            argument_buffer_assignments.insert(location, new_binding.clone());
        }

        next_msl_argument_buffer_id[resource.set_index as usize] +=
            resource.element_count_normalized();
    }

    Ok(argument_buffer_assignments)
}

fn msl_create_sampler_data(
    sampler_def: &RafxSamplerDef
) -> RafxResult<spirv_cross::msl::SamplerData> {
    let lod_clamp_min = LodBase16::from(sampler_def.mip_lod_bias);
    let lod_clamp_max = if sampler_def.mip_map_mode == RafxMipMapMode::Linear {
        LodBase16::MAX
    } else {
        LodBase16::ZERO
    };

    fn convert_filter(filter: RafxFilterType) -> SamplerFilter {
        match filter {
            RafxFilterType::Nearest => SamplerFilter::Nearest,
            RafxFilterType::Linear => SamplerFilter::Linear,
        }
    }

    fn convert_mip_map_mode(mip_map_mode: RafxMipMapMode) -> SamplerMipFilter {
        match mip_map_mode {
            RafxMipMapMode::Nearest => SamplerMipFilter::Nearest,
            RafxMipMapMode::Linear => SamplerMipFilter::Linear,
        }
    }

    fn convert_address_mode(address_mode: RafxAddressMode) -> SamplerAddress {
        match address_mode {
            RafxAddressMode::Mirror => SamplerAddress::MirroredRepeat,
            RafxAddressMode::Repeat => SamplerAddress::Repeat,
            RafxAddressMode::ClampToEdge => SamplerAddress::ClampToEdge,
            RafxAddressMode::ClampToBorder => SamplerAddress::ClampToBorder,
        }
    }

    fn convert_compare_op(compare_op: RafxCompareOp) -> SamplerCompareFunc {
        match compare_op {
            RafxCompareOp::Never => SamplerCompareFunc::Never,
            RafxCompareOp::Less => SamplerCompareFunc::Less,
            RafxCompareOp::Equal => SamplerCompareFunc::Equal,
            RafxCompareOp::LessOrEqual => SamplerCompareFunc::LessEqual,
            RafxCompareOp::Greater => SamplerCompareFunc::Greater,
            RafxCompareOp::NotEqual => SamplerCompareFunc::NotEqual,
            RafxCompareOp::GreaterOrEqual => SamplerCompareFunc::GreaterEqual,
            RafxCompareOp::Always => SamplerCompareFunc::Always,
        }
    }

    let max_anisotropy = if sampler_def.max_anisotropy == 0.0 {
        1
    } else {
        sampler_def.max_anisotropy as i32
    };

    use spirv_cross::msl::*;
    let sampler_data = SamplerData {
        coord: SamplerCoord::Normalized,
        min_filter: convert_filter(sampler_def.min_filter),
        mag_filter: convert_filter(sampler_def.mag_filter),
        mip_filter: convert_mip_map_mode(sampler_def.mip_map_mode),
        s_address: convert_address_mode(sampler_def.address_mode_u),
        t_address: convert_address_mode(sampler_def.address_mode_v),
        r_address: convert_address_mode(sampler_def.address_mode_w),
        compare_func: convert_compare_op(sampler_def.compare_op),
        border_color: SamplerBorderColor::TransparentBlack,
        lod_clamp_min,
        lod_clamp_max,
        max_anisotropy,

        // Sampler YCbCr conversion parameters
        planes: 0,
        resolution: FormatResolution::_444,
        chroma_filter: SamplerFilter::Nearest,
        x_chroma_offset: ChromaLocation::CositedEven,
        y_chroma_offset: ChromaLocation::CositedEven,
        swizzle: [
            ComponentSwizzle::Identity,
            ComponentSwizzle::Identity,
            ComponentSwizzle::Identity,
            ComponentSwizzle::Identity,
        ],
        ycbcr_conversion_enable: false,
        ycbcr_model: SamplerYCbCrModelConversion::RgbIdentity,
        ycbcr_range: SamplerYCbCrRange::ItuFull,
        bpc: 8,
    };

    Ok(sampler_data)
}

pub(crate) fn msl_const_samplers(
    entry_points: &[ReflectedEntryPoint],
    //msl_argument_buffer_assignments: &BTreeMap::<ResourceBindingLocation, ResourceBinding>,
) -> RafxResult<BTreeMap<SamplerLocation, SamplerData>> {
    let mut immutable_samplers = BTreeMap::<SamplerLocation, SamplerData>::default();

    for entry_point in entry_points {
        for layout in &entry_point.descriptor_set_layouts {
            if let Some(layout) = layout {
                for binding in &layout.bindings {
                    if let Some(immutable_sampler) = &binding.immutable_samplers {
                        let location = SamplerLocation {
                            desc_set: binding.resource.set_index,
                            binding: binding.resource.binding,
                        };

                        if immutable_sampler.len() > 1 {
                            Err(format!("Multiple immutable samplers in a single binding ({:?}) not supported in MSL", location))?;
                        }
                        let immutable_sampler = immutable_sampler.first().unwrap();

                        let sampler_data = msl_create_sampler_data(&immutable_sampler)?;

                        if let Some(old) = immutable_samplers.get(&location) {
                            if *old != sampler_data {
                                Err(format!("Samplers in different entry points but same location ({:?}) do not match: \n{:#?}\n{:#?}", location, old, sampler_data))?;
                            }
                        } else {
                            immutable_samplers.insert(location, sampler_data);
                        }
                    }
                }
            }
        }
    }

    Ok(immutable_samplers)
}

fn generate_gl_uniform_members(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    type_name: &str,
    prefix: String,
    offset: usize,
    gl_uniform_members: &mut Vec<RafxGlUniformMember>,
) -> RafxResult<()> {
    if builtin_types.contains_key(type_name) {
        //println!("{} at {}: {}", prefix, offset, type_name);
        gl_uniform_members.push(RafxGlUniformMember {
            name: prefix,
            offset: offset as u32,
        })
    } else {
        let user_type = user_types.get(type_name).ok_or_else(|| {
            format!(
                "Could not find type named {} in generate_gl_uniform_members",
                type_name
            )
        })?;

        let generated_struct = generate_struct(
            builtin_types,
            user_types,
            &user_type.type_name,
            user_type,
            MemoryLayout::Std140,
        )?;

        for field in &*user_type.fields {
            let struct_member = generated_struct
                .members
                .iter()
                .find(|x| x.name == field.field_name)
                .ok_or_else(|| {
                    format!(
                        "Could not find member {} within generated struct {}",
                        field.field_name, generated_struct.name
                    )
                })?;

            if field.array_sizes.is_empty() {
                let member_full_name = format!("{}.{}", prefix, field.field_name);
                let field_offset = offset + struct_member.offset;
                generate_gl_uniform_members(
                    builtin_types,
                    user_types,
                    &field.type_name,
                    member_full_name,
                    field_offset,
                    gl_uniform_members,
                )?;
            } else {
                let element_count = element_count(&field.array_sizes);
                for i in 0..element_count {
                    let member_full_name = format!("{}.{}[{}]", prefix, field.field_name, i);
                    let field_offset =
                        offset + struct_member.offset + (i * struct_member.size / element_count);
                    generate_gl_uniform_members(
                        builtin_types,
                        user_types,
                        &field.type_name,
                        member_full_name,
                        field_offset,
                        gl_uniform_members,
                    )?;
                }
            }
        }
    }

    Ok(())
}

pub struct ShaderProcessorRefectionData {
    pub reflection: Vec<ReflectedEntryPoint>,
    pub msl_argument_buffer_assignments: BTreeMap<ResourceBindingLocation, ResourceBinding>,
    pub msl_const_samplers: BTreeMap<SamplerLocation, SamplerData>,
}

impl ShaderProcessorRefectionData {
    // GL ES 2.0 attaches sampler state to textures. So every texture must be associated with a
    // single sampler. This function is called when cross-compiling to GL ES 2.0 to set
    // gl_sampler_name on all texture resources.
    pub fn set_gl_sampler_name(
        &mut self,
        texture_gl_name: &str,
        sampler_gl_name: &str,
    ) {
        for entry_point in &mut self.reflection {
            for resource in &mut entry_point.rafx_api_reflection.resources {
                if resource.gles_name.as_ref().unwrap().as_str() == texture_gl_name {
                    assert!(resource.resource_type.intersects(
                        RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE
                    ));
                    resource.gles_sampler_name = Some(sampler_gl_name.to_string());
                }
            }

            for layout in &mut entry_point.descriptor_set_layouts {
                if let Some(layout) = layout {
                    for resource in &mut layout.bindings {
                        if resource.resource.gles_name.as_ref().unwrap().as_str() == texture_gl_name
                        {
                            assert!(resource.resource.resource_type.intersects(
                                RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE
                            ));
                            resource.resource.gles_sampler_name = Some(sampler_gl_name.to_string());
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn reflect_data<TargetT>(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    require_semantics: bool,
) -> RafxResult<ShaderProcessorRefectionData>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    let mut reflected_entry_points = Vec::default();
    for entry_point in ast
        .get_entry_points()
        .map_err(|_x| "could not get entry point from reflection data")?
    {
        let entry_point_name = entry_point.name;
        let stage_flags = map_shader_stage_flags(entry_point.execution_model)?;

        let shader_resources = ast
            .get_shader_resources()
            .map_err(|_x| "could not get resources from reflection data")?;

        let dsc_bindings = get_all_reflected_bindings(
            builtin_types,
            user_types,
            &shader_resources,
            ast,
            declarations,
            stage_flags,
        )?;

        // stage inputs
        // stage outputs
        // subpass inputs
        // atomic counters
        // push constant buffers

        let mut descriptor_set_layouts: Vec<Option<ReflectedDescriptorSetLayout>> = vec![];
        let mut rafx_bindings = Vec::default();
        for binding in dsc_bindings {
            rafx_bindings.push(binding.resource.clone());

            while descriptor_set_layouts.len() <= binding.resource.set_index as usize {
                descriptor_set_layouts.push(None);
            }

            match &mut descriptor_set_layouts[binding.resource.set_index as usize] {
                Some(x) => x.bindings.push(binding),
                x @ None => {
                    *x = Some(ReflectedDescriptorSetLayout {
                        bindings: vec![binding],
                    })
                }
            }
        }

        //TODO: This is using a list of push constants but I don't think multiple are allowed within
        // the same file
        for push_constant in &shader_resources.push_constant_buffers {
            let push_constant_ranges = ast
                .get_active_buffer_ranges(push_constant.id)
                .map_err(|_x| "could not get active buffer ranges")?;
            for push_constant_range in &push_constant_ranges {
                let resource = RafxShaderResource {
                    resource_type: RafxResourceType::ROOT_CONSTANT,
                    size_in_bytes: push_constant_range.range as u32,
                    used_in_shader_stages: stage_flags,
                    name: Some(push_constant.name.clone()),
                    ..Default::default()
                };
                resource.validate()?;

                rafx_bindings.push(resource);
            }
        }

        //TODO: Store the type and verify that the format associated in the game i.e. R32G32B32 is
        // something reasonable (like vec3).
        let mut dsc_vertex_inputs = Vec::default();
        if entry_point.execution_model == spirv_cross::spirv::ExecutionModel::Vertex {
            for resource in shader_resources.stage_inputs {
                let name = &resource.name;
                let location = ast
                    .get_decoration(resource.id, spirv_cross::spirv::Decoration::Location)
                    .map_err(|_x| "could not get descriptor binding index from reflection data")?;

                let parsed_binding = declarations.bindings.iter().find(|x| x.parsed.layout_parts.location == Some(location as usize))
                    .or_else(|| declarations.bindings.iter().find(|x| x.parsed.instance_name == *name))
                    .ok_or_else(|| format!("A resource named {} in spirv reflection data was not matched up to a resource scanned in source code.", resource.name))?;

                let semantic = &parsed_binding
                    .annotations
                    .semantic
                    .as_ref()
                    .map(|x| x.0.clone());

                let semantic = if require_semantics {
                    semantic.clone().ok_or_else(|| format!("No semantic annotation for vertex input '{}'. All vertex inputs must have a semantic annotation if generating rust code and/or cooked shaders.", name))?
                } else {
                    "".to_string()
                };

                dsc_vertex_inputs.push(ReflectedVertexInput {
                    name: name.clone(),
                    semantic,
                    location,
                });
            }
        }

        let rafx_reflection = RafxShaderStageReflection {
            shader_stage: stage_flags,
            resources: rafx_bindings,
            entry_point_name: entry_point_name.clone(),
            compute_threads_per_group: Some([
                entry_point.work_group_size.x,
                entry_point.work_group_size.y,
                entry_point.work_group_size.z,
            ]),
        };

        reflected_entry_points.push(ReflectedEntryPoint {
            descriptor_set_layouts,
            vertex_inputs: dsc_vertex_inputs,
            rafx_api_reflection: rafx_reflection,
        });
    }

    let msl_argument_buffer_assignments = msl_assign_argument_buffer_ids(&reflected_entry_points)?;

    let msl_const_samplers = msl_const_samplers(&reflected_entry_points)?;

    Ok(ShaderProcessorRefectionData {
        reflection: reflected_entry_points,
        msl_argument_buffer_assignments,
        msl_const_samplers,
    })
}

fn map_shader_stage_flags(
    shader_stage: spirv_cross::spirv::ExecutionModel
) -> RafxResult<RafxShaderStageFlags> {
    Ok(match shader_stage {
        ExecutionModel::Vertex => RafxShaderStageFlags::VERTEX,
        ExecutionModel::TessellationControl => RafxShaderStageFlags::TESSELLATION_CONTROL,
        ExecutionModel::TessellationEvaluation => RafxShaderStageFlags::TESSELLATION_EVALUATION,
        ExecutionModel::Geometry => RafxShaderStageFlags::GEOMETRY,
        ExecutionModel::Fragment => RafxShaderStageFlags::FRAGMENT,
        ExecutionModel::GlCompute => RafxShaderStageFlags::COMPUTE,
        ExecutionModel::Kernel => RafxShaderStageFlags::COMPUTE,
    })
}
