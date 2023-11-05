use crate::assets::mesh_basic::MeshBasicMaterialData;
use hydrate_base::handle::Handle;
use hydrate_base::LoadHandle;
use rafx::api::RafxResult;
use rafx::assets::{
    AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, ImageAsset,
    MaterialInstanceAsset,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::MeshBasicAsset;

#[derive(Serialize, Deserialize, Clone)]
pub struct MeshMaterialBasicAssetDataLod {
    pub mesh: Handle<MeshBasicAsset>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "41ea076f-19d7-4deb-8af1-983148af5383"]
pub struct MeshMaterialBasicAssetData {
    pub material_instance: Handle<MaterialInstanceAsset>,
    pub material_data: MeshBasicMaterialData,
    pub color_texture: Option<Handle<ImageAsset>>,
    pub metallic_roughness_texture: Option<Handle<ImageAsset>>,
    pub normal_texture: Option<Handle<ImageAsset>>,
    pub emissive_texture: Option<Handle<ImageAsset>>,
    //shader_data: MeshBasicMaterialDataShaderParam,
    //color_texture: json_format.color_texture,
    //matallic_roughness_texture: json_format.matallic_roughness_texture,
    //normal_texture: json_format.normal_texture,
    //emissive_texture: json_format.emissive_texture,
}

pub struct MeshMaterialBasicAssetInner {
    pub data: MeshMaterialBasicAssetData,
    pub material_instance: MaterialInstanceAsset,
}

#[derive(TypeUuid, Clone)]
#[uuid = "907915b3-cfdf-4e2c-b23c-b66b1049957a"]
pub struct MeshMaterialBasicAsset {
    pub inner: Arc<MeshMaterialBasicAssetInner>,
}

impl MeshMaterialBasicAsset {
    pub fn data(&self) -> &MeshMaterialBasicAssetData {
        &self.inner.data
    }

    pub fn material_instance(&self) -> &MaterialInstanceAsset {
        &self.inner.material_instance
    }
}

pub struct MeshMaterialBasicLoadHandler;

impl DefaultAssetTypeLoadHandler<MeshMaterialBasicAssetData, MeshMaterialBasicAsset>
    for MeshMaterialBasicLoadHandler
{
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        asset_data: MeshMaterialBasicAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<MeshMaterialBasicAsset> {
        let material_instance = asset_manager
            .latest_asset(&asset_data.material_instance)
            .unwrap()
            .clone();

        let inner = MeshMaterialBasicAssetInner {
            data: asset_data,
            material_instance,
        };

        Ok(MeshMaterialBasicAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type MeshMaterialBasicAssetType = DefaultAssetTypeHandler<
    MeshMaterialBasicAssetData,
    MeshMaterialBasicAsset,
    MeshMaterialBasicLoadHandler,
>;
