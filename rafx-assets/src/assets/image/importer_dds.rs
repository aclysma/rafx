use crate::assets::image::{ImageAssetDataLayer, ImageAssetDataMipLevel};
use crate::schema::{
    GpuCompressedImageAssetRecord, GpuCompressedImageImportedDataRecord,
    GpuImageAssetDataFormatEnum,
};
use crate::ImageAssetDataFormat;
use ddsfile::DxgiFormat;
use hydrate_data::Record;
use hydrate_pipeline::{ImportContext, Importer, PipelineResult, ScanContext};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "a66a5767-0a03-4c3e-ac06-ce02c1a0a561"]
pub struct GpuCompressedImageImporterDds;

impl Importer for GpuCompressedImageImporterDds {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["dds"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        context.add_default_importable::<GpuCompressedImageAssetRecord>()?;
        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        let dds_bytes = std::fs::read(context.path).unwrap();
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
        let import_data = GpuCompressedImageImportedDataRecord::new_builder(context.schema_set);

        import_data.height().set(height).unwrap();
        import_data.width().set(width).unwrap();
        import_data
            .format()
            .set(match format {
                ImageAssetDataFormat::RGBA32_Linear => GpuImageAssetDataFormatEnum::RGBA32_Linear,
                ImageAssetDataFormat::RGBA32_Srgb => GpuImageAssetDataFormatEnum::RGBA32_Srgb,
                ImageAssetDataFormat::Basis_Linear => GpuImageAssetDataFormatEnum::Basis_Linear,
                ImageAssetDataFormat::Basis_Srgb => GpuImageAssetDataFormatEnum::Basis_Srgb,
                ImageAssetDataFormat::BC1_UNorm_Linear => {
                    GpuImageAssetDataFormatEnum::BC1_UNorm_Linear
                }
                ImageAssetDataFormat::BC1_UNorm_Srgb => GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb,
                ImageAssetDataFormat::BC2_UNorm_Linear => {
                    GpuImageAssetDataFormatEnum::BC2_UNorm_Linear
                }
                ImageAssetDataFormat::BC2_UNorm_Srgb => GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb,
                ImageAssetDataFormat::BC3_UNorm_Linear => {
                    GpuImageAssetDataFormatEnum::BC3_UNorm_Linear
                }
                ImageAssetDataFormat::BC3_UNorm_Srgb => GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb,
                ImageAssetDataFormat::BC4_UNorm => GpuImageAssetDataFormatEnum::BC4_UNorm,
                ImageAssetDataFormat::BC4_SNorm => GpuImageAssetDataFormatEnum::BC4_SNorm,
                ImageAssetDataFormat::BC5_UNorm => GpuImageAssetDataFormatEnum::BC5_UNorm,
                ImageAssetDataFormat::BC5_SNorm => GpuImageAssetDataFormatEnum::BC5_SNorm,
                ImageAssetDataFormat::BC6H_UFloat => GpuImageAssetDataFormatEnum::BC6H_UFloat,
                ImageAssetDataFormat::BC6H_SFloat => GpuImageAssetDataFormatEnum::BC6H_SFloat,
                ImageAssetDataFormat::BC7_Unorm_Linear => {
                    GpuImageAssetDataFormatEnum::BC7_Unorm_Linear
                }
                ImageAssetDataFormat::BC7_Unorm_Srgb => GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb,
            })
            .unwrap();

        for layer in layers_asset_data {
            let layer_entry = import_data.data_layers().add_entry().unwrap();
            let layer_record = import_data.data_layers().entry(layer_entry);

            for mip_level in layer.mip_levels {
                let mip_level_entry = layer_record.mip_levels().add_entry().unwrap();

                let mip_record = layer_record.mip_levels().entry(mip_level_entry);
                mip_record.width().set(mip_level.width).unwrap();
                mip_record.height().set(mip_level.height).unwrap();
                mip_record.bytes().set(Arc::new(mip_level.bytes)).unwrap();
            }
        }

        //
        // Create the default asset
        //
        let default_asset = GpuCompressedImageAssetRecord::new_builder(context.schema_set);

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
    }
}
