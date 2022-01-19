use super::MeshAdvMaterialData;
use crate::assets::mesh_adv::{
    MeshAdvBlendMethod, MeshAdvMaterialDataShaderParam, MeshAdvShadowMethod,
    MeshMaterialAdvAssetData,
};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::loader::AssetRef;
use distill::{core::AssetUuid, importer::ImportOp};
use distill::{make_handle, make_handle_from_str};
use rafx::assets::{ImageAsset, MaterialInstanceAssetData, MaterialInstanceSlotAssignment};
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

        let null_image_handle = make_handle_from_str("fc937369-cad2-4a00-bf42-5968f1210784")?;

        let shadow_method = match json_format.shadow_method.as_ref().map(|x| x.as_str()) {
            None => MeshAdvShadowMethod::Opaque,
            Some("NONE") => MeshAdvShadowMethod::None,
            Some("OPAQUE") => MeshAdvShadowMethod::Opaque,
            _ => unimplemented!(), //"CLIP" => MeshAdvShadowMethod::AlphaClip,
                                   //"HASHED" => MeshAdvShadowMethod::AlphaStochastic
        };

        let blend_method = match json_format.blend_method.as_ref().map(|x| x.as_str()) {
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

        let mut slot_assignments = vec![];

        let material_data_shader_param: MeshAdvMaterialDataShaderParam =
            material_data.clone().into();
        slot_assignments.push(MaterialInstanceSlotAssignment {
            slot_name: "per_material_data".to_string(),
            array_index: 0,
            image: None,
            sampler: None,
            buffer_data: Some(rafx::base::memory::any_as_bytes(&material_data_shader_param).into()),
        });

        fn push_image_slot_assignment(
            slot_name: &str,
            slot_assignments: &mut Vec<MaterialInstanceSlotAssignment>,
            should_include: bool,
            image: &Option<Handle<ImageAsset>>,
            default_image: &Handle<ImageAsset>,
        ) {
            slot_assignments.push(MaterialInstanceSlotAssignment {
                slot_name: slot_name.to_string(),
                array_index: 0,
                image: if should_include {
                    Some(image.as_ref().map_or(default_image, |x| x).clone())
                } else {
                    Some(default_image.clone())
                },
                sampler: None,
                buffer_data: None,
            });
        }

        push_image_slot_assignment(
            "base_color_texture",
            &mut slot_assignments,
            material_data.has_base_color_texture,
            &json_format.color_texture,
            &null_image_handle,
        );
        push_image_slot_assignment(
            "metallic_roughness_texture",
            &mut slot_assignments,
            material_data.has_metallic_roughness_texture,
            &json_format.metallic_roughness_texture,
            &null_image_handle,
        );
        push_image_slot_assignment(
            "normal_texture",
            &mut slot_assignments,
            material_data.has_normal_texture,
            &json_format.normal_texture,
            &null_image_handle,
        );
        push_image_slot_assignment(
            "emissive_texture",
            &mut slot_assignments,
            material_data.has_emissive_texture,
            &json_format.emissive_texture,
            &null_image_handle,
        );

        let material_instance_asset_data = MaterialInstanceAssetData {
            material: material_handle,
            slot_assignments,
        };

        let material_instance_handle = make_handle(material_instance_id);

        let mesh_material_data = MeshMaterialAdvAssetData {
            material_data,
            material_instance: material_instance_handle.clone(),
            color_texture: json_format.color_texture,
            metallic_roughness_texture: json_format.metallic_roughness_texture,
            normal_texture: json_format.normal_texture,
            emissive_texture: json_format.emissive_texture,
        };

        Ok(ImporterValue {
            assets: vec![
                ImportedAsset {
                    id: mesh_material_id,
                    search_tags: vec![],
                    build_deps: vec![],
                    load_deps: vec![AssetRef::Uuid(material_instance_id)],
                    build_pipeline: None,
                    asset_data: Box::new(mesh_material_data),
                },
                ImportedAsset {
                    id: material_instance_id,
                    search_tags: vec![],
                    build_deps: vec![],
                    load_deps: vec![],
                    build_pipeline: None,
                    asset_data: Box::new(material_instance_asset_data),
                },
            ],
        })
    }
}
