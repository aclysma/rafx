use atelier_assets::core::AssetUuid;
use atelier_assets::core::AssetRef;
use atelier_assets::importer::{
    ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::{Read, Cursor};
use std::convert::TryInto;
use crate::pipeline::sprite::SpriteAsset;
use atelier_assets::importer::Error as ImportError;
use crate::pipeline::pipeline::PipelineAsset;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "3a7fe150-1627-4622-9e34-091b9c15fc26"]
struct PipelineImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "ecf05dde-045a-4201-9ec5-f14f91f14014"]
struct PipelineImporter;
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
    fn import(
        &self,
        source: &mut Read,
        options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = PipelineImporterState(Some(id));

        let pipeline_asset = ron::de::from_reader::<_, PipelineAsset>(source)?;

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
