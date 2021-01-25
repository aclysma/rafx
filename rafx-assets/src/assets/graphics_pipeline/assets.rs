use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::{AssetManager, ImageAsset, ShaderAsset};
use atelier_assets::loader::handle::Handle;
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::{
    RafxBlendState, RafxBlendStateRenderTarget, RafxCompareOp, RafxCullMode, RafxDepthState,
    RafxFillMode, RafxFrontFace, RafxImmutableSamplerKey, RafxRasterizerState, RafxResult,
    RafxSamplerDef, RafxShaderStageDef, RafxShaderStageFlags,
};
use rafx_nodes::{RenderPhase, RenderPhaseIndex};
pub use rafx_resources::DescriptorSetLayoutResource;
pub use rafx_resources::GraphicsPipelineResource;
use rafx_resources::{
    DescriptorSetArc, DescriptorSetLayout, FixedFunctionState, ResourceArc, ShaderModuleMeta,
    SlotLocation, SlotNameLookup,
};
use rafx_resources::{DescriptorSetWriteSet, MaterialPassResource, SamplerResource};
use rafx_resources::{MaterialPassVertexInput, ShaderModuleResource};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "7f30b29c-7fb9-4b31-a354-7cefbbade2f9"]
pub struct SamplerAssetData {
    pub sampler: RafxSamplerDef,
}

#[derive(TypeUuid, Clone)]
#[uuid = "9fe2825d-a7c5-43f6-97bb-d3385fb2c2c9"]
pub struct SamplerAsset {
    pub sampler: ResourceArc<SamplerResource>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub enum AlphaBlendingPreset {
    Disabled,
    Enabled,
}

impl Default for AlphaBlendingPreset {
    fn default() -> Self {
        AlphaBlendingPreset::Disabled
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub enum DepthBufferPreset {
    Disabled,
    Enabled,
    EnabledReverseZ,
}

impl Default for DepthBufferPreset {
    fn default() -> Self {
        DepthBufferPreset::Disabled
    }
}

// #[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
// pub enum CullModePreset {
//     UseRasterizerState,
//     Enabled
// }
//
// impl Default for CullModePreset {
//     fn default() -> Self {
//         CullModePreset::UseRasterizerState
//     }
// }
//
// #[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
// pub enum FrontFacePreset {
//     UseRasterizerState,
//     Enabled
// }
//
// impl Default for FrontFacePreset {
//     fn default() -> Self {
//         FrontFacePreset::UseRasterizerState
//     }
// }
//
// #[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
// pub enum FillModePreset {
//     UseRasterizerState,
//     Enabled
// }
//
// impl Default for FillModePreset {
//     fn default() -> Self {
//         FillModePreset::UseRasterizerState
//     }
// }

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "0dfa5d9a-89cd-40a1-adac-baf801db61db"]
pub struct GraphicsPipelineAssetData {
    #[serde(default)]
    blend_state: RafxBlendState,
    #[serde(default)]
    depth_state: RafxDepthState,
    #[serde(default)]
    rasterizer_state: RafxRasterizerState,

