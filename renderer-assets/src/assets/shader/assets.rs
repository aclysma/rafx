use renderer_resources::ShaderModuleResource;
use renderer_resources::{vk_description as dsc, ResourceArc};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use fnv::FnvHashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedDescriptorSetLayoutBinding {
    pub name: String,
    pub binding: u32,
    pub descriptor_type: dsc::DescriptorType,
    // (array length, essentially)
    pub descriptor_count: u32,
    pub stage_flags: dsc::ShaderStageFlags,

    // Mostly for uniform data
    pub size: u32,
    //pub padded_size: u32,
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

// #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
// pub struct ReflectedShaderReflectionData {
//     // These are indexed by descriptor set index (i.e. not sparse)
//     pub descriptor_sets: Vec<Option<ReflectedDescriptorSetLayout>>,
//     pub input_variables: Vec<ReflectedInputVariable>,
//     pub output_variables: Vec<ReflectedOutputVariable>,
// }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReflectedEntryPoint {
    pub name: String,
    pub stage_flags: dsc::ShaderStageFlags,
    // These are indexed by descriptor set index (i.e. not sparse)
    pub descriptor_set_layouts: Vec<Option<ReflectedDescriptorSetLayout>>,
    pub input_variables: Vec<ReflectedInputVariable>,
    pub output_variables: Vec<ReflectedOutputVariable>,
    pub push_constants: Vec<ReflectedPushConstant>,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "e0ae2222-1a44-4022-af95-03c9101ac89e"]
pub struct ShaderAssetData {
    pub shader: dsc::ShaderModule,
    pub reflection_data: Vec<ReflectedEntryPoint>
}

//
// The "loaded" state of assets. Assets may have dependencies. Arcs to those dependencies ensure
// they do not get destroyed. All of the raw resources are hashed to avoid duplicating anything that
// is functionally identical. So for example if you have two windows with identical swapchain
// surfaces, they could share the same renderpass/pipeline resources
//
#[derive(TypeUuid, Clone)]
#[uuid = "b6958faa-5769-4048-a507-f91a07f49af4"]
pub struct ShaderAsset {
    pub shader_module: ResourceArc<ShaderModuleResource>,
    pub reflection_data: Arc<FnvHashMap<String, ReflectedEntryPoint>>,
}
