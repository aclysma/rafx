use crate::assets::graphics_pipeline::{
    GraphicsPipelineAssetData, MaterialAssetData, MaterialInstanceAssetData, SamplerAssetData,
};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "62e662dc-cb15-444f-a7ac-eb89f52a4042"]
pub struct SamplerImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "9dfad44f-72e8-4ba6-b89a-96b017fb9cd9"]
pub struct SamplerImporter;
impl Importer for SamplerImporter {
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

    type State = SamplerImporterState;

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
        *state = SamplerImporterState(Some(id));

        let sampler_asset = ron::de::from_reader::<_, SamplerAssetData>(source)?;
        log::trace!("IMPORTED SAMPLER:\n{:#?}", sampler_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(sampler_asset),
            }],
        })
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "25c8b7df-e3a4-4436-b41c-ce32eed76e18"]
pub struct PipelineImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "3906ac10-8782-446d-aee4-e94611c6d61e"]
pub struct PipelineImporter;
impl Importer for PipelineImporter {
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

    type State = PipelineImporterState;

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
        *state = PipelineImporterState(Some(id));

        let pipeline_asset = ron::de::from_reader::<_, GraphicsPipelineAssetData>(source)?;
        log::trace!("IMPORTED PIPELINE:\n{:#?}", pipeline_asset);

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

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "5cfac411-55a1-49dc-b07e-1ac486f9fe98"]
pub struct MaterialImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "eb9a20b7-3957-46fd-b832-2e7e99852bb0"]
pub struct MaterialImporter;
impl Importer for MaterialImporter {
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

    type State = MaterialImporterState;

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
        *state = MaterialImporterState(Some(id));

        let material_asset = ron::de::from_reader::<_, MaterialAssetData>(source)?;
        log::trace!("IMPORTED MATERIAL:\n{:#?}", material_asset);

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

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "d40e33f3-ba7d-4218-8266-a18d7c65b06e"]
pub struct MaterialInstanceImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ce02143-a5c4-4433-b843-07cdccf013b0"]
pub struct MaterialInstanceImporter;
impl Importer for MaterialInstanceImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        6
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MaterialInstanceImporterState;

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
        *state = MaterialInstanceImporterState(Some(id));

        let material_asset = ron::de::from_reader::<_, MaterialInstanceAssetData>(source)?;
        log::trace!("IMPORTED MATERIALINSTANCE:\n{:#?}", material_asset);

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
