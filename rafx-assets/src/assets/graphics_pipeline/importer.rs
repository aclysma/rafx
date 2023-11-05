use crate::assets::graphics_pipeline::{
    GraphicsPipelineShaderStage, MaterialAssetData, MaterialInstanceAssetData, MaterialRon,
    SamplerAssetData,
};
use crate::assets::shader::ShaderPackageImporterCooked;
use crate::schema::{GraphicsPipelineShaderStageRecord, MaterialAssetRecord};
use crate::MaterialPassData;
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, ImporterId, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable, SchemaLinker,
};
use rafx_framework::MaterialShaderStage;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use type_uuid::*;
use uuid::Uuid;
//
// #[derive(TypeUuid, Serialize, Deserialize, Default)]
// #[uuid = "5cfac411-55a1-49dc-b07e-1ac486f9fe98"]
// pub struct MaterialImporterState(Option<AssetUuid>);
//
// #[derive(TypeUuid)]
// #[uuid = "eb9a20b7-3957-46fd-b832-2e7e99852bb0"]
// pub struct MaterialImporter;
// impl Importer for MaterialImporter {
//     fn version_static() -> u32
//     where
//         Self: Sized,
//     {
//         2
//     }
//
//     fn version(&self) -> u32 {
//         Self::version_static()
//     }
//
//     type Options = ();
//
//     type State = MaterialImporterState;
//
//     /// Reads the given bytes and produces assets.
//     #[profiling::function]
//     fn import(
//         &self,
//         _op: &mut ImportOp,
//         source: &mut dyn Read,
//         _options: &Self::Options,
//         state: &mut Self::State,
//     ) -> distill::importer::Result<ImporterValue> {
//         let id = state
//             .0
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         *state = MaterialImporterState(Some(id));
//
//         let material_asset = ron::de::from_reader::<_, MaterialAssetData>(source)?;
//         log::trace!("IMPORTED MATERIAL:\n{:#?}", material_asset);
//
//         Ok(ImporterValue {
//             assets: vec![ImportedAsset {
//                 id,
//                 search_tags: vec![],
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(material_asset),
//             }],
//         })
//     }
// }

// #[derive(TypeUuid, Serialize, Deserialize, Default)]
// #[uuid = "d40e33f3-ba7d-4218-8266-a18d7c65b06e"]
// pub struct MaterialInstanceImporterState(Option<AssetUuid>);
//
// #[derive(TypeUuid)]
// #[uuid = "4ce02143-a5c4-4433-b843-07cdccf013b0"]
// pub struct MaterialInstanceImporter;
// impl Importer for MaterialInstanceImporter {
//     fn version_static() -> u32
//     where
//         Self: Sized,
//     {
//         6
//     }
//
//     fn version(&self) -> u32 {
//         Self::version_static()
//     }
//
//     type Options = ();
//
//     type State = MaterialInstanceImporterState;
//
//     /// Reads the given bytes and produces assets.
//     #[profiling::function]
//     fn import(
//         &self,
//         _op: &mut ImportOp,
//         source: &mut dyn Read,
//         _options: &Self::Options,
//         state: &mut Self::State,
//     ) -> distill::importer::Result<ImporterValue> {
//         let id = state
//             .0
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         *state = MaterialInstanceImporterState(Some(id));
//
//         let material_asset = ron::de::from_reader::<_, MaterialInstanceAssetData>(source)?;
//         log::trace!("IMPORTED MATERIALINSTANCE:\n{:#?}", material_asset);
//
//         Ok(ImporterValue {
//             assets: vec![ImportedAsset {
//                 id,
//                 search_tags: vec![],
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(material_asset),
//             }],
//         })
//     }
// }
