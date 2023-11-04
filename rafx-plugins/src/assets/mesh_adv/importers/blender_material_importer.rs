use super::MeshAdvMaterialData;
use crate::assets::mesh_adv::{MeshAdvBlendMethod, MeshAdvShadowMethod, MeshMaterialAdvAssetData};
use crate::schema::{MeshAdvBlendMethodEnum, MeshAdvMaterialAssetRecord, MeshAdvShadowMethodEnum};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::make_handle_from_str;
use distill::{core::AssetUuid, importer::ImportOp};
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerMut, Enum, ImporterId, ObjectRefField, Record, SchemaSet};
use hydrate_model::{
    AssetPlugin, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistryBuilder, JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable,
    SchemaLinker,
};
use rafx::assets::{GpuImageImporterSimple, ImageAsset};
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use type_uuid::*;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct HydrateMaterialJsonFileFormat {
    pub base_color_factor: [f32; 4], // default: 1,1,1,1
    pub emissive_factor: [f32; 3],   // default: 0,0,0
    pub metallic_factor: f32,        // default: 1,
    pub roughness_factor: f32,       // default: 1,
    pub normal_texture_scale: f32,   // default: 1

    #[serde(default)]
    pub color_texture: Option<PathBuf>,
    #[serde(default)]
    pub metallic_roughness_texture: Option<PathBuf>,
    #[serde(default)]
    pub normal_texture: Option<PathBuf>,
    #[serde(default)]
    pub emissive_texture: Option<PathBuf>,

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

#[derive(TypeUuid, Default)]
#[uuid = "e76bab79-654a-476f-93b1-88cd5fee7d1f"]
pub struct BlenderMaterialImporter;

impl hydrate_model::Importer for BlenderMaterialImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_material"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
    ) -> Vec<ScannedImportable> {
        let asset_type = schema_set
            .find_named_type(MeshAdvMaterialAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let json_str = std::fs::read_to_string(path).unwrap();
        let json_data: HydrateMaterialJsonFileFormat = serde_json::from_str(&json_str).unwrap();

        let mut file_references: Vec<ReferencedSourceFile> = Default::default();

        fn try_add_file_reference<T: TypeUuid>(
            file_references: &mut Vec<ReferencedSourceFile>,
            path_as_string: &Option<PathBuf>,
        ) {
            let importer_image_id = ImporterId(Uuid::from_bytes(T::UUID));
            if let Some(path_as_string) = path_as_string {
                file_references.push(ReferencedSourceFile {
                    importer_id: importer_image_id,
                    path: path_as_string.clone(),
                })
            }
        }

        //TODO: We assume a particular importer but we should probably just say what kind of imported
        // data or asset we would use?
        try_add_file_reference::<GpuImageImporterSimple>(
            &mut file_references,
            &json_data.color_texture,
        );
        try_add_file_reference::<GpuImageImporterSimple>(
            &mut file_references,
            &json_data.metallic_roughness_texture,
        );
        try_add_file_reference::<GpuImageImporterSimple>(
            &mut file_references,
            &json_data.normal_texture,
        );
        try_add_file_reference::<GpuImageImporterSimple>(
            &mut file_references,
            &json_data.emissive_texture,
        );

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
        let json_str = std::fs::read_to_string(path).unwrap();
        let json_data: HydrateMaterialJsonFileFormat = serde_json::from_str(&json_str).unwrap();

        //
        // Parse strings to enums or provide default value if they weren't specified
        //
        let shadow_method = if let Some(shadow_method_string) = &json_data.shadow_method {
            //TODO: This relies on input json and code matching perfectly, ideally we would search schema type for aliases
            //println!("find MeshAdvShadowMethodEnum {:?}", shadow_method_string);
            MeshAdvShadowMethodEnum::from_symbol_name(shadow_method_string.as_str()).unwrap()
        } else {
            MeshAdvShadowMethodEnum::None
        };

        let blend_method = if let Some(blend_method_string) = &json_data.blend_method {
            //TODO: This relies on input json and code matching perfectly, ideally we would search schema type for alias
            //println!("find MeshAdvBlendMethodEnum {:?}", blend_method_string);
            MeshAdvBlendMethodEnum::from_symbol_name(blend_method_string.as_str()).unwrap()
        } else {
            MeshAdvBlendMethodEnum::Opaque
        };

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object =
                MeshAdvMaterialAssetRecord::new_single_object(schema_set).unwrap();
            let mut default_asset_data_container =
                DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            let x = MeshAdvMaterialAssetRecord::default();
            x.base_color_factor()
                .set_vec4(
                    &mut default_asset_data_container,
                    json_data.base_color_factor,
                )
                .unwrap();
            x.emissive_factor()
                .set_vec3(&mut default_asset_data_container, json_data.emissive_factor)
                .unwrap();
            x.metallic_factor()
                .set(&mut default_asset_data_container, json_data.metallic_factor)
                .unwrap();
            x.roughness_factor()
                .set(
                    &mut default_asset_data_container,
                    json_data.roughness_factor,
                )
                .unwrap();
            x.normal_texture_scale()
                .set(
                    &mut default_asset_data_container,
                    json_data.normal_texture_scale,
                )
                .unwrap();

            fn try_find_file_reference(
                importable_objects: &HashMap<Option<String>, ImportableObject>,
                data_container: &mut DataContainerMut,
                ref_field: ObjectRefField,
                path_as_string: &Option<PathBuf>,
            ) {
                if let Some(path_as_string) = path_as_string {
                    if let Some(referenced_object_id) = importable_objects
                        .get(&None)
                        .unwrap()
                        .referenced_paths
                        .get(path_as_string)
                    {
                        ref_field
                            .set(data_container, *referenced_object_id)
                            .unwrap();
                    }
                }
            }

            try_find_file_reference(
                &importable_objects,
                &mut default_asset_data_container,
                x.color_texture(),
                &json_data.color_texture,
            );
            try_find_file_reference(
                &importable_objects,
                &mut default_asset_data_container,
                x.metallic_roughness_texture(),
                &json_data.metallic_roughness_texture,
            );
            try_find_file_reference(
                &importable_objects,
                &mut default_asset_data_container,
                x.normal_texture(),
                &json_data.normal_texture,
            );
            try_find_file_reference(
                &importable_objects,
                &mut default_asset_data_container,
                x.emissive_texture(),
                &json_data.emissive_texture,
            );

            x.shadow_method()
                .set(&mut default_asset_data_container, shadow_method)
                .unwrap();
            x.blend_method()
                .set(&mut default_asset_data_container, blend_method)
                .unwrap();
            x.alpha_threshold()
                .set(
                    &mut default_asset_data_container,
                    json_data.alpha_threshold.unwrap_or(0.5),
                )
                .unwrap();
            x.backface_culling()
                .set(
                    &mut default_asset_data_container,
                    json_data.backface_culling.unwrap_or(true),
                )
                .unwrap();
            //TODO: Does this incorrectly write older enum string names when code is older than schema file?
            x.color_texture_has_alpha_channel()
                .set(
                    &mut default_asset_data_container,
                    json_data.color_texture_has_alpha_channel,
                )
                .unwrap();
            default_asset_object
        };

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: None,
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}

pub struct BlenderMaterialAssetPlugin;

impl AssetPlugin for BlenderMaterialAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<BlenderMaterialImporter>(schema_linker);
    }
}
