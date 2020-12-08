use crate::assets::compute_pipeline::ComputePipelineAssetData;
use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{ImportedAsset, Importer, ImporterValue};
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "4c1d6cd6-8fa3-4835-8985-f733a5ad3af0"]
pub struct ComputePipelineImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "a4cce767-da93-455d-92a8-c8b1f8e5f273"]
pub struct ComputePipelineImporter;
impl Importer for ComputePipelineImporter {
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

    type State = ComputePipelineImporterState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ComputePipelineImporterState(Some(id));

        let compute_pipeline_asset = ron::de::from_reader::<_, ComputePipelineAssetData>(source)?;
        log::trace!("IMPORTED COMPUTE PIPELINE:\n{:#?}", compute_pipeline_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(compute_pipeline_asset),
            }],
        })
    }
}
