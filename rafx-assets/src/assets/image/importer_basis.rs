use crate::assets::image::{
    ImageAssetData, ImageAssetDataPayload, ImageAssetDataPayloadSingleBuffer,
};
use crate::ImageAssetDataFormat;
#[cfg(feature = "basis-universal")]
use basis_universal::BasisTextureType;
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

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
        let texture_type = transcoder.basis_texture_type(&bytes);
        let resource_type = match texture_type {
            BasisTextureType::TextureType2D => RafxResourceType::TEXTURE,
            BasisTextureType::TextureType2DArray => RafxResourceType::TEXTURE,
            BasisTextureType::TextureTypeCubemapArray => RafxResourceType::TEXTURE_CUBE,
            BasisTextureType::TextureTypeVideoFrames => RafxResourceType::TEXTURE,
            BasisTextureType::TextureTypeVolume => RafxResourceType::TEXTURE,
        };

        let image_count = transcoder.image_count(&bytes);
        let level_count_0 = transcoder.image_level_count(&bytes, 0);
        for i in 1..image_count {
            let level_count_i = transcoder.image_level_count(&bytes, i);
            if level_count_0 != level_count_i {
                Err(format!(
                    "Basis image has images with different mip level counts {} and {}",
                    level_count_0, level_count_i
                ))?;
            }

            for j in 1..level_count_0 {
                let level_info_0 = transcoder.image_level_description(&bytes, 0, j).unwrap();
                let level_info_i = transcoder.image_level_description(&bytes, i, j).unwrap();

                if level_info_0.original_width != level_info_i.original_width
                    || level_info_0.original_height != level_info_i.original_height
                {
                    Err(format!(
                        "Basis image has images with different mip level counts {}x{} and {}x{}",
                        level_info_0.original_width,
                        level_info_0.original_height,
                        level_info_i.original_width,
                        level_info_i.original_height
                    ))?;
                }
            }
        }

        let level_info = transcoder.image_level_description(&bytes, 0, 0).unwrap();
        let asset_data = ImageAssetData {
            width: level_info.original_width,
            height: level_info.original_height,
            format: ImageAssetDataFormat::Basis_Srgb,
            generate_mips_at_runtime: false,
            resource_type,
            data: ImageAssetDataPayload::SingleBuffer(ImageAssetDataPayloadSingleBuffer {
                buffer: bytes,
            }),
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
