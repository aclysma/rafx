use crate::schema::{
    GpuCompressedImageAssetOwned, GpuCompressedImageImportedDataOwned, GpuImageAssetDataFormatEnum,
};
#[cfg(feature = "basis-universal")]
use basis_universal::BasisTextureType;

use hydrate_data::RecordOwned;
use hydrate_pipeline::{ImportContext, Importer, PipelineResult, ScanContext};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "b40fd7a0-adae-48c1-972d-650ae3c08f5f"]
pub struct GpuCompressedImageImporterBasis;

impl Importer for GpuCompressedImageImporterBasis {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["basis"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        context.add_default_importable::<GpuCompressedImageAssetOwned>()?;
        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        let bytes = std::fs::read(context.path).unwrap();

        let transcoder = basis_universal::Transcoder::new();
        let texture_type = transcoder.basis_texture_type(&bytes);
        let is_cube_texture = match texture_type {
            BasisTextureType::TextureType2D => false,
            BasisTextureType::TextureType2DArray => false,
            BasisTextureType::TextureTypeCubemapArray => true,
            BasisTextureType::TextureTypeVideoFrames => false,
            BasisTextureType::TextureTypeVolume => false,
        };

        let image_count = transcoder.image_count(&bytes);
        let level_count_0 = transcoder.image_level_count(&bytes, 0);
        for i in 1..image_count {
            let level_count_i = transcoder.image_level_count(&bytes, i);
            if level_count_0 != level_count_i {
                Err::<(), String>(format!(
                    "Basis image has images with different mip level counts {} and {}",
                    level_count_0, level_count_i
                ))
                .unwrap();
            }

            for j in 1..level_count_0 {
                let level_info_0 = transcoder.image_level_description(&bytes, 0, j).unwrap();
                let level_info_i = transcoder.image_level_description(&bytes, i, j).unwrap();

                if level_info_0.original_width != level_info_i.original_width
                    || level_info_0.original_height != level_info_i.original_height
                {
                    Err::<(), String>(format!(
                        "Basis image has images with different mip level counts {}x{} and {}x{}",
                        level_info_0.original_width,
                        level_info_0.original_height,
                        level_info_i.original_width,
                        level_info_i.original_height
                    ))
                    .unwrap();
                }
            }
        }

        let level_info = transcoder.image_level_description(&bytes, 0, 0).unwrap();

        //
        // Create import data
        //
        let import_data = GpuCompressedImageImportedDataOwned::new_builder(context.schema_set);
        import_data
            .height()
            .set(level_info.original_height)
            .unwrap();
        import_data.width().set(level_info.original_width).unwrap();
        import_data
            .format()
            .set(GpuImageAssetDataFormatEnum::Basis_Srgb)
            .unwrap();
        import_data.is_cube_texture().set(is_cube_texture).unwrap();
        import_data
            .data_single_buffer()
            .set(Arc::new(bytes))
            .unwrap();

        //
        // Create the default asset
        //
        let default_asset = GpuCompressedImageAssetOwned::new_builder(context.schema_set);

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
    }
}
