use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::{AssetManager, ImageAsset, ShaderAsset};
use ash::prelude::VkResult;
use ash::vk;
use atelier_assets::loader::handle::Handle;
use fnv::FnvHashMap;
use renderer_nodes::{RenderPhase, RenderPhaseIndex};
pub use renderer_resources::DescriptorSetLayoutResource;
pub use renderer_resources::GraphicsPipelineResource;
pub use renderer_resources::PipelineLayoutResource;
use renderer_resources::ShaderModuleResource;
use renderer_resources::{vk_description as dsc, DescriptorSetArc, ResourceArc};
use renderer_resources::{DescriptorSetWriteSet, MaterialPassResource};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "366d277d-6cb5-430a-a8fa-007d8ae69886"]
pub struct RenderpassAssetData {
    pub renderpass: dsc::RenderPass,
}

#[derive(TypeUuid, Clone)]
#[uuid = "bfefdc09-1ba6-422a-9514-b59b5b913128"]
pub struct RenderpassAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub renderpass_def: Arc<dsc::RenderPass>,
    // Renderpass assets can produce multiple renderpass resources depending on number of active
    // swapchains. So they are not available here. Use get_or_create_renderpass in resource lookup
    // to fetch the one that matches the SwapchainSurfaceInfo you have
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "0dfa5d9a-89cd-40a1-adac-baf801db61db"]
pub struct PipelineAssetData {
    pub input_assembly_state: dsc::PipelineInputAssemblyState,
    pub viewport_state: dsc::PipelineViewportState,
    pub rasterization_state: dsc::PipelineRasterizationState,
    pub multisample_state: dsc::PipelineMultisampleState,
    pub color_blend_state: dsc::PipelineColorBlendState,
    pub dynamic_state: dsc::PipelineDynamicState,
    pub depth_stencil_state: dsc::PipelineDepthStencilState,
}

