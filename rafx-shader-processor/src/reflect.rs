use rafx_assets::assets::reflect::{
    ReflectedDescriptorSetLayout, ReflectedDescriptorSetLayoutBinding, ReflectedEntryPoint,
    ReflectedVertexInput,
};

use rafx_api::{
    RafxResourceType, RafxResult, RafxShaderResource, RafxShaderStageFlags,
    RafxShaderStageReflection,
};
use spirv_cross::spirv::Type;

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

fn get_binding_dsc<TargetT>(
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
    let set = ast
        .get_decoration(resource.id, spirv_cross::spirv::Decoration::DescriptorSet)
        .map_err(|_x| "could not get descriptor set index from reflection data")?;
    let binding = ast
        .get_decoration(resource.id, spirv_cross::spirv::Decoration::Binding)
        .map_err(|_x| "could not get descriptor binding index from reflection data")?;
    let element_count = get_descriptor_count_from_type(ast, resource.type_id)?;

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

    let slot_name = if let Some(annotation) = &parsed_binding.annotations.slot_name {
        Some(annotation.0.clone())
    } else {
        None
    };

    let rafx_resource = RafxShaderResource {
        name: slot_name.clone(),
        set_index: set,
        binding,
        size_in_bytes: 0, // Only for push constants
        resource_type,
        used_in_shader_stages: stage_flags,
        element_count,
    };

    Ok(ReflectedDescriptorSetLayoutBinding {
        resource: rafx_resource,
        internal_buffer_per_descriptor_size,
        immutable_samplers,
    })
}

fn get_binding_rafx<TargetT>(
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
    let name = &resource.name;
    let set = ast
        .get_decoration(resource.id, spirv_cross::spirv::Decoration::DescriptorSet)
        .map_err(|_x| "could not get descriptor set index from reflection data")?;
    let binding = ast
        .get_decoration(resource.id, spirv_cross::spirv::Decoration::Binding)
        .map_err(|_x| "could not get descriptor binding index from reflection data")?;
    let element_count = get_descriptor_count_from_type(ast, resource.type_id)?;

    let parsed_binding = declarations.bindings.iter().find(|x| x.parsed.layout_parts.binding == Some(binding as usize) && x.parsed.layout_parts.set == Some(set as usize))
        .or_else(|| declarations.bindings.iter().find(|x| x.parsed.instance_name == *name))
        .ok_or_else(|| format!("A resource named {} in spirv reflection data was not matched up to a resource scanned in source code.", resource.name))?;

    let slot_name = if let Some(annotation) = &parsed_binding.annotations.slot_name {
        Some(annotation.0.clone())
    } else {
        None
    };

    let resource = RafxShaderResource {
        resource_type,
        set_index: set,
        binding,
        element_count,
        size_in_bytes: 0,
        used_in_shader_stages: stage_flags,
        name: Some(slot_name.unwrap_or_else(|| name.clone())),
    };

    resource.validate()?;

    Ok(resource)
}

fn get_bindings_dsc<TargetT>(
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
        descriptors.push(get_binding_dsc(
            ast,
            declarations,
            resource,
            resource_type,
            stage_flags,
        )?);
    }

    Ok(())
}

fn get_bindings_rafx<TargetT>(
    descriptors: &mut Vec<RafxShaderResource>,
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
        descriptors.push(get_binding_rafx(
            ast,
            declarations,
            resource,
            resource_type,
            stage_flags,
        )?);
    }

    Ok(())
}

fn get_all_bindings_dsc<TargetT>(
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
    get_bindings_dsc(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.uniform_buffers,
        RafxResourceType::UNIFORM_BUFFER,
        stage_flags,
    )?;
    get_bindings_dsc(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.storage_buffers,
        RafxResourceType::BUFFER,
        stage_flags,
    )?;
    get_bindings_dsc(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.storage_images,
        RafxResourceType::TEXTURE_READ_WRITE,
        stage_flags,
    )?;
    get_bindings_dsc(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.sampled_images,
        RafxResourceType::COMBINED_IMAGE_SAMPLER,
        stage_flags,
    )?;
    get_bindings_dsc(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.separate_images,
        RafxResourceType::TEXTURE,
        stage_flags,
    )?;
    get_bindings_dsc(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.separate_samplers,
        RafxResourceType::SAMPLER,
        stage_flags,
    )?;

    Ok(bindings)
}

fn get_all_bindings_rafx<TargetT>(
    shader_resources: &spirv_cross::spirv::ShaderResources,
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    stage_flags: RafxShaderStageFlags,
) -> RafxResult<Vec<RafxShaderResource>>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    let mut bindings = Vec::default();
    get_bindings_rafx(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.uniform_buffers,
        RafxResourceType::UNIFORM_BUFFER,
        stage_flags,
    )?;
    get_bindings_rafx(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.storage_buffers,
        RafxResourceType::BUFFER_READ_WRITE,
        stage_flags,
    )?;
    get_bindings_rafx(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.storage_images,
        RafxResourceType::TEXTURE_READ_WRITE,
        stage_flags,
    )?;
    get_bindings_rafx(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.sampled_images,
        RafxResourceType::COMBINED_IMAGE_SAMPLER,
        stage_flags,
    )?;
    get_bindings_rafx(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.separate_images,
        RafxResourceType::TEXTURE,
        stage_flags,
    )?;
    get_bindings_rafx(
        &mut bindings,
        ast,
        declarations,
        &shader_resources.separate_samplers,
        RafxResourceType::SAMPLER,
        stage_flags,
    )?;

    Ok(bindings)
}

pub(crate) fn reflect_data<TargetT>(
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
) -> RafxResult<Vec<ReflectedEntryPoint>>
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

        let dsc_bindings = get_all_bindings_dsc(&shader_resources, ast, declarations, stage_flags)?;

        let mut rafx_bindings =
            get_all_bindings_rafx(&shader_resources, ast, declarations, stage_flags)?;

        // stage inputs
        // stage outputs
        // subpass inputs
        // atomic counters
        // push constant buffers

        let mut descriptor_set_layouts: Vec<Option<ReflectedDescriptorSetLayout>> = vec![];
        for binding in dsc_bindings {
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

                let semantic = &parsed_binding.annotations.semantic.as_ref().map(|x| x.0.clone())
                    .ok_or_else(|| format!("No semantic annotation for vertex input '{}'. All vertex inputs must have a semantic annotation if generating rust code and/or cooked shaders.", name))?;

                dsc_vertex_inputs.push(ReflectedVertexInput {
                    name: name.clone(),
                    semantic: semantic.clone(),
                    location,
                });
            }
        }

        let rafx_reflection = RafxShaderStageReflection {
            shader_stage: stage_flags,
            resources: rafx_bindings,
            entry_point_name: entry_point_name.clone(),
            thread_count: [
                entry_point.work_group_size.x,
                entry_point.work_group_size.y,
                entry_point.work_group_size.z,
            ],
        };

        reflected_entry_points.push(ReflectedEntryPoint {
            descriptor_set_layouts,
            vertex_inputs: dsc_vertex_inputs,
            rafx_reflection,
        });
    }

    Ok(reflected_entry_points)
}

fn map_shader_stage_flags(
    shader_stage: spirv_cross::spirv::ExecutionModel
) -> RafxResult<RafxShaderStageFlags> {
    use spirv_cross::spirv::ExecutionModel;
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
