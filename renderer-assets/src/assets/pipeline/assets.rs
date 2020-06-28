use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::vk_description as dsc;
use atelier_assets::loader::handle::Handle;
use crate::assets::shader::ShaderAssetData;
use crate::assets::image::ImageAssetData;
use std::hash::Hash;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "366d277d-6cb5-430a-a8fa-007d8ae69886"]
pub struct RenderpassAssetData {
    pub renderpass: dsc::RenderPass,
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

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub struct PipelineShaderStage {
    pub stage: dsc::ShaderStageFlags,
    pub shader_module: Handle<ShaderAssetData>,
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

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub struct MaterialPass {
    pub phase: String,
    pub pipeline: Handle<PipelineAssetData>,
    pub renderpass: Handle<RenderpassAssetData>,
    pub shaders: Vec<PipelineShaderStage>,
    pub shader_interface: MaterialPassShaderInterface,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "ad94bca2-1f02-4e5f-9117-1a7b03456a11"]
pub struct MaterialAssetData {
    pub passes: Vec<MaterialPass>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialInstanceSlotAssignment {
    pub slot_name: String,
    pub image: Option<Handle<ImageAssetData>>,
    pub sampler: Option<dsc::Sampler>,

    // Would be nice to use this, but I don't think it works with Option
    //#[serde(with = "serde_bytes")]
    pub buffer_data: Option<Vec<u8>>,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "0d8cacf7-79df-4aa6-b99e-659a9c3b5e6b"]
pub struct MaterialInstanceAssetData {
    pub material: Handle<MaterialAssetData>,
    pub slot_assignments: Vec<MaterialInstanceSlotAssignment>,
}
