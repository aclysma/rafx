use renderer_assets::assets::reflect::{
    ReflectedDescriptorSetLayout, ReflectedDescriptorSetLayoutBinding, ReflectedEntryPoint,
    ReflectedPushConstant, ReflectedVertexInput,
};

use renderer_resources::vk_description as dsc;
use spirv_cross::spirv::Type;

fn get_descriptor_count_from_type<TargetT>(
    ast: &spirv_cross::spirv::Ast<TargetT>,
    ty: u32,
) -> Result<u32, String>
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

fn get_descriptor_size_from_resource<TargetT>(
    ast: &spirv_cross::spirv::Ast<TargetT>,
    resource: &spirv_cross::spirv::Resource,
    descriptor_type: dsc::DescriptorType,
) -> Result<u32, String>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    Ok(match descriptor_type {
        dsc::DescriptorType::UniformBuffer
        | dsc::DescriptorType::UniformBufferDynamic
        | dsc::DescriptorType::StorageBuffer
        | dsc::DescriptorType::StorageBufferDynamic => {
            (ast.get_declared_struct_size(resource.type_id)
                .map_err(|_x| "could not get size from reflection data")?
                + 15)
                / 16
                * 16
        }
        _ => 0,
    })
}

fn get_descriptor_from_resource<TargetT>(
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    resource: &spirv_cross::spirv::Resource,
    descriptor_type: dsc::DescriptorType,
    stage_flags: dsc::ShaderStageFlags,
) -> Result<ReflectedDescriptorSetLayoutBinding, String>
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
    let descriptor_count = get_descriptor_count_from_type(ast, resource.type_id)?;

    let parsed_binding = declarations.bindings.iter().find(|x| x.parsed.layout_parts.binding == Some(binding as usize) && x.parsed.layout_parts.set == Some(set as usize))
        .or_else(|| declarations.bindings.iter().find(|x| x.parsed.instance_name == *name))
        .ok_or_else(|| format!("A resource named {} in spirv reflection data was not matched up to a resource scanned in source code.", resource.name))?;

    let size = get_descriptor_size_from_resource(ast, resource, descriptor_type)
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

    Ok(ReflectedDescriptorSetLayoutBinding {
        name: name.clone(),
        set,
        binding,
        size,
        descriptor_type,
        stage_flags,
        descriptor_count,
        internal_buffer_per_descriptor_size,
        immutable_samplers,
        slot_name,
    })
}

fn get_descriptors_from_resources<TargetT>(
    descriptors: &mut Vec<ReflectedDescriptorSetLayoutBinding>,
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
    resources: &[spirv_cross::spirv::Resource],
    descriptor_type: dsc::DescriptorType,
    stage_flags: dsc::ShaderStageFlags,
) -> Result<(), String>
where
    TargetT: spirv_cross::spirv::Target,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Parse<TargetT>,
    spirv_cross::spirv::Ast<TargetT>: spirv_cross::spirv::Compile<TargetT>,
{
    for resource in resources {
        descriptors.push(get_descriptor_from_resource(
            ast,
            declarations,
            resource,
            descriptor_type,
            stage_flags,
        )?);
    }

    Ok(())
}

pub(crate) fn reflect_data<TargetT>(
    ast: &spirv_cross::spirv::Ast<TargetT>,
    declarations: &super::parse_declarations::ParseDeclarationsResult,
) -> Result<Vec<ReflectedEntryPoint>, String>
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

        let mut bindings = Vec::default();

        let shader_resources = ast
            .get_shader_resources()
            .map_err(|_x| "could not get resources from reflection data")?;

        get_descriptors_from_resources(
            &mut bindings,
            ast,
            declarations,
            &shader_resources.uniform_buffers,
            dsc::DescriptorType::UniformBuffer,
            stage_flags,
        )?;
        get_descriptors_from_resources(
            &mut bindings,
            ast,
            declarations,
            &shader_resources.storage_buffers,
            dsc::DescriptorType::StorageBuffer,
            stage_flags,
        )?;
        get_descriptors_from_resources(
            &mut bindings,
            ast,
            declarations,
            &shader_resources.storage_images,
            dsc::DescriptorType::StorageImage,
            stage_flags,
        )?;
        get_descriptors_from_resources(
            &mut bindings,
            ast,
            declarations,
            &shader_resources.sampled_images,
            dsc::DescriptorType::CombinedImageSampler,
            stage_flags,
        )?;
        get_descriptors_from_resources(
            &mut bindings,
            ast,
            declarations,
            &shader_resources.separate_images,
            dsc::DescriptorType::SampledImage,
            stage_flags,
        )?;
        get_descriptors_from_resources(
            &mut bindings,
            ast,
            declarations,
            &shader_resources.separate_samplers,
            dsc::DescriptorType::Sampler,
            stage_flags,
        )?;
        // stage inputs
        // stage outputs
        // subpass inputs
        // atomic counters
        // push constant buffers

        let mut descriptor_set_layouts: Vec<Option<ReflectedDescriptorSetLayout>> = vec![];
        for binding in bindings {
            while descriptor_set_layouts.len() <= binding.set as usize {
                descriptor_set_layouts.push(None);
            }

            match &mut descriptor_set_layouts[binding.set as usize] {
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
        let mut push_constants = Vec::<ReflectedPushConstant>::default();
        for push_constant in &shader_resources.push_constant_buffers {
            let push_constant_ranges = ast
                .get_active_buffer_ranges(push_constant.id)
                .map_err(|_x| "could not get active buffer ranges")?;
            for push_constant_range in &push_constant_ranges {
                push_constants.push(ReflectedPushConstant {
                    name: push_constant.name.clone(),
                    push_constant: dsc::PushConstantRange {
                        size: push_constant_range.range as u32,
                        offset: push_constant_range.offset as u32,
                        stage_flags,
                    },
                });
            }
        }

        //TODO: Store the type and verify that the format associated in the game i.e. R32G32B32 is
        // something reasonable (like vec3).
        let mut vertex_inputs = Vec::default();
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
                    .ok_or_else(|| format!("No semantic annotation for vertex input '{}'. All vertex inputs must have a semantic annotation.", name))?;

                vertex_inputs.push(ReflectedVertexInput {
                    name: name.clone(),
                    semantic: semantic.clone(),
                    location,
                });
            }
        }

        reflected_entry_points.push(ReflectedEntryPoint {
            name: entry_point_name,
            stage_flags,
            descriptor_set_layouts,
            push_constants,
            vertex_inputs,
        });
    }

    Ok(reflected_entry_points)
}

fn map_shader_stage_flags(
    shader_stage: spirv_cross::spirv::ExecutionModel
) -> Result<dsc::ShaderStageFlags, String> {
    use spirv_cross::spirv::ExecutionModel;
    Ok(match shader_stage {
        ExecutionModel::Vertex => dsc::ShaderStageFlags::VERTEX,
        ExecutionModel::TessellationControl => dsc::ShaderStageFlags::TESSELLATION_CONTROL,
        ExecutionModel::TessellationEvaluation => dsc::ShaderStageFlags::TESSELLATION_EVALUATION,
        ExecutionModel::Geometry => dsc::ShaderStageFlags::GEOMETRY,
        ExecutionModel::Fragment => dsc::ShaderStageFlags::FRAGMENT,
        ExecutionModel::GlCompute => dsc::ShaderStageFlags::COMPUTE,
        ExecutionModel::Kernel => dsc::ShaderStageFlags::COMPUTE,
    })
}
