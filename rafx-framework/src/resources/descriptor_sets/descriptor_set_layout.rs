use serde::{Deserialize, Serialize};

use rafx_api::{RafxSamplerDef, RafxShaderResource};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutBinding {
    // Basic info required to create the RafxRootSignature
    pub resource: RafxShaderResource,

    // Samplers created here will be automatically created/bound
    pub immutable_samplers: Option<Vec<RafxSamplerDef>>,

    // If this is non-zero we will allocate a buffer owned by the descriptor set pool chunk,
    // and automatically bind it - this makes binding data easy to do without having to manage
    // buffers.
    pub internal_buffer_per_descriptor_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayout {
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayout {
    pub fn new() -> Self {
        DescriptorSetLayout {
            bindings: Default::default(),
        }
    }
}
