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

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub struct PipelineShaderStage {
    pub stage: dsc::ShaderStageFlags,
    pub shader_module: Handle<ShaderAsset>,
    pub entry_name: String
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