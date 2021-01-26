use crate::{
    DescriptorSetLayout, DescriptorSetLayoutBinding, MaterialPassVertexInput, ShaderModuleHash,
};
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::{
    RafxResult, RafxSamplerDef, RafxShaderPackage, RafxShaderResource, RafxShaderStageFlags,
    RafxShaderStageReflection,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct SlotLocation {
    pub layout_index: u32,
    pub binding_index: u32,
    //pub array_index: u32,
}

pub type SlotNameLookup = FnvHashMap<String, FnvHashSet<SlotLocation>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ReflectedDescriptorSetLayoutBinding {
    // Basic info required to create the RafxRootSignature
    pub resource: RafxShaderResource,

    // Samplers created here will be automatically created/bound
    pub immutable_samplers: Option<Vec<RafxSamplerDef>>,

    // If this is non-zero we will allocate a buffer owned by the descriptor set pool chunk,
    // and automatically bind it - this makes binding data easy to do without having to manage
    // buffers.
    pub internal_buffer_per_descriptor_size: Option<u32>,
}

impl Into<DescriptorSetLayoutBinding> for ReflectedDescriptorSetLayoutBinding {
    fn into(self) -> DescriptorSetLayoutBinding {
        DescriptorSetLayoutBinding {
            resource: self.resource.clone(),
            immutable_samplers: self.immutable_samplers.clone(),
            internal_buffer_per_descriptor_size: self.internal_buffer_per_descriptor_size,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedDescriptorSetLayout {
    // These are NOT indexable by binding (i.e. may be sparse)
    pub bindings: Vec<ReflectedDescriptorSetLayoutBinding>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedVertexInput {
    pub name: String,
    pub semantic: String,
    pub location: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedEntryPoint {
    // The reflection data used by rafx API
    pub rafx_api_reflection: RafxShaderStageReflection,

    // Additional reflection data used by the framework level for descriptor sets
    pub descriptor_set_layouts: Vec<Option<ReflectedDescriptorSetLayout>>,

    // Additional reflection data used by the framework level for vertex inputs
    pub vertex_inputs: Vec<ReflectedVertexInput>,
}

// An import format that will get turned into ShaderAssetData
#[derive(Serialize, Deserialize)]
pub struct CookedShaderPackage {
    pub hash: ShaderModuleHash,
    pub shader_package: RafxShaderPackage,
    pub entry_points: Vec<ReflectedEntryPoint>,
}

impl CookedShaderPackage {
    pub fn find_entry_point(
        &self,
        entry_point_name: &str,
    ) -> Option<&ReflectedEntryPoint> {
        self.entry_points
            .iter()
            .find(|x| x.rafx_api_reflection.entry_point_name == entry_point_name)
    }
}

pub struct ReflectedShader {
    pub descriptor_set_layout_defs: Vec<DescriptorSetLayout>,
    pub slot_name_lookup: SlotNameLookup,
    pub vertex_inputs: Option<Arc<Vec<MaterialPassVertexInput>>>,
}

impl ReflectedShader {
    pub fn new(entry_points: &[&ReflectedEntryPoint]) -> RafxResult<ReflectedShader> {
        let mut descriptor_set_layout_defs = Vec::default();
        let mut slot_name_lookup: SlotNameLookup = Default::default();
        let mut vertex_inputs = None;

        // We iterate through the entry points we will hit for each stage. Each stage may define
        // slightly different reflection data/bindings in use.
        for reflection_data in entry_points {
            log::trace!("  Reflection data:\n{:#?}", reflection_data);

            if reflection_data
                .rafx_api_reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::VERTEX)
            {
                let inputs: Vec<_> = reflection_data
                    .vertex_inputs
                    .iter()
                    .map(|x| MaterialPassVertexInput {
                        semantic: x.semantic.clone(),
                        location: x.location,
                    })
                    .collect();

                assert!(vertex_inputs.is_none());
                vertex_inputs = Some(Arc::new(inputs));
            }

            // Currently not using push constants and it will be handled in the rafx api layer
            // for (range_index, range) in reflection_data.push_constants.iter().enumerate() {
            //     if let Some(existing_range) = push_constant_ranges.get(range_index) {
            //         if range.push_constant != *existing_range {
            //             let error = format!(
            //                 "Load Material Failed - Pass has shaders with conflicting push constants",
            //             );
            //             log::error!("{}", error);
            //             return Err(error)?;
            //         } else {
            //             log::trace!("    Range index {} already exists and matches", range_index);
            //         }
            //     } else {
            //         log::trace!("    Add range index {} {:?}", range_index, range);
            //         push_constant_ranges.push(range.push_constant.clone());
            //     }
            // }

            for (set_index, layout) in reflection_data.descriptor_set_layouts.iter().enumerate() {
                // Expand the layout def to include the given set index
                while descriptor_set_layout_defs.len() <= set_index {
                    descriptor_set_layout_defs.push(DescriptorSetLayout::default());
                }

                if let Some(layout) = layout.as_ref() {
                    for binding in &layout.bindings {
                        let existing_binding = descriptor_set_layout_defs[set_index]
                            .bindings
                            .iter_mut()
                            .find(|x| x.resource.binding == binding.resource.binding);

                        if let Some(existing_binding) = existing_binding {
                            //
                            // Binding already exists, just make sure this shader's definition for this binding matches
                            // the shader that added it originally
                            //
                            if existing_binding.resource.resource_type
                                != binding.resource.resource_type
                            {
                                let error = format!(
                                    "Load Material Failed - Pass is using shaders in different stages with different descriptor types for set={} binding={}",
                                    set_index,
                                    binding.resource.binding
                                );
                                log::error!("{}", error);
                                return Err(error)?;
                            }

                            if existing_binding.resource.element_count_normalized()
                                != binding.resource.element_count_normalized()
                            {
                                let error = format!(
                                    "Load Material Failed - Pass is using shaders in different stages with different descriptor counts for set={} binding={}",
                                    set_index,
                                    binding.resource.binding
                                );
                                log::error!("{}", error);
                                return Err(error)?;
                            }

                            if existing_binding.immutable_samplers != binding.immutable_samplers {
                                let error = format!(
                                    "Load Material Failed - Pass is using shaders in different stages with different immutable samplers for set={} binding={}",
                                    set_index,
                                    binding.resource.binding
                                );
                                log::error!("{}", error);
                                return Err(error)?;
                            }

                            if existing_binding.internal_buffer_per_descriptor_size
                                != binding.internal_buffer_per_descriptor_size
                            {
                                let error = format!(
                                    "Load Material Failed - Pass is using shaders in different stages with different internal buffer configuration for set={} binding={}",
                                    set_index,
                                    binding.resource.binding
                                );
                                log::error!("{}", error);
                                return Err(error)?;
                            }

                            log::trace!("    Descriptor for binding set={} binding={} already exists, adding stage {:?}", set_index, binding.resource.binding, binding.resource.used_in_shader_stages);
                            existing_binding.resource.used_in_shader_stages |=
                                binding.resource.used_in_shader_stages;
                        } else {
                            //
                            // This binding was not bound by a previous shader stage, set it up and apply any configuration from this material
                            //
                            log::trace!(
                                "    Add descriptor binding set={} binding={} for stage {:?}",
                                set_index,
                                binding.resource.binding,
                                binding.resource.used_in_shader_stages
                            );
                            let def = binding.clone().into();

                            descriptor_set_layout_defs[set_index].bindings.push(def);
                        }

                        if let Some(slot_name) = &binding.resource.name {
                            log::trace!(
                                "  Assign slot name '{}' to binding set={} binding={}",
                                slot_name,
                                set_index,
                                binding.resource.binding
                            );
                            slot_name_lookup
                                .entry(slot_name.clone())
                                .or_default()
                                .insert(SlotLocation {
                                    layout_index: set_index as u32,
                                    binding_index: binding.resource.binding,
                                });
                        }
                    }
                }
            }
        }

        Ok(ReflectedShader {
            vertex_inputs,
            descriptor_set_layout_defs,
            slot_name_lookup,
        })
    }
}
