use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;

use crate::pipeline_description as dsc;
use atelier_assets::loader::handle::Handle;
use crate::pipeline::shader::ShaderAsset;
use crate::pipeline::image::ImageAsset;
use std::hash::{Hash, Hasher};
use crate::pipeline_description::{DescriptorSetLayoutBinding, DescriptorSetLayout};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub struct PipelineShaderStage {
    pub stage: dsc::ShaderStageFlags,
    pub shader_module: Handle<ShaderAsset>,
    pub entry_name: String,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "effdd6a3-71b9-4ac0-990d-770b1d7ac7e1"]
pub struct PipelineAsset {
    //WARNING: These are hashed in order to deduplicate pipelines that are functionally the same.
    // If you add a field that doesn't functionally alter the pipeline, rethink how we hash this
    // resource
    pub pipeline_layout: dsc::PipelineLayout,
    pub renderpass: dsc::RenderPass,
    pub fixed_function_state: dsc::FixedFunctionState,
    pub pipeline_shader_stages: Vec<PipelineShaderStage>,
}












#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "0dfa5d9a-89cd-40a1-adac-baf801db61db"]
pub struct PipelineAsset2 {
    pub renderpass: dsc::RenderPass,
    pub input_assembly_state: dsc::PipelineInputAssemblyState,
    pub viewport_state: dsc::PipelineViewportState,
    pub rasterization_state: dsc::PipelineRasterizationState,
    pub multisample_state: dsc::PipelineMultisampleState,
    pub color_blend_state: dsc::PipelineColorBlendState,
    pub dynamic_state: dsc::PipelineDynamicState,
}






#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutBindingWithSlotName {
    pub binding: u32,
    pub descriptor_type: dsc::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: dsc::ShaderStageFlags,
    //samplers: Vec<dsc::Sampler>,
    pub slot_name: String
}

impl Into<dsc::DescriptorSetLayoutBinding> for &DescriptorSetLayoutBindingWithSlotName {
    fn into(self) -> dsc::DescriptorSetLayoutBinding {
        dsc::DescriptorSetLayoutBinding {
            binding: self.binding,
            descriptor_type: self.descriptor_type,
            descriptor_count: self.descriptor_count,
            stage_flags: self.stage_flags
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutWithSlotName {
    pub descriptor_set_layout_bindings: Vec<DescriptorSetLayoutBindingWithSlotName>,
}

impl Into<dsc::DescriptorSetLayout> for &DescriptorSetLayoutWithSlotName {
    fn into(self) -> dsc::DescriptorSetLayout {
        let descriptor_set_layout_bindings = self.descriptor_set_layout_bindings.iter().map(|x| x.into()).collect();
        dsc::DescriptorSetLayout {
            descriptor_set_layout_bindings
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PushConstantRangeWithSlotName {
    pub stage_flags: dsc::ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
    pub slot_name: String
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
    pub pipeline: Handle<PipelineAsset2>,
    pub shaders: Vec<PipelineShaderStage>,
    pub shader_interface: MaterialPassShaderInterface,
}



#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "ad94bca2-1f02-4e5f-9117-1a7b03456a11"]
pub struct MaterialAsset2 {
    pub passes: Vec<MaterialPass>,
}









#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ImageSlotValue {
    slot_name: String,
    image: Handle<ImageAsset>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ScalarSlotValue {
    slot_name: String,
    value: f32
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "0d8cacf7-79df-4aa6-b99e-659a9c3b5e6b"]
pub struct MaterialInstanceAsset2 {
    pub material: Handle<MaterialAsset2>,
    pub image_slots: Vec<ImageSlotValue>,
}


