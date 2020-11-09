use renderer_resources::vk_description as dsc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedDescriptorSetLayoutBinding {
    pub name: String,
    pub set: u32,
    pub binding: u32,
    pub descriptor_type: dsc::DescriptorType,
    // (array length, essentially)
    pub descriptor_count: u32,
    pub stage_flags: dsc::ShaderStageFlags,

    // Mostly for uniform data
    pub size: u32,
    //pub padded_size: u32,


    pub internal_buffer_per_descriptor_size: Option<u32>,
    pub immutable_samplers: Option<Vec<dsc::Sampler>>,
    pub slot_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedDescriptorSetLayout {
    // These are NOT indexable by binding (i.e. may be sparse)
    pub bindings: Vec<ReflectedDescriptorSetLayoutBinding>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedInputVariable {
    pub name: String,
    pub location: u32,
    pub format: dsc::Format
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedOutputVariable {
    pub name: String,
    pub location: u32,
    pub format: dsc::Format
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedPushConstant {
    pub name: String,
    pub push_constant: dsc::PushConstantRange
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedEntryPoint {
    pub name: String,
    pub stage_flags: dsc::ShaderStageFlags,
    // These are indexed by descriptor set index (i.e. not sparse)
    pub descriptor_set_layouts: Vec<Option<ReflectedDescriptorSetLayout>>,
    // pub input_variables: Vec<ReflectedInputVariable>,
    // pub output_variables: Vec<ReflectedOutputVariable>,
    pub push_constants: Vec<ReflectedPushConstant>,
}
