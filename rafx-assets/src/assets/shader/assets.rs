use crate::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use distill::loader::LoadHandle;
use fnv::FnvHashMap;
use rafx_api::{
    RafxApiType, RafxHashedShaderPackage, RafxReflectedEntryPoint, RafxResult, RafxShaderPackage,
    RAFX_VALID_API_TYPES,
};
use rafx_framework::{ResourceArc, ShaderModuleResource};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "e0ae2222-1a44-4022-af95-03c9101ac89e"]
pub struct ShaderAssetData {
    pub shader_package: RafxHashedShaderPackage,
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
    // Indexed by RafxApiType
    pub reflection_data: Arc<Vec<FnvHashMap<String, RafxReflectedEntryPoint>>>,
}

impl ShaderAsset {
    pub fn find_reflection_data(
        &self,
        entry_point_name: &str,
        api_type: RafxApiType,
    ) -> Option<&RafxReflectedEntryPoint> {
        self.reflection_data[api_type as usize].get(entry_point_name)
    }
}

pub struct ShaderLoadHandler;

// we get a warning from this when building without any backends, which occurs when recompiling
// shaders and regenerating corresponding rust code
#[allow(dead_code)]
fn build_reflection_data_map(
    reflection_data_lookup: &mut Vec<FnvHashMap<String, RafxReflectedEntryPoint>>,
    shader_package: &RafxShaderPackage,
    api_type: RafxApiType,
) {
    if let Some(reflection_data) = shader_package.reflection(api_type) {
        for entry_point in reflection_data {
            let old = reflection_data_lookup[api_type as usize].insert(
                entry_point.rafx_api_reflection.entry_point_name.clone(),
                entry_point.clone(),
            );
            assert!(old.is_none());
        }
    }
}

impl DefaultAssetTypeLoadHandler<ShaderAssetData, ShaderAsset> for ShaderLoadHandler {
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        asset_data: ShaderAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<ShaderAsset> {
        let mut reflection_data_lookup = Vec::with_capacity(RAFX_VALID_API_TYPES.len());
        reflection_data_lookup.resize_with(RAFX_VALID_API_TYPES.len(), Default::default);

        #[cfg(feature = "rafx-vulkan")]
        build_reflection_data_map(
            &mut reflection_data_lookup,
            asset_data.shader_package.shader_package(),
            RafxApiType::Vk,
        );
        #[cfg(feature = "rafx-dx12")]
        build_reflection_data_map(
            &mut reflection_data_lookup,
            asset_data.shader_package.shader_package(),
            RafxApiType::Dx12,
        );
        #[cfg(feature = "rafx-metal")]
        build_reflection_data_map(
            &mut reflection_data_lookup,
            asset_data.shader_package.shader_package(),
            RafxApiType::Metal,
        );
        #[cfg(feature = "rafx-gles2")]
        build_reflection_data_map(
            &mut reflection_data_lookup,
            asset_data.shader_package.shader_package(),
            RafxApiType::Gles2,
        );
        #[cfg(feature = "rafx-gles3")]
        build_reflection_data_map(
            &mut reflection_data_lookup,
            asset_data.shader_package.shader_package(),
            RafxApiType::Gles3,
        );

        let shader_module = asset_manager
            .resources()
            .get_or_create_shader_module_from_hashed_package(&asset_data.shader_package)?;

        Ok(ShaderAsset {
            shader_module,
            reflection_data: Arc::new(reflection_data_lookup),
        })
    }
}

pub type ShaderAssetTypeHandler =
    DefaultAssetTypeHandler<ShaderAssetData, ShaderAsset, ShaderLoadHandler>;
