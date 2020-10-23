use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::{
    vk_description as dsc, ImageAsset, ShaderAsset, DescriptorSetArc, ResourceArc,
    RenderPassResource,
};
use atelier_assets::loader::handle::Handle;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use crate::resources::{DescriptorSetWriteSet, MaterialPassResource};
pub use crate::resources::GraphicsPipelineResource;
pub use crate::resources::DescriptorSetLayoutResource;
pub use crate::resources::PipelineLayoutResource;
use crate::resources::ShaderModuleResource;
use fnv::FnvHashMap;
use ash::vk;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "366d277d-6cb5-430a-a8fa-007d8ae69886"]
pub struct RenderpassAssetData {
    pub renderpass: dsc::RenderPass,
}

#[derive(TypeUuid, Clone)]
#[uuid = "bfefdc09-1ba6-422a-9514-b59b5b913128"]
pub struct RenderpassAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub data: Arc<RenderpassAssetData>,
    // Renderpass assets can produce multiple renderpass resources depending on number of active
    // swapchains.
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

pub struct MaterialPassSwapchainResources {
    pub pipeline: ResourceArc<GraphicsPipelineResource>,
}

pub struct MaterialPass {
    pub shader_modules: Vec<ResourceArc<ShaderModuleResource>>,
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,

    // Potentially one of these per swapchain surface
    //pub per_swapchain_data: Mutex<Vec<MaterialPassSwapchainResources>>,

    // Info required to recreate the pipeline for new swapchains
    pub material_pass_resource: ResourceArc<MaterialPassResource>,

    //descriptor_set_factory: DescriptorSetFactory,
    pub shader_interface: MaterialPassShaderInterface,

    //TODO: Use hash instead of string. Probably want to have a "hashed string" type that keeps the
    // string around only in debug mode. Maybe this could be generalized to a HashOfThing<T>.
    pub pass_slot_name_lookup: Arc<SlotNameLookup>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "165673cd-d81d-4708-b9a4-d7e1a2a67976"]
pub struct MaterialAsset {
    //TODO: Consider making this named
    //TODO: Get cached graphics pipelines working
    //TODO: Could consider decoupling render cache from phases
    pub passes: Arc<Vec<MaterialPass>>,
    pub pass_phase_name_to_index: FnvHashMap<String, usize>,
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
    pub material: Handle<MaterialAsset>,

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
        material_descriptor_sets: Arc<Vec<Vec<DescriptorSetArc>>>,
        slot_assignments: Vec<MaterialInstanceSlotAssignment>,
        descriptor_set_writes: Vec<Vec<DescriptorSetWriteSet>>,
    ) -> Self {
        let inner = MaterialInstanceAssetInner {
            material,
            material_descriptor_sets,
            slot_assignments,
            descriptor_set_writes,
        };

        MaterialInstanceAsset {
            inner: Arc::new(inner),
        }
    }
}
