use super::ModelBasicAssetData;
use crate::assets::mesh_adv::{MeshBasicAsset, ModelBasicAssetDataLod};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug)]
struct ModelLodJsonFormat {
    pub mesh: Handle<MeshBasicAsset>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelJsonFormat {
    pub lods: Vec<ModelLodJsonFormat>,
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "6fc6dae5-4995-46f8-b808-aa0149f6067b"]
pub struct MeshBasicBlenderModelImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "9c3fbad2-0ab9-46ba-8e8c-e179cded2321"]
pub struct MeshBasicBlenderModelImporter;
impl Importer for MeshBasicBlenderModelImporter {
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

    type State = MeshBasicBlenderModelImporterState;

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
        *state = MeshBasicBlenderModelImporterState(Some(id));

        let json_format: ModelJsonFormat = serde_json::from_reader(source)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let mut lods = Vec::with_capacity(json_format.lods.len());
        for lod in json_format.lods {
            lods.push(ModelBasicAssetDataLod {
                mesh: lod.mesh.clone(),
            });
        }

        let asset_data = ModelBasicAssetData { lods };

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
