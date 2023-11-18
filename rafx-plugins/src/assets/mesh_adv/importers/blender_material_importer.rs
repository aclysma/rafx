use crate::schema::{
    MeshAdvBlendMethodEnum, MeshAdvMaterialAssetAccessor, MeshAdvShadowMethodEnum,
};
use hydrate_base::handle::Handle;
use hydrate_base::hashing::HashMap;
use hydrate_data::{AssetRefFieldAccessor, DataContainerRefMut, Enum, RecordAccessor, SchemaSet};
use hydrate_pipeline::{
    AssetPlugin, BuilderRegistryBuilder, ImportContext, ImportableAsset, ImportedImportable,
    ImporterRegistry, ImporterRegistryBuilder, JobProcessorRegistryBuilder, ReferencedSourceFile,
    ScanContext, ScannedImportable, SchemaLinker,
};
use rafx::assets::ImageAsset;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use type_uuid::*;

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

#[derive(TypeUuid, Default)]
#[uuid = "e76bab79-654a-476f-93b1-88cd5fee7d1f"]
pub struct BlenderMaterialImporter;

impl hydrate_pipeline::Importer for BlenderMaterialImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_material"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        let asset_type = context
            .schema_set
            .find_named_type(MeshAdvMaterialAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let json_str = std::fs::read_to_string(context.path).unwrap();
        let json_data: HydrateMaterialJsonFileFormat = serde_json::from_str(&json_str).unwrap();

        let mut file_references: Vec<ReferencedSourceFile> = Default::default();

        fn try_add_image_file_reference(
            file_references: &mut Vec<ReferencedSourceFile>,
            path_as_string: &Option<PathBuf>,
            importer_registry: &ImporterRegistry,
        ) {
            if let Some(path_as_string) = path_as_string {
                let extension = path_as_string
                    .extension()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                // We could be using different image importers scanning for different formats, so search for the best importer
                let importers = importer_registry.importers_for_file_extension(&extension);
                let importer_id = importers[0];

                file_references.push(ReferencedSourceFile {
                    importer_id,
                    path: path_as_string.clone(),
                })
            }
        }

        //TODO: We assume a particular importer but we should probably just say what kind of imported
        // data or asset we would use?
        try_add_image_file_reference(
            &mut file_references,
            &json_data.color_texture,
            context.importer_registry,
        );
        try_add_image_file_reference(
            &mut file_references,
            &json_data.metallic_roughness_texture,
            context.importer_registry,
        );
        try_add_image_file_reference(
            &mut file_references,
            &json_data.normal_texture,
            context.importer_registry,
        );
        try_add_image_file_reference(
            &mut file_references,
            &json_data.emissive_texture,
            context.importer_registry,
        );

        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references,
        }]
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let json_str = std::fs::read_to_string(context.path).unwrap();
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
                MeshAdvMaterialAssetAccessor::new_single_object(context.schema_set).unwrap();
            let mut default_asset_data_container = DataContainerRefMut::from_single_object(
                &mut default_asset_object,
                context.schema_set,
            );
            let x = MeshAdvMaterialAssetAccessor::default();
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
                importable_assets: &HashMap<Option<String>, ImportableAsset>,
                data_container: &mut DataContainerRefMut,
                ref_field: AssetRefFieldAccessor,
                path_as_string: &Option<PathBuf>,
            ) {
                if let Some(path_as_string) = path_as_string {
                    if let Some(referenced_object_id) = importable_assets
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
                &context.importable_assets,
                &mut default_asset_data_container,
                x.color_texture(),
                &json_data.color_texture,
            );
            try_find_file_reference(
                &context.importable_assets,
                &mut default_asset_data_container,
                x.metallic_roughness_texture(),
                &json_data.metallic_roughness_texture,
            );
            try_find_file_reference(
                &context.importable_assets,
                &mut default_asset_data_container,
                x.normal_texture(),
                &json_data.normal_texture,
            );
            try_find_file_reference(
                &context.importable_assets,
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
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        _builder_registry: &mut BuilderRegistryBuilder,
        _job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<BlenderMaterialImporter>();
    }
}