    // These override the above states
    #[serde(default)]
    alpha_blending: AlphaBlendingPreset,
    #[serde(default)]
    depth_testing: DepthBufferPreset,
    #[serde(default)]
    cull_mode: Option<RafxCullMode>,
    #[serde(default)]
    front_face: Option<RafxFrontFace>,
    #[serde(default)]
    fill_mode: Option<RafxFillMode>,
}

pub struct PreparedGraphicsPipelineAssetData {
    blend_state: RafxBlendState,
    depth_state: RafxDepthState,
    rasterizer_state: RafxRasterizerState,
}

impl GraphicsPipelineAssetData {
    pub fn prepare(self) -> RafxResult<PreparedGraphicsPipelineAssetData> {
        let mut blend_state = self.blend_state.clone();
        let mut depth_state = self.depth_state.clone();
        let mut rasterizer_state = self.rasterizer_state.clone();

        match self.alpha_blending {
            AlphaBlendingPreset::Disabled => {
                blend_state.independent_blend = false;
                blend_state.render_target_blend_states =
                    vec![RafxBlendStateRenderTarget::default_alpha_disabled()]
            }
            AlphaBlendingPreset::Enabled => {
                blend_state.independent_blend = false;
                blend_state.render_target_blend_states =
                    vec![RafxBlendStateRenderTarget::default_alpha_enabled()]
            }
        }

        match self.depth_testing {
            DepthBufferPreset::Disabled => {
                depth_state.depth_test_enable = false;
                depth_state.depth_write_enable = false;
            }
            DepthBufferPreset::Enabled => {
                depth_state.depth_test_enable = true;
                depth_state.depth_write_enable = true;
                depth_state.depth_compare_op = RafxCompareOp::LessOrEqual;
            }
            DepthBufferPreset::EnabledReverseZ => {
                depth_state.depth_test_enable = true;
                depth_state.depth_write_enable = true;
                depth_state.depth_compare_op = RafxCompareOp::GreaterOrEqual;
            }
        }

        if let Some(cull_mode) = self.cull_mode {
            rasterizer_state.cull_mode = cull_mode;
        }

        if let Some(fill_mode) = self.fill_mode {
            rasterizer_state.fill_mode = fill_mode;
        }

        if let Some(front_face) = self.front_face {
            rasterizer_state.front_face = front_face;
        }

        Ok(PreparedGraphicsPipelineAssetData {
            blend_state,
            depth_state,
            rasterizer_state,
        })
    }
}

// The actual GPU resources are held in Material because the pipeline does not specify everything
// needed to create the pipeline
#[derive(TypeUuid, Clone)]
#[uuid = "7a6a7ba8-a3ca-41eb-94f4-5d3723cd8b44"]
pub struct GraphicsPipelineAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub pipeline_asset: Arc<PreparedGraphicsPipelineAssetData>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderStage {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    Compute,
}

impl Into<RafxShaderStageFlags> for ShaderStage {
    fn into(self) -> RafxShaderStageFlags {
        match self {
            ShaderStage::Vertex => RafxShaderStageFlags::VERTEX,
            ShaderStage::TessellationControl => RafxShaderStageFlags::TESSELLATION_CONTROL,
            ShaderStage::TessellationEvaluation => RafxShaderStageFlags::TESSELLATION_EVALUATION,
            ShaderStage::Geometry => RafxShaderStageFlags::GEOMETRY,
            ShaderStage::Fragment => RafxShaderStageFlags::FRAGMENT,
            ShaderStage::Compute => RafxShaderStageFlags::COMPUTE,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GraphicsPipelineShaderStage {
    pub stage: ShaderStage,
    pub shader_module: Handle<ShaderAsset>,
    pub entry_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialPassData {
    pub name: Option<String>,
    pub phase: Option<String>,
    pub pipeline: Handle<GraphicsPipelineAsset>,
    pub shaders: Vec<GraphicsPipelineShaderStage>,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "ad94bca2-1f02-4e5f-9117-1a7b03456a11"]
pub struct MaterialAssetData {
    pub passes: Vec<MaterialPassData>,
}

pub struct MaterialPassInner {
    pub shader_modules: Vec<ResourceArc<ShaderModuleResource>>,

    // Info required to recreate the pipeline for new swapchains
    pub material_pass_resource: ResourceArc<MaterialPassResource>,

    //descriptor_set_factory: DescriptorSetFactory,
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
    #[profiling::function]
    pub fn new(
        asset_manager: &AssetManager,
        material_pass_data: &MaterialPassData,
    ) -> RafxResult<MaterialPass> {
        use atelier_assets::loader::handle::AssetHandle;
        //
        // Pipeline asset (represents fixed function state)
        //
        let loaded_pipeline_asset = asset_manager
            .loaded_assets()
            .graphics_pipelines
            .get_latest(material_pass_data.pipeline.load_handle())
            .unwrap();
        let pipeline_asset = loaded_pipeline_asset.pipeline_asset.clone();

        let fixed_function_state = Arc::new(FixedFunctionState {
            depth_state: pipeline_asset.depth_state.clone(),
            blend_state: pipeline_asset.blend_state.clone(),
            rasterizer_state: pipeline_asset.rasterizer_state.clone(),
        });

        //
        // Shaders
        //
        let mut shader_module_metas = Vec::with_capacity(material_pass_data.shaders.len());
        let mut shader_modules = Vec::with_capacity(material_pass_data.shaders.len());

        let mut descriptor_set_layout_defs = Vec::default();
        let mut pass_slot_name_lookup: SlotNameLookup = Default::default();
        let mut vertex_inputs = None;

        let mut rafx_shader_stages = Vec::with_capacity(material_pass_data.shaders.len());

        // We iterate through the entry points we will hit for each stage. Each stage may define
        // slightly different reflection data/bindings in use.
        for stage in &material_pass_data.shaders {
            log::trace!(
                "Set up material pass stage: {:?} material pass name: {:?}",
                stage,
                material_pass_data.name
            );
            let shader_module_meta = ShaderModuleMeta {
                stage: stage.stage.into(),
                entry_name: stage.entry_name.clone(),
            };
            shader_module_metas.push(shader_module_meta);

            let shader_asset = asset_manager
                .loaded_assets()
                .shader_modules
                .get_latest(stage.shader_module.load_handle())
                .unwrap();
            shader_modules.push(shader_asset.shader_module.clone());

            let reflection_data = shader_asset.reflection_data.get(&stage.entry_name);
            let reflection_data = reflection_data.ok_or_else(|| {
                let error_message = format!(
                    "Load Material Failed - Pass refers to entry point named {}, but no matching reflection data was found",
                    stage.entry_name
                );
                log::error!("{}", error_message);
                error_message
            })?;

            rafx_shader_stages.push(RafxShaderStageDef {
                shader_module: shader_asset.shader_module.get_raw().shader_module.clone(),
                reflection: reflection_data.rafx_api_reflection.clone(),
            });

            // Check that the compiled shader supports the given stage
            if (reflection_data.rafx_api_reflection.shader_stage & stage.stage.into()).is_empty() {
                let error = format!(
                    "Load Material Failed - Pass is using a shader for stage {:?}, but this shader supports stages {:?}.",
                    stage.stage,
                    reflection_data.rafx_api_reflection.shader_stage
                );
                log::error!("{}", error);
                return Err(error)?;
            }

            log::trace!("  Reflection data:\n{:#?}", reflection_data);

            if stage.stage == ShaderStage::Vertex {
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
                            pass_slot_name_lookup
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

        let vertex_inputs = vertex_inputs.ok_or_else(|| {
            let message = format!(
                "The material pass named '{:?}' does not specify a vertex shader",
                material_pass_data.name
            );
            log::error!("{}", message);
            message
        })?;

        //
        // Shader and Root signature
        //

        let shader = asset_manager
            .resources()
            .get_or_create_shader(&rafx_shader_stages, &shader_modules)?;

        // Put all samplers into a hashmap so that we avoid collecting duplicates, and keep them
        // around to prevent the ResourceArcs from dropping out of scope and being destroyed
        let mut immutable_samplers = FnvHashSet::default();

        // We also need to save vecs of samplers that
        let mut immutable_rafx_sampler_lists = Vec::default();
        let mut immutable_rafx_sampler_keys = Vec::default();

        for (set_index, descriptor_set_layout_def) in descriptor_set_layout_defs.iter().enumerate()
        {
            // Get or create samplers and add them to the two above structures
            for binding in &descriptor_set_layout_def.bindings {
                if let Some(sampler_defs) = &binding.immutable_samplers {
                    let mut samplers = Vec::with_capacity(sampler_defs.len());
                    for sampler_def in sampler_defs {
                        let sampler = asset_manager
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

        let root_signature = asset_manager.resources().get_or_create_root_signature(
            &[shader.clone()],
            &immutable_rafx_sampler_keys,
            &immutable_rafx_sampler_lists,
        )?;

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layouts = Vec::with_capacity(descriptor_set_layout_defs.len());

        for (set_index, descriptor_set_layout_def) in descriptor_set_layout_defs.iter().enumerate()
        {
            let descriptor_set_layout = asset_manager
                .resources()
                .get_or_create_descriptor_set_layout(
                    &root_signature,
                    set_index as u32,
                    &descriptor_set_layout_def,
                )?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        let material_pass = asset_manager.resources().get_or_create_material_pass(
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
        let render_phase_index = if let Some(phase_name) = &material_pass_data.phase {
            let render_phase_index = asset_manager
                .graphics_pipeline_cache()
                .get_render_phase_by_name(phase_name);
            match render_phase_index {
                Some(render_phase_index) => asset_manager
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
            pass_slot_name_lookup: Arc::new(pass_slot_name_lookup),
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
            .material_pass_resource
            .get_raw()
            .descriptor_set_layouts
            .iter()
            .map(|layout| {
                rafx_resources::descriptor_sets::create_uninitialized_write_set_for_layout(
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

pub struct MaterialAssetInner {
    //TODO: Consider making this named
    //TODO: Get cached graphics pipelines working
    //TODO: Could consider decoupling render cache from phases
    pub passes: Vec<MaterialPass>,
    pub pass_name_to_index: FnvHashMap<String, usize>,
    pub pass_phase_to_index: FnvHashMap<RenderPhaseIndex, usize>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "165673cd-d81d-4708-b9a4-d7e1a2a67976"]
pub struct MaterialAsset {
    pub inner: Arc<MaterialAssetInner>,
}

impl MaterialAsset {
    pub fn new(
        passes: Vec<MaterialPass>,
        pass_name_to_index: FnvHashMap<String, usize>,
        pass_phase_to_index: FnvHashMap<RenderPhaseIndex, usize>,
    ) -> Self {
        let inner = MaterialAssetInner {
            passes,
            pass_name_to_index,
            pass_phase_to_index,
        };

        MaterialAsset {
            inner: Arc::new(inner),
        }
    }

    pub fn find_pass_by_name(
        &self,
        name: &str,
    ) -> Option<usize> {
        self.inner.pass_name_to_index.get(name).copied()
    }

    pub fn find_pass_by_phase<T: RenderPhase>(&self) -> Option<usize> {
        self.inner
            .pass_phase_to_index
            .get(&T::render_phase_index())
            .copied()
    }

    pub fn find_pass_by_phase_index(
        &self,
        index: RenderPhaseIndex,
    ) -> Option<usize> {
        self.inner.pass_phase_to_index.get(&index).copied()
    }
}

impl Deref for MaterialAsset {
    type Target = MaterialAssetInner;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialInstanceSlotAssignment {
    pub slot_name: String,
    pub image: Option<Handle<ImageAsset>>,
    pub sampler: Option<RafxSamplerDef>,

    // Would be nice to use this, but I don't think it works with Option
    //#[serde(with = "serde_bytes")]
    pub buffer_data: Option<Vec<u8>>,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "0d8cacf7-79df-4aa6-b99e-659a9c3b5e6b"]
pub struct MaterialInstanceAssetData {
    pub material: Handle<MaterialAsset>,
    pub slot_assignments: Vec<MaterialInstanceSlotAssignment>,
}

pub struct MaterialInstanceAssetInner {
    pub material_handle: Handle<MaterialAsset>,
    pub material: MaterialAsset,

    // Arc these individually because some downstream systems care only about the descriptor sets
    pub material_descriptor_sets: Arc<Vec<Vec<Option<DescriptorSetArc>>>>,
    pub slot_assignments: Vec<MaterialInstanceSlotAssignment>,
    pub descriptor_set_writes: Vec<Vec<DescriptorSetWriteSet>>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "c60f6a3d-3e8d-4eea-8576-0971cd71b60f"]
pub struct MaterialInstanceAsset {
    pub inner: Arc<MaterialInstanceAssetInner>,
}

impl MaterialInstanceAsset {
    pub fn new(
        material: Handle<MaterialAsset>,
        material_asset: MaterialAsset,
        material_descriptor_sets: Arc<Vec<Vec<Option<DescriptorSetArc>>>>,
        slot_assignments: Vec<MaterialInstanceSlotAssignment>,
        descriptor_set_writes: Vec<Vec<DescriptorSetWriteSet>>,
    ) -> Self {
        let inner = MaterialInstanceAssetInner {
            material_handle: material,
            material: material_asset,
            material_descriptor_sets,
            slot_assignments,
            descriptor_set_writes,
        };

        MaterialInstanceAsset {
            inner: Arc::new(inner),
        }
    }
}

impl Deref for MaterialInstanceAsset {
    type Target = MaterialInstanceAssetInner;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
