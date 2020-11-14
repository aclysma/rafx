use crate::assets::shader::ShaderAssetData;
use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{ImportedAsset, Importer, ImporterValue};
use renderer_resources::vk_description as dsc;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;
use std::ops::Add;
use spirv_reflect::types::ReflectDescriptorType;

use super::{ReflectedEntryPoint, ReflectedDescriptorSetLayoutBinding, ReflectedDescriptorSetLayout, ReflectedInputVariable, ReflectedOutputVariable, ReflectedPushConstant};


#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "867bc278-67b5-469c-aeea-1c05da722918"]
pub struct ShaderImporterSpvState(Option<AssetUuid>);

// There may be a better way to do this type coercing
fn coerce_result_str<T>(result: Result<T, &str>) -> atelier_assets::importer::Result<T> {
    let ok = result.map_err(|x| -> Box<dyn std::error::Error + Send> { Box::<dyn std::error::Error + Send + Sync>::from(x) })?;
    Ok(ok)
}

fn coerce_result_string<T>(result: Result<T, String>) -> atelier_assets::importer::Result<T> {
    let ok = result.map_err(|x| -> Box<dyn std::error::Error + Send> { Box::<dyn std::error::Error + Send + Sync>::from(x) })?;
    Ok(ok)
}

#[derive(TypeUuid)]
#[uuid = "90fdad4b-cec1-4f59-b679-97895711b6e1"]
pub struct ShaderImporterSpv;
impl Importer for ShaderImporterSpv {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        4
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = ShaderImporterSpvState;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let asset_id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ShaderImporterSpvState(Some(asset_id));

        // Raw compiled shader
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let shader_module = coerce_result_str(spirv_reflect::create_shader_module(&bytes))?;
        let code = shader_module.get_code();
        log::trace!("Import shader asset {:?} with {} bytes of code", asset_id, code.len() * std::mem::size_of::<u32>());

        // Scan the original source for custom markup/directives (doesn't do anything yet)
        read_spv_source(asset_id, &code);

        // Auto-create vulkan descriptors from reflection data
        let reflection_data = read_spv_reflection_data(&shader_module)?;
        log::info!("Import shader asset {:?} reflection data: \n{:#?}", asset_id, reflection_data);

        // The hash is used in some places identify the shader
        let code_hash = dsc::ShaderModuleCodeHash::hash_shader_code(&code);

        let shader_asset = ShaderAssetData {
            shader: dsc::ShaderModule { code, code_hash },
            reflection_data
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: asset_id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader_asset),
            }],
        })
    }
}


fn read_source_from_spv_opcodes(spv_code: &[u32], start: usize, end: usize) -> &str {
    let source = &spv_code[(start..end)];

    // Unsafe to cast the u32 slice to a u8 slice
    assert!(end <= spv_code.len());
    let byte_slice = unsafe {
        // Convert the u32 slice to a u8 slice
        std::slice::from_raw_parts(source.as_ptr() as *const u8, source.len() * std::mem::size_of::<u32>())
    };

    // Parse it into a UTF-8 string and find the first null character
    let string = std::str::from_utf8(byte_slice).unwrap();
    let string_length = string.find('\0').unwrap_or(string.len());
    &string[0..string_length]
}

fn read_spv_source(asset_id: AssetUuid, spv_code: &[u32]) {
    //TODO: We could strip things, example here:
    // https://github.com/KhronosGroup/SPIRV-Reflect/blob/master/util/stripper/stripper.cpp
    const SPV_HEADER_LENGTH: usize = 5;
    const SPV_MAGIC_NUMBER: u32 = 0x07230203;
    const SPV_SOURCE_OPCODE: u32 = 3;
    const SPV_SOURCE_CONTINUED_OPCODE: u32 = 2;

    let mut language_id = None;
    let mut language_version = None;
    let mut source_text = "".to_string();

    assert_eq!(spv_code[0], SPV_MAGIC_NUMBER);
    let mut i = SPV_HEADER_LENGTH;
    while i < spv_code.len() {
        let inst_len = (spv_code[i] >> 16) as usize;
        let opcode = spv_code[i] & 0x0000ffff;

        // https://www.khronos.org/registry/spir-v/specs/unified1/SPIRV.html#OpSource
        if opcode == SPV_SOURCE_OPCODE {
            if !source_text.is_empty() {
                log::trace!("Import shader asset {:?} language: {:?}", asset_id, language_id);
                log::trace!("Import shader asset {:?} version {:?}", asset_id, language_version);
                log::trace!("Import shader asset {:?} code {}", asset_id, source_text);

                // Reset to import another file
                assert!(language_id.is_some());
                assert!(language_version.is_some());
                language_id = None;
                language_version = None;
                source_text.clear();
            }

            // HLSL, GLSL, etc.
            assert!(language_id.is_none());
            language_id = Some(spv_code[i + 1]);

            // version of the language
            assert!(language_version.is_none());
            language_version = Some(spv_code[i + 2]);

            // ID (can be mapped to string by scanning OpString opcodes) is at word offset 3, but
            // doesn't seem very useful

            // The source code, null-terminated UTF-8
            let start = i + 4;
            let end = i + inst_len;
            let string = read_source_from_spv_opcodes(&spv_code, start, end);

            source_text = source_text.add(string);
        } else if opcode == SPV_SOURCE_CONTINUED_OPCODE {
            // The source code, null-terminated UTF-8, append onto source we've read so far
            let start = i + 1;
            let end = i + inst_len;
            let string = read_source_from_spv_opcodes(&spv_code, start, end);

            source_text = source_text.add(string);
        }

        i += inst_len;
    }

    log::trace!("Import shader asset {:?} language: {:?}", asset_id, language_id);
    log::trace!("Import shader asset {:?} version {:?}", asset_id, language_version);
    log::trace!("Import shader asset {:?} code {}", asset_id, source_text);
}