// The actual GPU resources are held in Material because the pipeline does not specify everything
// needed to create the pipeline
#[derive(TypeUuid, Clone)]
#[uuid = "7a6a7ba8-a3ca-41eb-94f4-5d3723cd8b44"]
pub struct PipelineAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub pipeline_asset: Arc<PipelineAssetData>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PipelineShaderStage {
    pub stage: dsc::ShaderStageFlags,
    pub shader_module: Handle<ShaderAsset>,
    pub entry_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutBindingWithSlotName {
    pub binding: u32,
    pub descriptor_type: dsc::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: dsc::ShaderStageFlags,
    pub slot_name: String,

    pub immutable_samplers: Option<Vec<dsc::Sampler>>,
    pub internal_buffer_per_descriptor_size: Option<u32>,
}

impl Into<dsc::DescriptorSetLayoutBinding> for &DescriptorSetLayoutBindingWithSlotName {
    fn into(self) -> dsc::DescriptorSetLayoutBinding {
        dsc::DescriptorSetLayoutBinding {
            binding: self.binding,
            descriptor_type: self.descriptor_type,
            descriptor_count: self.descriptor_count,
            stage_flags: self.stage_flags,
            immutable_samplers: self.immutable_samplers.clone(),
            internal_buffer_per_descriptor_size: self.internal_buffer_per_descriptor_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutWithSlotName {
    pub descriptor_set_layout_bindings: Vec<DescriptorSetLayoutBindingWithSlotName>,
}

impl Into<dsc::DescriptorSetLayout> for &DescriptorSetLayoutWithSlotName {
    fn into(self) -> dsc::DescriptorSetLayout {
        let descriptor_set_layout_bindings = self
            .descriptor_set_layout_bindings
            .iter()
            .map(|x| x.into())
            .collect();
        dsc::DescriptorSetLayout {
            descriptor_set_layout_bindings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PushConstantRangeWithSlotName {
    pub stage_flags: dsc::ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
    pub slot_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub struct MaterialPassShaderInterface {
    pub descriptor_set_layouts: Vec<DescriptorSetLayoutWithSlotName>,
    pub push_constant_ranges: Vec<dsc::PushConstantRange>,
    pub vertex_input_state: dsc::PipelineVertexInputState,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MaterialPassDataRenderpassRef {
    Asset(Handle<RenderpassAsset>),
    LookupByPhaseName,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialPassData {
    pub name: Option<String>,
    pub phase: Option<String>,
    pub pipeline: Handle<PipelineAsset>,
    //pub renderpass: MaterialPassDataRenderpassRef,
    pub shaders: Vec<PipelineShaderStage>,
    pub shader_interface: MaterialPassShaderInterface,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "ad94bca2-1f02-4e5f-9117-1a7b03456a11"]
pub struct MaterialAssetData {
    pub passes: Vec<MaterialPassData>,
}

pub struct SlotLocation {
    pub layout_index: u32,
    pub binding_index: u32,
    //pub array_index: u32,
}

pub type SlotNameLookup = FnvHashMap<String, Vec<SlotLocation>>;

pub struct MaterialPassInner {
    pub shader_modules: Vec<ResourceArc<ShaderModuleResource>>,
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,

    // Info required to recreate the pipeline for new swapchains
    pub material_pass_resource: ResourceArc<MaterialPassResource>,

    //descriptor_set_factory: DescriptorSetFactory,
    pub shader_interface: MaterialPassShaderInterface,

    //TODO: Use hash instead of string. Probably want to have a "hashed string" type that keeps the
    // string around only in debug mode. Maybe this could be generalized to a HashOfThing<T>.
    pub pass_slot_name_lookup: Arc<SlotNameLookup>,
}

#[derive(Clone)]
pub struct MaterialPass {
    inner: Arc<MaterialPassInner>,
}

impl MaterialPass {
    pub fn new(
        asset_manager: &AssetManager,
        material_pass_data: &MaterialPassData,
    ) -> VkResult<MaterialPass> {
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

        let fixed_function_state = Arc::new(dsc::FixedFunctionState {
            vertex_input_state: material_pass_data
                .shader_interface
                .vertex_input_state
                .clone(),
            input_assembly_state: pipeline_asset.input_assembly_state.clone(),
            viewport_state: pipeline_asset.viewport_state.clone(),
            rasterization_state: pipeline_asset.rasterization_state.clone(),
            multisample_state: pipeline_asset.multisample_state.clone(),
            color_blend_state: pipeline_asset.color_blend_state.clone(),
            dynamic_state: pipeline_asset.dynamic_state.clone(),
            depth_stencil_state: pipeline_asset.depth_stencil_state.clone(),
        });

        //
        // Shaders
        //
        let mut shader_module_metas = Vec::with_capacity(material_pass_data.shaders.len());
        let mut shader_modules = Vec::with_capacity(material_pass_data.shaders.len());
        for stage in &material_pass_data.shaders {
            let shader_module_meta = dsc::ShaderModuleMeta {
                stage: stage.stage,
                entry_name: stage.entry_name.clone(),
            };
            shader_module_metas.push(shader_module_meta);

            let shader_module = asset_manager
                .loaded_assets()
                .shader_modules
                .get_latest(stage.shader_module.load_handle())
                .unwrap();
            shader_modules.push(shader_module.shader_module.clone());
        }

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layouts = Vec::with_capacity(
            material_pass_data
                .shader_interface
                .descriptor_set_layouts
                .len(),
        );
        let mut descriptor_set_layout_defs = Vec::with_capacity(
            material_pass_data
                .shader_interface
                .descriptor_set_layouts
                .len(),
        );
        for descriptor_set_layout_def in &material_pass_data.shader_interface.descriptor_set_layouts
        {
            let descriptor_set_layout_def = descriptor_set_layout_def.into();
            let descriptor_set_layout = asset_manager
                .resources()
                .get_or_create_descriptor_set_layout(&descriptor_set_layout_def)?;
            descriptor_set_layouts.push(descriptor_set_layout);
            descriptor_set_layout_defs.push(descriptor_set_layout_def);
        }

        //
        // Pipeline layout
        //
        let pipeline_layout_def = dsc::PipelineLayout {
            descriptor_set_layouts: descriptor_set_layout_defs,
            push_constant_ranges: material_pass_data
                .shader_interface
                .push_constant_ranges
                .clone(),
        };

        let pipeline_layout = asset_manager
            .resources()
            .get_or_create_pipeline_layout(&pipeline_layout_def)?;

        let material_pass = asset_manager.resources().get_or_create_material_pass(
            shader_modules.clone(),
            shader_module_metas,
            pipeline_layout.clone(),
            fixed_function_state,
        )?;

        //
        // If a phase name is specified, register the pass with the pipeline cache. The pipeline
        // cache is responsible for ensuring pipelines are created for renderpasses that execute
        // within the pipeline's phase
        //
        if let Some(phase_name) = &material_pass_data.phase {
            let renderphase_index = asset_manager
                .graphics_pipeline_cache()
                .get_renderphase_by_name(phase_name);
            match renderphase_index {
                Some(renderphase_index) => asset_manager
                    .graphics_pipeline_cache()
                    .register_material_to_phase_index(&material_pass, renderphase_index),
                None => {
                    log::error!(
                            "Load Material Failed - Pass refers to phase name {}, but this phase name was not registered",
                            phase_name
                        );
                    return Err(vk::Result::ERROR_UNKNOWN);
                }
            }
        }

        // Create a lookup of the slot names
        let mut pass_slot_name_lookup: SlotNameLookup = Default::default();
        for (layout_index, layout) in material_pass_data
            .shader_interface
            .descriptor_set_layouts
            .iter()
            .enumerate()
        {
            for (binding_index, binding) in layout.descriptor_set_layout_bindings.iter().enumerate()
            {
                pass_slot_name_lookup
                    .entry(binding.slot_name.clone())
                    .or_default()
                    .push(SlotLocation {
                        layout_index: layout_index as u32,
                        binding_index: binding_index as u32,
                        //array_index: 0
                    });
            }
        }

        let inner = MaterialPassInner {
            descriptor_set_layouts,
            pipeline_layout,
            shader_modules,
            //per_swapchain_data: Mutex::new(per_swapchain_data),
            material_pass_resource: material_pass.clone(),
            shader_interface: material_pass_data.shader_interface.clone(),
            pass_slot_name_lookup: Arc::new(pass_slot_name_lookup),
        };

        Ok(MaterialPass {
            inner: Arc::new(inner),
        })
    }

    pub fn create_uninitialized_write_sets_for_material_pass(&self) -> Vec<DescriptorSetWriteSet> {
        // The metadata for the descriptor sets within this pass, one for each set within the pass
        let descriptor_set_layouts = &self.shader_interface.descriptor_set_layouts;

        let pass_descriptor_set_writes: Vec<_> = descriptor_set_layouts
            .iter()
            .map(|layout| {
                renderer_resources::descriptor_sets::create_uninitialized_write_set_for_layout(
                    &layout.into(),
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
    pub sampler: Option<dsc::Sampler>,

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
    pub material_descriptor_sets: Arc<Vec<Vec<DescriptorSetArc>>>,
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
        material_descriptor_sets: Arc<Vec<Vec<DescriptorSetArc>>>,
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
