use crate::resources::resource_lookup::ShaderResource;
use crate::{
    ComputePipelineResource, FixedFunctionState, MaterialPassResource, MaterialPassVertexInput,
    ResourceArc, ResourceLookupSet, SamplerResource, ShaderModuleResource,
};
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::{
    RafxImmutableSamplerKey, RafxReflectedDescriptorSetLayout, RafxReflectedEntryPoint, RafxResult,
    RafxShaderStageFlags,
};
use std::sync::Arc;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct SlotLocation {
    pub layout_index: u32,
    pub binding_index: u32,
}

pub type SlotNameLookup = FnvHashMap<String, FnvHashSet<SlotLocation>>;

pub struct ReflectedShaderMetadata {
    pub descriptor_set_layout_defs: Vec<RafxReflectedDescriptorSetLayout>,
    pub slot_name_lookup: SlotNameLookup,
    pub vertex_inputs: Option<Arc<Vec<MaterialPassVertexInput>>>,
}

impl ReflectedShaderMetadata {
    pub fn new(entry_points: &[&RafxReflectedEntryPoint]) -> RafxResult<ReflectedShaderMetadata> {
        let mut descriptor_set_layout_defs = Vec::default();
        let mut slot_name_lookup: SlotNameLookup = Default::default();
        let mut vertex_inputs = None;

        // We iterate through the entry points we will hit for each stage. Each stage may define
        // slightly different reflection data/bindings in use.
        for reflection_data in entry_points {
            //log::trace!("  Reflection data:\n{:#?}", reflection_data);

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
                        gl_attribute_name: x.name.clone(),
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
                    descriptor_set_layout_defs.push(RafxReflectedDescriptorSetLayout::default());
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

        Ok(ReflectedShaderMetadata {
            vertex_inputs,
            descriptor_set_layout_defs,
            slot_name_lookup,
        })
    }
}

pub struct ReflectedShader {
    pub metadata: ReflectedShaderMetadata,
    pub shader: ResourceArc<ShaderResource>,
}

impl ReflectedShader {
    pub fn new(
        resources: &ResourceLookupSet,
        shader_modules: &[ResourceArc<ShaderModuleResource>],
        entry_points: &[&RafxReflectedEntryPoint],
    ) -> RafxResult<Self> {
        let metadata = ReflectedShaderMetadata::new(entry_points)?;
        let shader = resources.get_or_create_shader(shader_modules, entry_points)?;

        Ok(ReflectedShader { metadata, shader })
    }

    pub fn create_immutable_samplers<'a>(
        resources: &'a ResourceLookupSet,
        descriptor_set_layouts: &'a [RafxReflectedDescriptorSetLayout],
    ) -> RafxResult<(
        Vec<RafxImmutableSamplerKey<'a>>,
        Vec<Vec<ResourceArc<SamplerResource>>>,
    )> {
        // Put all samplers into a hashmap so that we avoid collecting duplicates, and keep them
        // around to prevent the ResourceArcs from dropping out of scope and being destroyed
        let mut immutable_samplers = FnvHashSet::default();

        // We also need to save vecs of samplers that are immutable
        let mut immutable_rafx_sampler_lists = Vec::default();
        let mut immutable_rafx_sampler_keys = Vec::default();

        for (set_index, descriptor_set_layout_def) in descriptor_set_layouts.iter().enumerate() {
            // Get or create samplers and add them to the two above structures
            for binding in &descriptor_set_layout_def.bindings {
                if let Some(sampler_defs) = &binding.immutable_samplers {
                    let mut samplers = Vec::with_capacity(sampler_defs.len());
                    for sampler_def in sampler_defs {
                        let sampler = resources.get_or_create_sampler(sampler_def)?;
                        samplers.push(sampler.clone());
                        immutable_samplers.insert(sampler);
                    }

                    immutable_rafx_sampler_keys.push(RafxImmutableSamplerKey::Binding(
                        set_index as u32,
                        binding.resource.binding,
                    ));
                    immutable_rafx_sampler_lists.push(samplers);
                }
            }
        }
        Ok((immutable_rafx_sampler_keys, immutable_rafx_sampler_lists))
    }