fn read_spv_reflection_data(shader_module: &spirv_reflect::ShaderModule) -> atelier_assets::importer::Result<Vec<ReflectedEntryPoint>> {
    let mut entry_points = vec![];
    let enumerated_entry_points = coerce_result_str(shader_module.enumerate_entry_points())?;
    for entry_point in enumerated_entry_points {
        let entry_point_name = entry_point.name.clone();
        let entry_point_shader_stage_flags = map_shader_stage_flags(entry_point.shader_stage)?;
        //TODO: Validate entry_point.shader_stage and maybe spirv_execution_model?
        //TODO: Do something with used uniforms/used push constants?

        let mut descriptor_set_layouts = vec![];
        for descriptor_set in &entry_point.descriptor_sets {
            let mut descriptor_set_bindings = vec![];

            for binding in &descriptor_set.bindings {
                let name = &binding.name;
                //let padded_size = binding.block.padded_size;
                let size = binding.block.size;
                // Size is available as well, but I think padded is the better one to use here

                println!("!!!!! binding {:#?}", binding);

                let descriptor_set_binding = ReflectedDescriptorSetLayoutBinding {
                    name: name.clone(),
                    binding: binding.binding,
                    stage_flags: entry_point_shader_stage_flags,
                    descriptor_count: binding.array.dims.get(0).cloned().unwrap_or(1),
                    descriptor_type: map_descriptor_type(binding.descriptor_type)?,
                    size,
                    //padded_size,
                };

                descriptor_set_bindings.push(descriptor_set_binding);
            }

            while descriptor_set_layouts.len() <= descriptor_set.set as usize {
                descriptor_set_layouts.push(None);
            }

            descriptor_set_layouts[descriptor_set.set as usize] = Some(ReflectedDescriptorSetLayout {
                //set: descriptor_set.set,
                bindings: descriptor_set_bindings
            });
        }

        let mut input_variables = vec![];
        for input_variable in &entry_point.input_variables {
            let name = &input_variable.name;
            let location = input_variable.location;

            // Probably a built-in type (like gl_Position)
            if location == u32::MAX {
                continue;
            }

            let format = coerce_result_string(map_format(input_variable.format).map_err(|x| format!("Error reading input var {} at location {}: {}\n{:#?}", name, location, x, input_variable)))?;

            input_variables.push(ReflectedInputVariable {
                name: name.clone(),
                location,
                format
            })
        }

        let mut output_variables = vec![];
        for output_variable in &entry_point.output_variables {
            let name = &output_variable.name;
            let location = output_variable.location;

            // Probably a built-in type (like gl_Position)
            if location == u32::MAX {
                continue;
            }

            let format = coerce_result_string(map_format(output_variable.format).map_err(|x| format!("Error reading output var {} at location {}: {}\n{:#?}", name, location, x, output_variable)))?;

            output_variables.push(ReflectedOutputVariable {
                name: name.clone(),
                location,
                format
            })
        }

        let mut push_constants = vec![];
        let enumerated_push_constants = coerce_result_str(shader_module.enumerate_push_constant_blocks(Some(&entry_point_name)))?;
        for push_constant in enumerated_push_constants {
            let name = push_constant.name;
            let offset = push_constant.absolute_offset;
            let size = push_constant.padded_size;

            push_constants.push(ReflectedPushConstant {
                name,
                push_constant: dsc::PushConstantRange {
                    stage_flags: entry_point_shader_stage_flags,
                    offset,
                    size
                }
            })
        }

        entry_points.push(ReflectedEntryPoint {
            name: entry_point.name.clone(),
            stage_flags: entry_point_shader_stage_flags,
            descriptor_set_layouts,
            input_variables,
            output_variables,
            push_constants
        });
    }

    Ok(entry_points)
}

