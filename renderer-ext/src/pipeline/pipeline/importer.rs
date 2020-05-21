use atelier_assets::core::AssetUuid;
use atelier_assets::core::AssetRef;
use atelier_assets::importer::{ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::{Read, Cursor};
use std::convert::TryInto;
//use crate::pipeline::sprite::SpriteAsset;
use atelier_assets::importer::Error as ImportError;
use crate::pipeline::pipeline::{PipelineAsset2, MaterialAsset2, MaterialInstanceAsset2};















#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "25c8b7df-e3a4-4436-b41c-ce32eed76e18"]
struct PipelineImporterState2(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "3906ac10-8782-446d-aee4-e94611c6d61e"]
struct PipelineImporter2;
impl Importer for PipelineImporter2 {
    fn version_static() -> u32
        where
            Self: Sized,
    {
        1
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = PipelineImporterState2;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut Read,
        options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = PipelineImporterState2(Some(id));

        let pipeline_asset = ron::de::from_reader::<_, PipelineAsset2>(source)?;
        println!("IMPORTED PIPELINE2:\n{:#?}", pipeline_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(pipeline_asset),
            }],
        })
    }
}

inventory::submit!(SourceFileImporter {
    extension: "pipeline",
    instantiator: || Box::new(PipelineImporter2 {}),
});
























#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "5cfac411-55a1-49dc-b07e-1ac486f9fe98"]
struct MaterialImporterState2(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "eb9a20b7-3957-46fd-b832-2e7e99852bb0"]
struct MaterialImporter2;
impl Importer for MaterialImporter2 {
    fn version_static() -> u32
        where
            Self: Sized,
    {
        1
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MaterialImporterState2;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut Read,
        options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = MaterialImporterState2(Some(id));

        let material_asset = ron::de::from_reader::<_, MaterialAsset2>(source)?;
        println!("IMPORTED MATERIAL2:\n{:#?}", material_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(material_asset),
            }],
        })
    }
}

inventory::submit!(SourceFileImporter {
    extension: "material",
    instantiator: || Box::new(MaterialImporter2 {}),
});







#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "d40e33f3-ba7d-4218-8266-a18d7c65b06e"]
struct MaterialInstanceImporterState2(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ce02143-a5c4-4433-b843-07cdccf013b0"]
struct MaterialInstanceImporter2;
impl Importer for MaterialInstanceImporter2 {
    fn version_static() -> u32
        where
            Self: Sized,
    {
        2
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MaterialInstanceImporterState2;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut Read,
        options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = MaterialInstanceImporterState2(Some(id));

        let material_asset = ron::de::from_reader::<_, MaterialInstanceAsset2>(source)?;
        println!("IMPORTED MATERIALINSTANCE2:\n{:#?}", material_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(material_asset),
            }],
        })
    }
}

inventory::submit!(SourceFileImporter {
    extension: "materialinstance",
    instantiator: || Box::new(MaterialInstanceImporter2 {}),
});
