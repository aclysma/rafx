use fnv::FnvHashMap;
use rafx_resources::{ReflectedEntryPoint, ShaderModuleResource};
use rafx_resources::{ResourceArc, ShaderModuleResourceDef};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "e0ae2222-1a44-4022-af95-03c9101ac89e"]
pub struct ShaderAssetData {
    pub shader: ShaderModuleResourceDef,
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