fn map_descriptor_type(descriptor_type: spirv_reflect::types::ReflectDescriptorType) -> atelier_assets::importer::Result<dsc::DescriptorType> {
    coerce_result_string(match descriptor_type {
        ReflectDescriptorType::Sampler => Ok(dsc::DescriptorType::Sampler),
        ReflectDescriptorType::CombinedImageSampler => Ok(dsc::DescriptorType::CombinedImageSampler),
        ReflectDescriptorType::SampledImage => Ok(dsc::DescriptorType::SampledImage),
        ReflectDescriptorType::StorageImage => Ok(dsc::DescriptorType::StorageImage),
        ReflectDescriptorType::UniformTexelBuffer => Ok(dsc::DescriptorType::UniformTexelBuffer),
        ReflectDescriptorType::StorageTexelBuffer => Ok(dsc::DescriptorType::StorageTexelBuffer),
        ReflectDescriptorType::UniformBuffer => Ok(dsc::DescriptorType::UniformBuffer),
        ReflectDescriptorType::StorageBuffer => Ok(dsc::DescriptorType::StorageBuffer),
        ReflectDescriptorType::UniformBufferDynamic => Ok(dsc::DescriptorType::UniformBufferDynamic),
        ReflectDescriptorType::StorageBufferDynamic => Ok(dsc::DescriptorType::StorageBufferDynamic),
        ReflectDescriptorType::InputAttachment => Ok(dsc::DescriptorType::InputAttachment),
        descriptor_type @ _ => Err(format!("Unrecognized descriptor type {:?}", descriptor_type))
    })
}

fn map_shader_stage_flags(shader_stage: spirv_reflect::types::ReflectShaderStageFlags) -> atelier_assets::importer::Result<dsc::ShaderStageFlags> {
    use spirv_reflect::types::ReflectShaderStageFlags;
    let mut stages = dsc::ShaderStageFlags::default();
    if !(shader_stage & ReflectShaderStageFlags::VERTEX).is_empty() { stages |= dsc::ShaderStageFlags::VERTEX; }
    if !(shader_stage & ReflectShaderStageFlags::TESSELLATION_CONTROL).is_empty() { stages |= dsc::ShaderStageFlags::TESSELLATION_CONTROL; }
    if !(shader_stage & ReflectShaderStageFlags::TESSELLATION_EVALUATION).is_empty() { stages |= dsc::ShaderStageFlags::TESSELLATION_EVALUATION; }
    if !(shader_stage & ReflectShaderStageFlags::GEOMETRY).is_empty() { stages |= dsc::ShaderStageFlags::GEOMETRY; }
    if !(shader_stage & ReflectShaderStageFlags::FRAGMENT).is_empty() { stages |= dsc::ShaderStageFlags::FRAGMENT; }
    if !(shader_stage & ReflectShaderStageFlags::COMPUTE).is_empty() { stages |= dsc::ShaderStageFlags::COMPUTE; }
    Ok(stages)
}

fn map_format(descriptor_type: spirv_reflect::types::ReflectFormat) -> Result<dsc::Format, String> {
    use spirv_reflect::types::ReflectFormat;
    match descriptor_type {
        ReflectFormat::R32_UINT => Ok(dsc::Format::R32_UINT),
        ReflectFormat::R32_SINT => Ok(dsc::Format::R32_SINT),
        ReflectFormat::R32_SFLOAT => Ok(dsc::Format::R32_SFLOAT),
        ReflectFormat::R32G32_UINT => Ok(dsc::Format::R32G32_UINT),
        ReflectFormat::R32G32_SINT => Ok(dsc::Format::R32G32_SINT),
        ReflectFormat::R32G32_SFLOAT => Ok(dsc::Format::R32G32_SFLOAT),
        ReflectFormat::R32G32B32_UINT => Ok(dsc::Format::R32G32B32_UINT),
        ReflectFormat::R32G32B32_SINT => Ok(dsc::Format::R32G32B32_SINT),
        ReflectFormat::R32G32B32_SFLOAT => Ok(dsc::Format::R32G32B32_SFLOAT),
        ReflectFormat::R32G32B32A32_UINT => Ok(dsc::Format::R32G32B32A32_UINT),
        ReflectFormat::R32G32B32A32_SINT => Ok(dsc::Format::R32G32B32A32_SINT),
        ReflectFormat::R32G32B32A32_SFLOAT => Ok(dsc::Format::R32G32B32A32_SFLOAT),
        descriptor_type @ _ => Err(format!("Unrecognized format {:?}", descriptor_type))
    }
}