use crate::schema::{
    GpuCompressedImageAssetAccessor, GpuCompressedImageImportedDataAccessor,
    GpuImageAssetDataFormatEnum,
};
#[cfg(feature = "basis-universal")]
use basis_universal::BasisTextureType;
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerRefMut, RecordAccessor, SchemaSet};
use hydrate_pipeline::{
    ImportContext, ImportableAsset, ImportedImportable, ImporterRegistry, ScanContext,
    ScannedImportable,
};
use std::path::Path;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "b40fd7a0-adae-48c1-972d-650ae3c08f5f"]
pub struct GpuCompressedImageImporterBasis;

impl hydrate_pipeline::Importer for GpuCompressedImageImporterBasis {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["basis"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        let asset_type = context
            .schema_set
            .find_named_type(GpuCompressedImageAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
        }]
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> HashMap<Option<String>, ImportedImportable> {
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
        let import_data = {
            let mut import_object =
                GpuCompressedImageImportedDataAccessor::new_single_object(context.schema_set)
                    .unwrap();
            let mut import_data_container =
                DataContainerRefMut::from_single_object(&mut import_object, context.schema_set);
            let x = GpuCompressedImageImportedDataAccessor::default();

            x.height()
                .set(&mut import_data_container, level_info.original_height)
                .unwrap();
            x.width()
                .set(&mut import_data_container, level_info.original_width)
                .unwrap();
            x.format()
                .set(
                    &mut import_data_container,
                    GpuImageAssetDataFormatEnum::Basis_Srgb,
                )
                .unwrap();
            x.is_cube_texture()
                .set(&mut import_data_container, is_cube_texture)
                .unwrap();
            x.data_single_buffer()
                .set(&mut import_data_container, bytes)
                .unwrap();

            import_object
        };

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object =
                GpuCompressedImageAssetAccessor::new_single_object(context.schema_set).unwrap();

            // no fields to set

            default_asset_object
        };

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data),
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}
