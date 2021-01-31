use crate::assets::image::{ImageAssetColorSpace, ImageAssetData};
use distill::importer::{Error, ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use image2::Image;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "23f90369-6916-4548-81d0-a76e0b162df2"]
pub struct ImageImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ae5ddc5-6805-4cf5-aa14-d44c6e0b8251"]
pub struct ImageImporter;
impl Importer for ImageImporter {
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

    type State = ImageImporterState;

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
        *state = ImageImporterState(Some(id));
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let decoded_image = image2::io::decode::<_, _, image2::Rgba>(&bytes)
            .map_err(|e| Error::Boxed(Box::new(e)))?;

        let image_asset = ImageAssetData {
            width: decoded_image.width() as u32,
            height: decoded_image.height() as u32,
            color_space: ImageAssetColorSpace::Srgb,
            data: decoded_image.data().to_vec(),
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(image_asset),
            }],
        })
    }
}
