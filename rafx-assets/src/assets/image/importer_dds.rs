use crate::assets::image::{ImageAssetDataLayer, ImageAssetDataMipLevel};
use crate::schema::{
    GpuCompressedImageAssetRecord, GpuCompressedImageImportedDataRecord,
    GpuImageAssetDataFormatEnum,
};
use crate::ImageAssetDataFormat;
use ddsfile::DxgiFormat;
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerMut, Record, SchemaSet};
use hydrate_model::{ImportableAsset, ImportedImportable, ImporterRegistry, ScannedImportable};
use std::path::Path;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "a66a5767-0a03-4c3e-ac06-ce02c1a0a561"]
pub struct GpuCompressedImageImporterDds;

impl hydrate_model::Importer for GpuCompressedImageImporterDds {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["dds"]
    }

    fn scan_file(
        &self,
        _path: &Path,
        schema_set: &SchemaSet,
        _importer_registry: &ImporterRegistry,
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
        importable_assets: &HashMap<Option<String>, ImportableAsset>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        let dds_bytes = std::fs::read(path).unwrap();
        let dds = ddsfile::Dds::read(&mut &dds_bytes[..]).unwrap();

        let format = if let Some(dxgi_format) = dds.get_dxgi_format() {
            match dxgi_format {
                //DxgiFormat::BC1_Typeless => {}
                DxgiFormat::BC1_UNorm => ImageAssetDataFormat::BC1_UNorm_Linear,
                DxgiFormat::BC1_UNorm_sRGB => ImageAssetDataFormat::BC1_UNorm_Srgb,
                //DxgiFormat::BC2_Typeless => {}
                DxgiFormat::BC2_UNorm => ImageAssetDataFormat::BC2_UNorm_Linear,
                DxgiFormat::BC2_UNorm_sRGB => ImageAssetDataFormat::BC2_UNorm_Srgb,
                //DxgiFormat::BC3_Typeless => {}
                DxgiFormat::BC3_UNorm => ImageAssetDataFormat::BC3_UNorm_Linear,
                DxgiFormat::BC3_UNorm_sRGB => ImageAssetDataFormat::BC3_UNorm_Srgb,
                //DxgiFormat::BC4_Typeless => {}
                DxgiFormat::BC4_UNorm => ImageAssetDataFormat::BC4_UNorm,
                DxgiFormat::BC4_SNorm => ImageAssetDataFormat::BC4_SNorm,
                //DxgiFormat::BC5_Typeless => {}
                DxgiFormat::BC5_UNorm => ImageAssetDataFormat::BC5_UNorm,
                DxgiFormat::BC5_SNorm => ImageAssetDataFormat::BC5_SNorm,
                //DxgiFormat::BC6H_Typeless => {}
                DxgiFormat::BC6H_UF16 => ImageAssetDataFormat::BC6H_UFloat,
                DxgiFormat::BC6H_SF16 => ImageAssetDataFormat::BC6H_SFloat,
                //DxgiFormat::BC7_Typeless => {}
                DxgiFormat::BC7_UNorm => ImageAssetDataFormat::BC7_Unorm_Linear,
                DxgiFormat::BC7_UNorm_sRGB => ImageAssetDataFormat::BC7_Unorm_Srgb,
                _ => unimplemented!(),
            }
        } else {
            unimplemented!();
        };

        let width = dds.get_width();
        let height = dds.get_height();
        let array_layer_count = dds.get_num_array_layers();
        let mip_level_count = dds.get_num_mipmap_levels();

        if dds.get_depth() != 1 {
            unimplemented!("DDS importer only supports image depth = 1");
        }

        log::trace!(
            "w: {} h: {} layers: {} mips: {} format: {:?} dxgi_format: {:?} d3d_format: {:?}",
            width,
            height,
            array_layer_count,
            mip_level_count,
            format,
            dds.get_dxgi_format(),
            dds.get_d3d_format()
        );
        //println!("Import DDS texture: {:?}", dds);

        let mut layers_asset_data = Vec::with_capacity(array_layer_count as usize);
        for layer_index in 0..array_layer_count {
            let layer = dds.get_data(0).unwrap();

            let mut current_mipmap_size_bytes = dds.get_main_texture_size().unwrap() as usize;
            let min_mipmap_size_bytes = dds.get_min_mipmap_size_in_bytes() as usize;
            let mut offset_bytes = 0_usize;

            let mut mip_width = width;
            let mut mip_height = height;

            let mut mip_levels_asset_data = Vec::with_capacity(mip_level_count as usize);
            for mip_index in 0..mip_level_count {
                let mip_data: Vec<u8> = layer
                    [offset_bytes..(offset_bytes + current_mipmap_size_bytes)]
                    .iter()
                    .copied()
                    .collect();
                log::trace!(
                    "Gathered mip data {} {} {}",
                    layer_index,
                    mip_index,
                    mip_data.len()
                );

                mip_levels_asset_data.push(ImageAssetDataMipLevel {
                    width: mip_width,
                    height: mip_height,
                    bytes: mip_data,
                });

                offset_bytes += current_mipmap_size_bytes;
                current_mipmap_size_bytes /= 4;
                if current_mipmap_size_bytes < min_mipmap_size_bytes {
                    current_mipmap_size_bytes = min_mipmap_size_bytes;
                }

                mip_width /= 2;
                mip_height /= 2;
            }

            layers_asset_data.push(ImageAssetDataLayer {
                mip_levels: mip_levels_asset_data,
            });
        }

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                GpuCompressedImageImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::from_single_object(&mut import_object, schema_set);
            let x = GpuCompressedImageImportedDataRecord::default();

            x.height().set(&mut import_data_container, height).unwrap();
            x.width().set(&mut import_data_container, width).unwrap();
            x.format()
                .set(
                    &mut import_data_container,
                    match format {
                        ImageAssetDataFormat::RGBA32_Linear => {
                            GpuImageAssetDataFormatEnum::RGBA32_Linear
                        }
                        ImageAssetDataFormat::RGBA32_Srgb => {
                            GpuImageAssetDataFormatEnum::RGBA32_Srgb
                        }
                        ImageAssetDataFormat::Basis_Linear => {
                            GpuImageAssetDataFormatEnum::Basis_Linear
                        }
                        ImageAssetDataFormat::Basis_Srgb => GpuImageAssetDataFormatEnum::Basis_Srgb,
                        ImageAssetDataFormat::BC1_UNorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC1_UNorm_Linear
                        }
                        ImageAssetDataFormat::BC1_UNorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb
                        }
                        ImageAssetDataFormat::BC2_UNorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC2_UNorm_Linear
                        }
                        ImageAssetDataFormat::BC2_UNorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb
                        }
                        ImageAssetDataFormat::BC3_UNorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC3_UNorm_Linear
                        }
                        ImageAssetDataFormat::BC3_UNorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb
                        }
                        ImageAssetDataFormat::BC4_UNorm => GpuImageAssetDataFormatEnum::BC4_UNorm,
                        ImageAssetDataFormat::BC4_SNorm => GpuImageAssetDataFormatEnum::BC4_SNorm,
                        ImageAssetDataFormat::BC5_UNorm => GpuImageAssetDataFormatEnum::BC5_UNorm,
                        ImageAssetDataFormat::BC5_SNorm => GpuImageAssetDataFormatEnum::BC5_SNorm,
                        ImageAssetDataFormat::BC6H_UFloat => {
                            GpuImageAssetDataFormatEnum::BC6H_UFloat
                        }
                        ImageAssetDataFormat::BC6H_SFloat => {
                            GpuImageAssetDataFormatEnum::BC6H_SFloat
                        }
                        ImageAssetDataFormat::BC7_Unorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC7_Unorm_Linear
                        }
                        ImageAssetDataFormat::BC7_Unorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb
                        }
                    },
                )
                .unwrap();

            for layer in layers_asset_data {
                let layer_entry = x
                    .data_layers()
                    .add_entry(&mut import_data_container)
                    .unwrap();
                let layer_record = x.data_layers().entry(layer_entry);

                for mip_level in layer.mip_levels {
                    let mip_level_entry = layer_record
                        .mip_levels()
                        .add_entry(&mut import_data_container)
                        .unwrap();

                    let mip_record = layer_record.mip_levels().entry(mip_level_entry);
                    mip_record
                        .width()
                        .set(&mut import_data_container, mip_level.width)
                        .unwrap();
                    mip_record
                        .height()
                        .set(&mut import_data_container, mip_level.height)
                        .unwrap();
                    mip_record
                        .bytes()
                        .set(&mut import_data_container, mip_level.bytes)
                        .unwrap();
                }
            }

            import_object
        };

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object =
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
