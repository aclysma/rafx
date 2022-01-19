use crate::assets::mesh_adv::MeshAdvMaterialData;
use distill::loader::handle::Handle;
use rafx::api::RafxResult;
use rafx::assets::{
    AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, ImageAsset,
    MaterialInstanceAsset,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::MeshAdvAsset;

#[derive(Serialize, Deserialize, Clone)]
pub struct MeshMaterialAdvAssetDataLod {
    pub mesh: Handle<MeshAdvAsset>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "41ea076f-19d7-4deb-8af1-983148af5383"]
pub struct MeshMaterialAdvAssetData {
    pub material_instance: Handle<MaterialInstanceAsset>,
    pub material_data: MeshAdvMaterialData,
    pub color_texture: Option<Handle<ImageAsset>>,
    pub metallic_roughness_texture: Option<Handle<ImageAsset>>,
    pub normal_texture: Option<Handle<ImageAsset>>,
    pub emissive_texture: Option<Handle<ImageAsset>>,
    //shader_data: MeshAdvMaterialDataShaderParam,
    //color_texture: json_format.color_texture,
    //matallic_roughness_texture: json_format.matallic_roughness_texture,
    //normal_texture: json_format.normal_texture,
    //emissive_texture: json_format.emissive_texture,
}

pub struct MeshMaterialAdvAssetInner {
    pub data: MeshMaterialAdvAssetData,
    pub material_instance: MaterialInstanceAsset,
}

#[derive(TypeUuid, Clone)]
#[uuid = "ff52550c-a599-4a27-820b-f6ee4caebd8a"]
pub struct MeshMaterialAdvAsset {
    pub inner: Arc<MeshMaterialAdvAssetInner>,
}

impl MeshMaterialAdvAsset {
    pub fn data(&self) -> &MeshMaterialAdvAssetData {
        &self.inner.data
    }

    pub fn material_instance(&self) -> &MaterialInstanceAsset {
        &self.inner.material_instance
    }
}

pub struct MeshMaterialAdvLoadHandler;

impl DefaultAssetTypeLoadHandler<MeshMaterialAdvAssetData, MeshMaterialAdvAsset>
    for MeshMaterialAdvLoadHandler
{
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        asset_data: MeshMaterialAdvAssetData,
    ) -> RafxResult<MeshMaterialAdvAsset> {
        println!("LOAD MeshMaterialAdvAsset");
        let material_instance = asset_manager
            .latest_asset(&asset_data.material_instance)
            .unwrap()
            .clone();

        let inner = MeshMaterialAdvAssetInner {
            data: asset_data,
            material_instance,
        };

        Ok(MeshMaterialAdvAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type MeshMaterialAdvAssetType = DefaultAssetTypeHandler<
    MeshMaterialAdvAssetData,
    MeshMaterialAdvAsset,
    MeshMaterialAdvLoadHandler,
>;
