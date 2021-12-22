use super::MeshAdvMaterialData;
use crate::assets::mesh_adv::MeshAdvMaterialDataShaderParam;
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::make_handle_from_str;
use distill::{core::AssetUuid, importer::ImportOp};
use rafx::assets::{ImageAsset, MaterialInstanceAssetData, MaterialInstanceSlotAssignment};
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(Serialize, Deserialize)]
struct MaterialJsonFileFormat {
    pub base_color_factor: [f32; 4],     // default: 1,1,1,1
    pub emissive_factor: [f32; 3],       // default: 0,0,0
    pub metallic_factor: f32,            // default: 1,
    pub roughness_factor: f32,           // default: 1,
    pub normal_texture_scale: f32,       // default: 1
    pub occlusion_texture_strength: f32, // default 1
    pub alpha_cutoff: f32,               // default 0.5

    #[serde(default)]
    color_texture: Option<Handle<ImageAsset>>,
    #[serde(default)]
    pub pbr_texture: Option<Handle<ImageAsset>>,
    #[serde(default)]
    pub normal_texture: Option<Handle<ImageAsset>>,
    #[serde(default)]
    pub emissive_texture: Option<Handle<ImageAsset>>,
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "8ecd5157-2703-4cdc-b8ec-7ba6fac29593"]
pub struct MeshAdvBlenderMaterialImporterState(Option<AssetUuid>);

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
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = MeshAdvBlenderMaterialImporterState(Some(id));

        let json_format: MaterialJsonFileFormat = serde_json::from_reader(source)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let material_handle = make_handle_from_str("680c6edd-8bed-407b-aea0-d0f6056093d6")?;

        let null_image_handle = make_handle_from_str("fc937369-cad2-4a00-bf42-5968f1210784")?;

        let material_data = MeshAdvMaterialData {
            base_color_factor: json_format.base_color_factor,
            emissive_factor: json_format.emissive_factor,
            metallic_factor: json_format.metallic_factor,
            roughness_factor: json_format.roughness_factor,
            normal_texture_scale: json_format.normal_texture_scale,
            occlusion_texture_strength: json_format.occlusion_texture_strength,
            alpha_cutoff: json_format.alpha_cutoff,
            has_base_color_texture: json_format.color_texture.is_some(),
            has_metallic_roughness_texture: json_format.pbr_texture.is_some(),
            has_normal_texture: json_format.normal_texture.is_some(),
            has_occlusion_texture: false,
            has_emissive_texture: json_format.emissive_texture.is_some(),
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
            &json_format.pbr_texture,
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
            "occlusion_texture",
            &mut slot_assignments,
            material_data.has_occlusion_texture,
            &json_format.pbr_texture,
            &null_image_handle,
        );
        push_image_slot_assignment(
            "emissive_texture",
            &mut slot_assignments,
            material_data.has_emissive_texture,
            &json_format.emissive_texture,
            &null_image_handle,
        );

        let asset_data = MaterialInstanceAssetData {
            material: material_handle.clone(),
            slot_assignments,
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(asset_data),
            }],
        })
    }
}
