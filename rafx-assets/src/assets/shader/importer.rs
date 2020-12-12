use crate::assets::shader::ShaderAssetData;
use crate::CookedShader;
use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{ImportedAsset, Importer, ImporterValue};
use rafx_resources::vk_description as dsc;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use type_uuid::*;

// There may be a better way to do this type coercing
// fn coerce_result_str<T>(result: Result<T, &str>) -> atelier_assets::importer::Result<T> {
//     let ok = result.map_err(|x| -> Box<dyn std::error::Error + Send> { Box::<dyn std::error::Error + Send + Sync>::from(x) })?;
//     Ok(ok)
// }

fn coerce_result_string<T>(result: Result<T, String>) -> atelier_assets::importer::Result<T> {
    let ok = result.map_err(|x| -> Box<dyn std::error::Error + Send> {
        Box::<dyn std::error::Error + Send + Sync>::from(x)
    })?;
    Ok(ok)
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "867bc278-67b5-469c-aeea-1c05da722918"]
pub struct ShaderImporterSpvState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "90fdad4b-cec1-4f59-b679-97895711b6e1"]
pub struct ShaderImporterSpv;
impl Importer for ShaderImporterSpv {
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

    type State = ShaderImporterSpvState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let asset_id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ShaderImporterSpvState(Some(asset_id));

        // Raw compiled shader
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let code = rafx_api_vulkan::util::read_spv(&mut Cursor::new(bytes.as_mut_slice()))?;

        log::trace!(
            "Import shader asset {:?} with {} bytes of code",
            asset_id,
            code.len() * std::mem::size_of::<u32>()
        );

        // The hash is used in some places identify the shader
        let code_hash = dsc::ShaderModuleCodeHash::hash_shader_code(&code);

        let shader_asset = ShaderAssetData {
            shader: dsc::ShaderModule { code, code_hash },
            reflection_data: None,
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: asset_id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader_asset),
            }],
        })
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "d4fb07ce-76e6-497e-ac31-bcaeb43528aa"]
pub struct ShaderImporterCookedState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "cab0cf4c-16ff-4dbd-aae7-8705246d85d6"]
pub struct ShaderImporterCooked;
impl Importer for ShaderImporterCooked {
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

    type State = ShaderImporterCookedState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let asset_id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ShaderImporterCookedState(Some(asset_id));

        // Raw compiled shader
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let cooked_shader: CookedShader = coerce_result_string(
            bincode::deserialize::<CookedShader>(&bytes)
                .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x)),
        )?;

        let code = rafx_api_vulkan::util::read_spv(&mut Cursor::new(&cooked_shader.spv))?;

        log::trace!(
            "Import shader asset {:?} with {} bytes of code",
            asset_id,
            code.len() * std::mem::size_of::<u32>()
        );

        // The hash is used in some places identify the shader
        let code_hash = dsc::ShaderModuleCodeHash::hash_shader_code(&code);

        let shader_asset = ShaderAssetData {
            shader: dsc::ShaderModule { code, code_hash },
            reflection_data: Some(cooked_shader.entry_points),
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: asset_id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader_asset),
            }],
        })
    }
}
