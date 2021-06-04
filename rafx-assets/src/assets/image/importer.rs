use crate::assets::image::{ImageAssetColorSpace, ImageAssetData};
use crate::ImageAssetDataFormat;
#[cfg(feature = "basis-universal")]
use basis_universal::BasisTextureType;
use distill::importer::{Error, ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use image::GenericImageView;
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "23f90369-6916-4548-81d0-a76e0b162df2"]
pub struct ImageImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ae5ddc5-6805-4cf5-aa14-d44c6e0b8251"]
pub struct ImageImporter(pub image::ImageFormat);
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

        use image::EncodableLayout;

        let decoded_image = image::load_from_memory_with_format(&bytes, self.0)
            .map_err(|e| Error::Boxed(Box::new(e)))?;
        let (width, height) = decoded_image.dimensions();
        let (format, mip_generation) = ImageAssetData::default_format_and_mip_generation();
        let asset_data = ImageAssetData::from_raw_rgba32(
            width,
            height,
            ImageAssetColorSpace::Srgb,
            format,
            mip_generation,
            RafxResourceType::TEXTURE,
            decoded_image.into_rgba8().as_bytes(),
        )
        .unwrap();

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

#[cfg(feature = "basis-universal")]
#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "66ee2e3c-0c11-4cf3-a5f0-f8f3cdaa368c"]
pub struct BasisImageImporterState(Option<AssetUuid>);

#[cfg(feature = "basis-universal")]
#[derive(TypeUuid)]
#[uuid = "6da05c9f-2592-4bd4-a815-2438e05b89a4"]
pub struct BasisImageImporter;

#[cfg(feature = "basis-universal")]
impl Importer for BasisImageImporter {
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

    type State = BasisImageImporterState;

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
        *state = BasisImageImporterState(Some(id));
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let transcoder = basis_universal::Transcoder::new();
        let level_info = transcoder.image_level_description(&bytes, 0, 0).unwrap();
        let texture_type = transcoder.basis_texture_type(&bytes);
        let resource_type = match texture_type {
            BasisTextureType::TextureType2D => RafxResourceType::TEXTURE,
            BasisTextureType::TextureType2DArray => RafxResourceType::TEXTURE,
            BasisTextureType::TextureTypeCubemapArray => RafxResourceType::TEXTURE_CUBE,
            BasisTextureType::TextureTypeVideoFrames => RafxResourceType::TEXTURE,
            BasisTextureType::TextureTypeVolume => RafxResourceType::TEXTURE,
        };

        let asset_data = ImageAssetData {
            width: level_info.original_width,
            height: level_info.original_height,
            color_space: ImageAssetColorSpace::Srgb,
            format: ImageAssetDataFormat::BasisCompressed,
            generate_mips_at_runtime: false,
            resource_type,
            data: bytes,
        };

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
