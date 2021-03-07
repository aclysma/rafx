use crate::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use fnv::FnvHashMap;
use rafx_api::{RafxResult, RafxShaderPackage};
use rafx_framework::ResourceArc;
use rafx_framework::{ReflectedEntryPoint, ShaderModuleHash, ShaderModuleResource};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "e0ae2222-1a44-4022-af95-03c9101ac89e"]
pub struct ShaderAssetData {
    pub shader_module_hash: ShaderModuleHash,
    pub shader_package: RafxShaderPackage,
    pub reflection_data: Option<Vec<ReflectedEntryPoint>>,
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

pub struct ShaderLoadHandler;

impl DefaultAssetTypeLoadHandler<ShaderAssetData, ShaderAsset> for ShaderLoadHandler {
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        asset_data: ShaderAssetData,
    ) -> RafxResult<ShaderAsset> {
        let mut reflection_data_lookup = FnvHashMap::default();
        if let Some(reflection_data) = &asset_data.reflection_data {
            for entry_point in reflection_data {
                let old = reflection_data_lookup.insert(
                    entry_point.rafx_api_reflection.entry_point_name.clone(),
                    entry_point.clone(),
                );
                assert!(old.is_none());
            }
        }

        let shader_module = asset_manager.resources().get_or_create_shader_module(
            &asset_data.shader_package,
            Some(asset_data.shader_module_hash),
        )?;

        Ok(ShaderAsset {
            shader_module,
            reflection_data: Arc::new(reflection_data_lookup),
        })
    }
}

pub type ShaderAssetTypeHandler =
    DefaultAssetTypeHandler<ShaderAssetData, ShaderAsset, ShaderLoadHandler>;
