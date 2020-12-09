use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::ShaderAsset;
use atelier_assets::loader::handle::Handle;
pub use rafx_resources::DescriptorSetLayoutResource;
pub use rafx_resources::GraphicsPipelineResource;
pub use rafx_resources::PipelineLayoutResource;
use rafx_resources::{ComputePipelineResource, ResourceArc};
use std::hash::Hash;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "e70aa3d2-5727-433a-80c2-4f6f1d01c91f"]
pub struct ComputePipelineAssetData {
    pub shader_module: Handle<ShaderAsset>,
    pub entry_name: String,
}

// The actual GPU resources are held in Material because the pipeline does not specify everything
// needed to create the pipeline
#[derive(TypeUuid, Clone)]
#[uuid = "d5673f07-c926-4e75-bab9-4e8b64e87f22"]
pub struct ComputePipelineAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub compute_pipeline: ResourceArc<ComputePipelineResource>,
}
