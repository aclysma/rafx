use super::MeshAdvMaterialData;
use crate::assets::mesh_adv::{MeshAdvBlendMethod, MeshAdvShadowMethod, MeshMaterialAdvAssetData};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::make_handle_from_str;
use distill::{core::AssetUuid, importer::ImportOp};
use rafx::assets::ImageAsset;
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(Serialize, Deserialize)]
struct MaterialJsonFileFormat {
    pub base_color_factor: [f32; 4], // default: 1,1,1,1
    pub emissive_factor: [f32; 3],   // default: 0,0,0
    pub metallic_factor: f32,        // default: 1,
    pub roughness_factor: f32,       // default: 1,
    pub normal_texture_scale: f32,   // default: 1

    #[serde(default)]
    pub color_texture: Option<Handle<ImageAsset>>,
    #[serde(default)]
    pub metallic_roughness_texture: Option<Handle<ImageAsset>>,
    #[serde(default)]
    pub normal_texture: Option<Handle<ImageAsset>>,
    #[serde(default)]
    pub emissive_texture: Option<Handle<ImageAsset>>,

    #[serde(default)]
    pub shadow_method: Option<String>,
    #[serde(default)]
    pub blend_method: Option<String>,
    #[serde(default)]
    pub alpha_threshold: Option<f32>,
    #[serde(default)]
    pub backface_culling: Option<bool>,
    #[serde(default)]
    pub color_texture_has_alpha_channel: bool,
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "8ecd5157-2703-4cdc-b8ec-7ba6fac29593"]
pub struct MeshAdvBlenderMaterialImporterState {
    pub mesh_material_id: Option<AssetUuid>,
    pub material_instance_id: Option<AssetUuid>,
}

#[derive(TypeUuid)]
#[uuid = "f358cd88-b79c-4439-83bb-501807d89cd3"]
pub struct MeshAdvBlenderMaterialImporter;
impl Importer for MeshAdvBlenderMaterialImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        3
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MeshAdvBlenderMaterialImporterState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let material_instance_id = state
            .material_instance_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        let mesh_material_id = state
            .mesh_material_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

        *state = MeshAdvBlenderMaterialImporterState {
            material_instance_id: Some(material_instance_id),
            mesh_material_id: Some(mesh_material_id),
        };

        let json_format: MaterialJsonFileFormat = serde_json::from_reader(source)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let material_handle = make_handle_from_str("680c6edd-8bed-407b-aea0-d0f6056093d6")?;

        let shadow_method = match json_format.shadow_method.as_deref() {
            None => MeshAdvShadowMethod::Opaque,
            Some("NONE") => MeshAdvShadowMethod::None,
            Some("OPAQUE") => MeshAdvShadowMethod::Opaque,
            _ => unimplemented!(), //"CLIP" => MeshAdvShadowMethod::AlphaClip,
                                   //"HASHED" => MeshAdvShadowMethod::AlphaStochastic
        };

        let blend_method = match json_format.blend_method.as_deref() {
            None => MeshAdvBlendMethod::Opaque,
            Some("OPAQUE") => MeshAdvBlendMethod::Opaque,
            Some("CLIP") => MeshAdvBlendMethod::AlphaClip,
            Some("BLEND") => MeshAdvBlendMethod::AlphaBlend,
            _ => unimplemented!(), //Some("HASHED") => MeshAdvBlendMethod::AlphaStochastic,
        };

        let material_data = MeshAdvMaterialData {
            base_color_factor: json_format.base_color_factor,
            emissive_factor: json_format.emissive_factor,
            metallic_factor: json_format.metallic_factor,
            roughness_factor: json_format.roughness_factor,
            normal_texture_scale: json_format.normal_texture_scale,
            has_base_color_texture: json_format.color_texture.is_some(),
            base_color_texture_has_alpha_channel: json_format.color_texture_has_alpha_channel,
            has_metallic_roughness_texture: json_format.metallic_roughness_texture.is_some(),
            has_normal_texture: json_format.normal_texture.is_some(),
            has_emissive_texture: json_format.emissive_texture.is_some(),
            shadow_method,
            blend_method,
            alpha_threshold: json_format.alpha_threshold.unwrap_or(0.5),
            backface_culling: json_format.backface_culling.unwrap_or(true),
        };

        let mesh_material_data = MeshMaterialAdvAssetData {
            material_data,
            material_asset: material_handle.clone(),
            color_texture: json_format.color_texture,
            metallic_roughness_texture: json_format.metallic_roughness_texture,
            normal_texture: json_format.normal_texture,
            emissive_texture: json_format.emissive_texture,
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: mesh_material_id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(mesh_material_data),
            }],
        })
    }
}
