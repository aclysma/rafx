use atelier_assets::core::AssetUuid;
use atelier_assets::core::AssetRef;
use atelier_assets::importer::{ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;
use crate::pipeline::sprite::SpriteAsset;
use atelier_assets::importer::Error as ImportError;

#[derive(Debug)]
pub enum SpriteImportError {
    JsonError(serde_json::Error),
}

impl std::error::Error for SpriteImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            SpriteImportError::JsonError(ref e) => Some(e),
        }
    }
}

impl core::fmt::Display for SpriteImportError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            SpriteImportError::JsonError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<serde_json::Error> for SpriteImportError {
    fn from(result: serde_json::Error) -> Self {
        SpriteImportError::JsonError(result)
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "366c1293-df04-4185-9d6c-a79e34ea423e"]
struct SpriteImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "8df57b53-9cad-46ca-be9d-14078b443942"]
struct SpriteImporter;
impl Importer for SpriteImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        4
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = SpriteImporterState;

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
        *state = SpriteImporterState(Some(id));

        let sprite_asset: SpriteAsset = serde_json::from_reader(source).map_err(|err| {
            println!("SPRITE IMPORT ERROR: {:?}", err);
            ImportError::Boxed(Box::new(SpriteImportError::JsonError(err)))
        })?;

        let load_deps = sprite_asset
            .images
            .iter()
            .map(|asset_uuid| AssetRef::Uuid(*asset_uuid))
            .collect();
        println!("SPRITE IMPORT {:?}", sprite_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps,
                build_pipeline: None,
                asset_data: Box::new(sprite_asset),
            }],
        })
    }
}

inventory::submit!(SourceFileImporter {
    extension: "sprite",
    instantiator: || Box::new(SpriteImporter {}),
});
