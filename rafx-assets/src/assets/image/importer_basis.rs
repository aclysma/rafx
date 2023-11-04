use crate::assets::image::{
    ImageAssetData, ImageAssetDataPayload, ImageAssetDataPayloadSingleBuffer,
};
use crate::schema::{
    GpuCompressedImageAssetRecord, GpuCompressedImageImportedDataRecord,
    GpuImageAssetDataFormatEnum,
};
use crate::ImageAssetDataFormat;
#[cfg(feature = "basis-universal")]
use basis_universal::BasisTextureType;
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerMut, Record, SchemaSet};
use hydrate_model::{ImportableObject, ImportedImportable, ImporterRegistry, ScannedImportable};
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
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

#[derive(TypeUuid, Default)]
#[uuid = "b40fd7a0-adae-48c1-972d-650ae3c08f5f"]
pub struct GpuCompressedImageImporterBasis;

impl hydrate_model::Importer for GpuCompressedImageImporterBasis {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["basis"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
        importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        let asset_type = schema_set
            .find_named_type(GpuCompressedImageAssetRecord::schema_name())
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
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        let bytes = std::fs::read(path).unwrap();

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
                GpuCompressedImageImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::new_single_object(&mut import_object, schema_set);
            let x = GpuCompressedImageImportedDataRecord::default();

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
            let mut default_asset_object =
                GpuCompressedImageAssetRecord::new_single_object(schema_set).unwrap();

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
