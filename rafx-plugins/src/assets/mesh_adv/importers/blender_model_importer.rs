use crate::schema::MeshAdvModelAssetRecord;
use hydrate_data::{ImportableName, Record};
use hydrate_pipeline::{
    AssetPlugin, AssetPluginSetupContext, ImportContext, Importer, PipelineResult, ScanContext,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug)]
struct ModelLodJsonFormat {
    pub mesh: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelJsonFormat {
    pub lods: Vec<ModelLodJsonFormat>,
}

#[derive(TypeUuid, Default)]
#[uuid = "a97c46e9-1deb-4ca2-9f70-b4ce97a74cf2"]
pub struct BlenderModelImporter;

impl Importer for BlenderModelImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_model"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let json_format: ModelJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Model Import error: {:?}", x))?;

        let importable = context.add_default_importable::<MeshAdvModelAssetRecord>()?;
        for lod in &json_format.lods {
            importable.add_path_reference(&lod.mesh)?;
        }

        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let json_format: ModelJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Model Import error: {:?}", x))?;

        //
        // Create the default asset
        //
        let default_asset = MeshAdvModelAssetRecord::new_builder(context.schema_set);

        let entry = default_asset.lods().add_entry()?;
        let lod_entry = default_asset.lods().entry(entry);

        for lod in &json_format.lods {
            let mesh_object_id = context.asset_id_for_referenced_file_path(
                ImportableName::default(),
                &lod.mesh.as_path().into(),
            )?;
            lod_entry.mesh().set(mesh_object_id)?;
        }

        //
        // Return the created objects
        //
        context.add_default_importable(default_asset.into_inner()?, None);
        Ok(())
    }
}

pub struct BlenderModelAssetPlugin;

impl AssetPlugin for BlenderModelAssetPlugin {
    fn setup(context: AssetPluginSetupContext) {
        context
            .importer_registry
            .register_handler::<BlenderModelImporter>();
    }
}
