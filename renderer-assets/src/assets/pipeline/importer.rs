use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{ImportedAsset, Importer, ImporterValue, SourceFileImporter};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::{Read};
use crate::assets::pipeline::{PipelineAssetData, MaterialAssetData, MaterialInstanceAssetData, RenderpassAssetData};

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "25c8b7df-e3a4-4436-b41c-ce32eed76e18"]
struct PipelineImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "3906ac10-8782-446d-aee4-e94611c6d61e"]
struct PipelineImporter;
impl Importer for PipelineImporter {
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

    type State = PipelineImporterState;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut dyn Read,
        _options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = PipelineImporterState(Some(id));

        let pipeline_asset = ron::de::from_reader::<_, PipelineAssetData>(source)?;
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

inventory::submit!(SourceFileImporter {
    extension: "pipeline",
    instantiator: || Box::new(PipelineImporter {}),
});

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "d09c8061-3458-4f97-9265-6396344c271c"]
struct RenderpassImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "a188149d-bb0c-4c7d-8a43-0267a528bec6"]
struct RenderpassImporter;
impl Importer for RenderpassImporter {
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

    type State = RenderpassImporterState;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut dyn Read,
        _options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = RenderpassImporterState(Some(id));

        let renderpass_asset = ron::de::from_reader::<_, RenderpassAssetData>(source)?;
        log::trace!("IMPORTED RENDERPASS:\n{:#?}", renderpass_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(renderpass_asset),
            }],
        })
    }
}

inventory::submit!(SourceFileImporter {
    extension: "renderpass",
    instantiator: || Box::new(RenderpassImporter {}),
});

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "5cfac411-55a1-49dc-b07e-1ac486f9fe98"]
struct MaterialImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "eb9a20b7-3957-46fd-b832-2e7e99852bb0"]
struct MaterialImporter;
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
    fn import(
        &self,
        source: &mut dyn Read,
        _options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
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

inventory::submit!(SourceFileImporter {
    extension: "material",
    instantiator: || Box::new(MaterialImporter {}),
});

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "d40e33f3-ba7d-4218-8266-a18d7c65b06e"]
struct MaterialInstanceImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ce02143-a5c4-4433-b843-07cdccf013b0"]
struct MaterialInstanceImporter;
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
    fn import(
        &self,
        source: &mut dyn Read,
        _options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
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

inventory::submit!(SourceFileImporter {
    extension: "materialinstance",
    instantiator: || Box::new(MaterialInstanceImporter {}),
});