    pub fn load_material_pass(
        &self,
        resources: &ResourceLookupSet,
        fixed_function_state: Arc<FixedFunctionState>,
    ) -> RafxResult<ResourceArc<MaterialPassResource>> {
        let vertex_inputs = self
            .metadata
            .vertex_inputs
            .as_ref()
            .ok_or_else(|| "The material pass does not specify a vertex shader")?
            .clone();

        //
        // Root Signature
        //
        let (immutable_rafx_sampler_keys, immutable_rafx_sampler_lists) =
            ReflectedShader::create_immutable_samplers(
                resources,
                &self.metadata.descriptor_set_layout_defs,
            )?;

        let root_signature = resources.get_or_create_root_signature(
            &[self.shader.clone()],
            &immutable_rafx_sampler_keys,
            &immutable_rafx_sampler_lists,
        )?;

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layouts =
            Vec::with_capacity(self.metadata.descriptor_set_layout_defs.len());

        for (set_index, descriptor_set_layout_def) in
            self.metadata.descriptor_set_layout_defs.iter().enumerate()
        {
            let descriptor_set_layout = resources.get_or_create_descriptor_set_layout(
                &root_signature,
                set_index as u32,
                &descriptor_set_layout_def,
            )?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        //
        // Create the material pass
        //
        resources.get_or_create_material_pass(
            self.shader.clone(),
            root_signature,
            descriptor_set_layouts,
            fixed_function_state,
            vertex_inputs.clone(),
        )
    }

    pub fn load_compute_pipeline(
        &self,
        resources: &ResourceLookupSet,
        //shader_module: &ResourceArc<ShaderModuleResource>,
        //entry_point: &ReflectedEntryPoint,
    ) -> RafxResult<ResourceArc<ComputePipelineResource>> {
        // let shader = resources
        //     .get_or_create_shader(&[shader_module.clone()], &[&entry_point])?;

        //let reflected_shader = ReflectedShader::new(&[entry_point])?;

        let (immutable_rafx_sampler_keys, immutable_rafx_sampler_lists) =
            ReflectedShader::create_immutable_samplers(
                resources,
                &self.metadata.descriptor_set_layout_defs,
            )?;

        let root_signature = resources.get_or_create_root_signature(
            &[self.shader.clone()],
            &immutable_rafx_sampler_keys,
            &immutable_rafx_sampler_lists,
        )?;

        //
        // Create the push constant ranges
        //

        // Currently unused, can be handled by the rafx api layer
        // let mut push_constant_ranges = vec![];
        // for (range_index, range) in entry_point.push_constants.iter().enumerate() {
        //     log::trace!("    Add range index {} {:?}", range_index, range);
        //     push_constant_ranges.push(range.push_constant.clone());
        // }

        //
        // Gather the descriptor set bindings
        //
        let mut descriptor_set_layout_defs = Vec::default();
        for (set_index, layout) in self.metadata.descriptor_set_layout_defs.iter().enumerate() {
            // Expand the layout def to include the given set index
            while descriptor_set_layout_defs.len() <= set_index {
                descriptor_set_layout_defs.push(RafxReflectedDescriptorSetLayout::default());
            }

            for binding in &layout.bindings {
                log::trace!(
                    "    Add descriptor binding set={} binding={} for stage {:?}",
                    set_index,
                    binding.resource.binding,
                    binding.resource.used_in_shader_stages
                );
                let def = binding.clone().into();

                descriptor_set_layout_defs[set_index].bindings.push(def);
            }
        }

        //
        // Create the descriptor set layout
        //
        let mut descriptor_set_layouts = Vec::with_capacity(descriptor_set_layout_defs.len());

        for (set_index, descriptor_set_layout_def) in descriptor_set_layout_defs.iter().enumerate()
        {
            let descriptor_set_layout = resources.get_or_create_descriptor_set_layout(
                &root_signature,
                set_index as u32,
                &descriptor_set_layout_def,
            )?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        //
        // Create the compute pipeline
        //
        resources.get_or_create_compute_pipeline(
            &self.shader,
            &root_signature,
            descriptor_set_layouts,
        )
    }
}
