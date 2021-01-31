use crate::{
    DescriptorSetWriteSet, FixedFunctionState, MaterialPassResource, MaterialPassVertexInput,
    RafxResult, ReflectedEntryPoint, ReflectedShader, ResourceArc, ResourceContext,
    ShaderModuleResource, SlotNameLookup,
};
use fnv::FnvHashSet;
use rafx_api::{RafxImmutableSamplerKey, RafxShaderStageFlags};
use rafx_nodes::RenderPhaseIndex;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialShaderStage {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    Compute,
}

impl Into<RafxShaderStageFlags> for MaterialShaderStage {
    fn into(self) -> RafxShaderStageFlags {
        match self {
            MaterialShaderStage::Vertex => RafxShaderStageFlags::VERTEX,
            MaterialShaderStage::TessellationControl => RafxShaderStageFlags::TESSELLATION_CONTROL,
            MaterialShaderStage::TessellationEvaluation => {
                RafxShaderStageFlags::TESSELLATION_EVALUATION
            }
            MaterialShaderStage::Geometry => RafxShaderStageFlags::GEOMETRY,
            MaterialShaderStage::Fragment => RafxShaderStageFlags::FRAGMENT,
            MaterialShaderStage::Compute => RafxShaderStageFlags::COMPUTE,
        }
    }
}

pub struct MaterialPassInner {
    pub shader_modules: Vec<ResourceArc<ShaderModuleResource>>,

    // Info required to recreate the pipeline for new swapchains
    pub material_pass_resource: ResourceArc<MaterialPassResource>,

    // Data this material pass expects to receive via bound vertex buffers
    pub vertex_inputs: Arc<Vec<MaterialPassVertexInput>>,

    //TODO: Use hash instead of string. Probably want to have a "hashed string" type that keeps the
    // string around only in debug mode. Maybe this could be generalized to a HashOfThing<T>.
    pub pass_slot_name_lookup: Arc<SlotNameLookup>,

    // This is a hint of what render phase we should register a material with in the pipeline cache
    // It is optional and the pipeline cache can handle materials used in any render phase
    pub render_phase_index: Option<RenderPhaseIndex>,
}

#[derive(Clone)]
pub struct MaterialPass {
    inner: Arc<MaterialPassInner>,
}

impl MaterialPass {
    pub fn new(
        resource_context: &ResourceContext,
        pass_name: Option<&str>,
        phase_name: Option<&str>,
        fixed_function_state: Arc<FixedFunctionState>,
        shader_modules: Vec<ResourceArc<ShaderModuleResource>>,
        entry_points: &[&ReflectedEntryPoint],
    ) -> RafxResult<MaterialPass> {
        // Combine reflection data from all stages in the shader
        let reflected_shader = ReflectedShader::new(entry_points)?;

        let vertex_inputs = reflected_shader.vertex_inputs.ok_or_else(|| {
            let message = format!(
                "The material pass named '{:?}' does not specify a vertex shader",
                pass_name
            );
            log::error!("{}", message);
            message
        })?;

        //
        // Shader
        //
        let shader = resource_context
            .resources()
            .get_or_create_shader(&shader_modules, entry_points)?;

        //
        // Root Signature
        //
        // Put all samplers into a hashmap so that we avoid collecting duplicates, and keep them
        // around to prevent the ResourceArcs from dropping out of scope and being destroyed
        let mut immutable_samplers = FnvHashSet::default();

        // We also need to save vecs of samplers that
        let mut immutable_rafx_sampler_lists = Vec::default();
        let mut immutable_rafx_sampler_keys = Vec::default();

        for (set_index, descriptor_set_layout_def) in reflected_shader
            .descriptor_set_layout_defs
            .iter()
            .enumerate()
        {
            // Get or create samplers and add them to the two above structures
            for binding in &descriptor_set_layout_def.bindings {
                if let Some(sampler_defs) = &binding.immutable_samplers {
                    let mut samplers = Vec::with_capacity(sampler_defs.len());
                    for sampler_def in sampler_defs {
                        let sampler = resource_context
                            .resources()
                            .get_or_create_sampler(sampler_def)?;
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

        let root_signature = resource_context.resources().get_or_create_root_signature(
            &[shader.clone()],
            &immutable_rafx_sampler_keys,
            &immutable_rafx_sampler_lists,
        )?;

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layouts =
            Vec::with_capacity(reflected_shader.descriptor_set_layout_defs.len());

        for (set_index, descriptor_set_layout_def) in reflected_shader
            .descriptor_set_layout_defs
            .iter()
            .enumerate()
        {
            let descriptor_set_layout = resource_context
                .resources()
                .get_or_create_descriptor_set_layout(
                    &root_signature,
                    set_index as u32,
                    &descriptor_set_layout_def,
                )?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        //
        // Create the material pass
        //
        let material_pass = resource_context.resources().get_or_create_material_pass(
            shader,
            root_signature,
            descriptor_set_layouts,
            fixed_function_state,
            vertex_inputs.clone(),
        )?;

        //
        // If a phase name is specified, register the pass with the pipeline cache. The pipeline
        // cache is responsible for ensuring pipelines are created for renderpasses that execute
        // within the pipeline's phase
        //
        let render_phase_index = if let Some(phase_name) = phase_name {
            let render_phase_index = resource_context
                .graphics_pipeline_cache()
                .get_render_phase_by_name(phase_name);
            match render_phase_index {
                Some(render_phase_index) => resource_context
                    .graphics_pipeline_cache()
                    .register_material_to_phase_index(&material_pass, render_phase_index),
                None => {
                    let error = format!(
                        "Load Material Failed - Pass refers to phase name {}, but this phase name was not registered",
                        phase_name
                    );
                    log::error!("{}", error);
                    return Err(error)?;
                }
            }

            render_phase_index
        } else {
            None
        };

        let inner = MaterialPassInner {
            shader_modules,
            material_pass_resource: material_pass.clone(),
            pass_slot_name_lookup: Arc::new(reflected_shader.slot_name_lookup),
            vertex_inputs,
            render_phase_index,
        };

        Ok(MaterialPass {
            inner: Arc::new(inner),
        })
    }

    pub fn create_uninitialized_write_sets_for_material_pass(&self) -> Vec<DescriptorSetWriteSet> {
        // The metadata for the descriptor sets within this pass, one for each set within the pass
        let pass_descriptor_set_writes: Vec<_> = self
            .inner
            .material_pass_resource
            .get_raw()
            .descriptor_set_layouts
            .iter()
            .map(|layout| {
                super::descriptor_sets::create_uninitialized_write_set_for_layout(
                    &layout.get_raw().descriptor_set_layout_def,
                )
            })
            .collect();

        pass_descriptor_set_writes
    }
}

impl Deref for MaterialPass {
    type Target = MaterialPassInner;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
