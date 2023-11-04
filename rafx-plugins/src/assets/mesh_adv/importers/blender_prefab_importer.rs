use crate::assets::mesh_adv::{
    BlenderModelImporter, ModelAdvAsset, PrefabAdvAssetData, PrefabAdvAssetDataObject,
    PrefabAdvAssetDataObjectLight, PrefabAdvAssetDataObjectLightKind,
    PrefabAdvAssetDataObjectLightSpot, PrefabAdvAssetDataObjectModel,
    PrefabAdvAssetDataObjectTransform,
};
use crate::schema::{
    MeshAdvModelAssetRecord, MeshAdvPrefabAssetRecord, MeshAdvPrefabImportDataRecord,
};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerMut, ImporterId, Record, SchemaSet};
use hydrate_model::{
    BuilderRegistryBuilder, ImportableObject, ImportedImportable, ImporterRegistry,
    ImporterRegistryBuilder, JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable,
    SchemaLinker,
};
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use type_uuid::*;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectTransform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectModel {
    pub model: Handle<ModelAdvAsset>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum MeshAdvPrefabJsonFormatObjectLightKind {
    Point,
    Spot,
    Directional,
}

impl Into<PrefabAdvAssetDataObjectLightKind> for MeshAdvPrefabJsonFormatObjectLightKind {
    fn into(self) -> PrefabAdvAssetDataObjectLightKind {
        match self {
            MeshAdvPrefabJsonFormatObjectLightKind::Point => {
                PrefabAdvAssetDataObjectLightKind::Point
            }
            MeshAdvPrefabJsonFormatObjectLightKind::Spot => PrefabAdvAssetDataObjectLightKind::Spot,
            MeshAdvPrefabJsonFormatObjectLightKind::Directional => {
                PrefabAdvAssetDataObjectLightKind::Directional
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectLightSpot {
    pub inner_angle: f32,
    pub outer_angle: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectLight {
    pub color: [f32; 3],
    pub kind: MeshAdvPrefabJsonFormatObjectLightKind,
    pub intensity: f32,
    pub cutoff_distance: Option<f32>,
    pub spot: Option<MeshAdvPrefabJsonFormatObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObject {
    pub transform: MeshAdvPrefabJsonFormatObjectTransform,
    pub model: Option<MeshAdvPrefabJsonFormatObjectModel>,
    pub light: Option<MeshAdvPrefabJsonFormatObjectLight>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormat {
    pub objects: Vec<MeshAdvPrefabJsonFormatObject>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HydrateMeshAdvPrefabJsonFormatObjectModel {
    pub model: PathBuf, //hydrate_base::Handle<ModelAdvAsset>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HydrateMeshAdvPrefabJsonFormatObject {
    pub transform: MeshAdvPrefabJsonFormatObjectTransform,
    pub model: Option<HydrateMeshAdvPrefabJsonFormatObjectModel>,
    pub light: Option<MeshAdvPrefabJsonFormatObjectLight>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HydrateMeshAdvPrefabJsonFormat {
    pub objects: Vec<HydrateMeshAdvPrefabJsonFormatObject>,
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "5f9022bc-fd83-4f99-9fb7-a395fd997361"]
pub struct MeshAdvBlenderPrefabImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "1441f5a2-5c3b-404b-b03f-2234146e2c2f"]
pub struct MeshAdvBlenderPrefabImporter;
impl Importer for MeshAdvBlenderPrefabImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        4
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MeshAdvBlenderPrefabImporterState;

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
        *state = MeshAdvBlenderPrefabImporterState(Some(id));

        let json_format: MeshAdvPrefabJsonFormat = serde_json::from_reader(source)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let mut objects = Vec::with_capacity(json_format.objects.len());
        for json_object in json_format.objects {
            let model = if let Some(json_model) = &json_object.model {
                let model = json_model.model.clone();

                Some(PrefabAdvAssetDataObjectModel { model })
            } else {
                None
            };

            let light = if let Some(json_light) = &json_object.light {
                let light = json_light.clone();
                let spot = light
                    .spot
                    .as_ref()
                    .map(|x| PrefabAdvAssetDataObjectLightSpot {
                        inner_angle: x.inner_angle,
                        outer_angle: x.outer_angle,
                    });

                let range = if light.cutoff_distance.unwrap_or(-1.0) < 0.0 {
                    None
                } else {
                    light.cutoff_distance
                };
                Some(PrefabAdvAssetDataObjectLight {
                    color: light.color.into(),
                    kind: light.kind.into(),
                    intensity: light.intensity,
                    range,
                    spot,
                })
            } else {
                None
            };

            let transform = PrefabAdvAssetDataObjectTransform {
                position: json_object.transform.position.into(),
                rotation: json_object.transform.rotation.into(),
                scale: json_object.transform.scale.into(),
            };

            objects.push(PrefabAdvAssetDataObject {
                transform,
                model,
                light,
            });
        }

        let asset_data = PrefabAdvAssetData { objects };

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

#[derive(TypeUuid, Default)]
#[uuid = "a40a442f-285e-4bb8-81f4-43d761b9f140"]
pub struct BlenderPrefabImporter;

impl hydrate_model::Importer for BlenderPrefabImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_prefab"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
        importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(path).unwrap();
        let json_format: HydrateMeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))
            .unwrap();

        let asset_type = schema_set
            .find_named_type(MeshAdvPrefabAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let model_importer_id = ImporterId(Uuid::from_bytes(BlenderModelImporter::UUID));

        for object in &json_format.objects {
            if let Some(model) = &object.model {
                file_references.push(ReferencedSourceFile {
                    importer_id: model_importer_id,
                    path: model.model.clone(),
                })
            }
        }

        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references,
        }]
    }

    fn import_file(
        &self,
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(path).unwrap();
        // We don't actually need to parse this now but worth doing to make sure it's well-formed at import time
        let _json_format: HydrateMeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))
            .unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object =
                MeshAdvPrefabAssetRecord::new_single_object(schema_set).unwrap();
            // let mut default_asset_data_container =
            //     DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            // let x = MeshAdvPrefabAssetRecord::default();

            // No fields to write
            default_asset_object
        };

        let import_data = {
            let mut import_data_object =
                MeshAdvPrefabImportDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_data_container =
                DataContainerMut::new_single_object(&mut import_data_object, schema_set);
            let x = MeshAdvPrefabImportDataRecord::default();

            x.json_data()
                .set(&mut import_data_data_container, source)
                .unwrap();

            // No fields to write
            import_data_object
        };

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data),
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}

pub struct BlenderPrefabAssetPlugin;

impl hydrate_model::AssetPlugin for BlenderPrefabAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<BlenderPrefabImporter>(schema_linker);
    }
}
