use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, ShaderAsset};
use hydrate_base::handle::Handle;
use hydrate_base::LoadHandle;
use rafx_api::RafxResult;
use rafx_framework::{ComputePipelineResource, ReflectedShader, ResourceArc};
use std::hash::Hash;
use std::path::PathBuf;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "52d1633a-baf3-4b4b-98a8-0d92c8d55af1"]
pub struct ComputePipelineRon {
    //TODO: This could be a Ref<T> type? We would detect it as a path and map to object ID
    // we have to determine importer, maybe we fall back to supported_file_extensions()?
    pub shader_module: PathBuf,
    pub entry_name: String,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "cab18bf6-384b-4e1a-bf3d-95b778122388"]
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

pub struct ComputePipelineLoadHandler;

impl DefaultAssetTypeLoadHandler<ComputePipelineAssetData, ComputePipelineAsset>
    for ComputePipelineLoadHandler
{
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        asset_data: ComputePipelineAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<ComputePipelineAsset> {
        //
        // Get the shader module
        //
        let shader_module = asset_manager
            .latest_asset(&asset_data.shader_module)
            .unwrap();

        //
        // Find the reflection data in the shader module for the given entry point
        //
        let reflection_data = shader_module.find_reflection_data(
            &asset_data.entry_name,
            asset_manager.device_context().api_type(),
        );
        let reflection_data = reflection_data.ok_or_else(|| {
            let error_message = format!(
                "Load Compute Shader Failed - Pass refers to entry point named {}, but no matching reflection data was found",
                asset_data.entry_name
            );
            log::error!("{}", error_message);
            error_message
        })?;

        let reflected_shader = ReflectedShader::new(
            asset_manager.resources(),
            &[shader_module.shader_module.clone()],
            &[&reflection_data],
        )?;
        let shader_module_resource = shader_module.shader_module.get_raw();
        let debug_name = shader_module_resource.shader_package.debug_name.as_deref();
        let compute_pipeline =
            reflected_shader.load_compute_pipeline(asset_manager.resources(), debug_name)?;

        Ok(ComputePipelineAsset { compute_pipeline })
    }
}

pub type ComputePipelineAssetTypeHandler = DefaultAssetTypeHandler<
    ComputePipelineAssetData,
    ComputePipelineAsset,
    ComputePipelineLoadHandler,
>;
