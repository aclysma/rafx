use atelier_assets::core::AssetUuid;
use atelier_assets::core::AssetRef;
use atelier_assets::importer::{ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::{Read, Cursor};
use std::convert::TryInto;
use crate::pipeline::sprite::SpriteAsset;
use atelier_assets::importer::Error as ImportError;
use crate::pipeline::shader::ShaderAsset;
use crate::pipeline_description as dsc;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "867bc278-67b5-469c-aeea-1c05da722918"]
struct ShaderImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "90fdad4b-cec1-4f59-b679-97895711b6e1"]
struct ShaderImporter;
impl Importer for ShaderImporter {
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

    type State = ShaderImporterState;

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
        *state = ShaderImporterState(Some(id));

        // Raw compiled shader
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let code = renderer_shell_vulkan::util::read_spv(&mut Cursor::new(bytes.as_mut_slice()))?;
        let shader_asset = ShaderAsset {
            shader: dsc::ShaderModule {
                code
            }
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader_asset),
            }],
        })
    }
}

inventory::submit!(SourceFileImporter {
    extension: "spv",
    instantiator: || Box::new(ShaderImporter {}),
});
