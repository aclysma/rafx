use crate::assets::image::{ImageAssetDataLayer, ImageAssetDataMipLevel};
use crate::schema::{
    GpuCompressedImageAssetRecord, GpuCompressedImageImportedDataRecord,
    GpuImageAssetDataFormatEnum, GpuImageAssetRecord, GpuImageImportedDataRecord,
};
use crate::ImageAssetDataFormat;
use basis_universal::sys::basisu_image;
use basis_universal::{TranscodeParameters, TranscoderTextureFormat};
use ddsfile::DxgiFormat;
use hydrate_data::{EnumFieldRef, Record};
use hydrate_pipeline::{
    ImportContext, Importer, PipelineError, PipelineResult, ScanContext, ThumbnailImage,
    ThumbnailProvider, ThumbnailProviderGatherContext, ThumbnailProviderRenderContext,
};
use image::{Pixel, RgbaImage};
use rafx_framework::upload::GpuImageDataColorSpace;
use std::fmt::format;
use std::sync::Arc;
use type_uuid::*;

fn decode_basis(
    format: GpuImageAssetDataFormatEnum,
    bytes: Arc<Vec<u8>>,
) -> PipelineResult<RgbaImage> {
    println!("decode_basis AAAAA");
    let mut transcoder = basis_universal::Transcoder::new();
    transcoder.prepare_transcoding(&**bytes).unwrap();
    println!("decode_basis BBBBB");

    let level_info = transcoder.image_level_info(&**bytes, 0, 0).unwrap();

    let level_data = transcoder
        .transcode_image_level(
            &**bytes,
            basis_universal::TranscoderTextureFormat::RGBA32,
            TranscodeParameters {
                image_index: 0,
                level_index: 0,
                ..Default::default()
            },
        )
        .unwrap();

    Ok(image::RgbaImage::from_raw(level_info.m_width, level_info.m_height, level_data).unwrap())
}

fn decode_bcn(
    width: u32,
    height: u32,
    format: GpuImageAssetDataFormatEnum,
    bytes: Arc<Vec<u8>>,
) -> PipelineResult<RgbaImage> {
    let image_format = match format {
        GpuImageAssetDataFormatEnum::BC1_UNorm_Linear => image_dds::ImageFormat::BC1Unorm,
        GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb => image_dds::ImageFormat::BC1Srgb,
        GpuImageAssetDataFormatEnum::BC2_UNorm_Linear => image_dds::ImageFormat::BC2Unorm,
        GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb => image_dds::ImageFormat::BC2Srgb,
        GpuImageAssetDataFormatEnum::BC3_UNorm_Linear => image_dds::ImageFormat::BC3Unorm,
        GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb => image_dds::ImageFormat::BC3Srgb,
        GpuImageAssetDataFormatEnum::BC4_UNorm => image_dds::ImageFormat::BC4Unorm,
        GpuImageAssetDataFormatEnum::BC4_SNorm => image_dds::ImageFormat::BC4Snorm,
        GpuImageAssetDataFormatEnum::BC5_UNorm => image_dds::ImageFormat::BC5Unorm,
        GpuImageAssetDataFormatEnum::BC5_SNorm => image_dds::ImageFormat::BC5Snorm,
        GpuImageAssetDataFormatEnum::BC6H_UFloat => image_dds::ImageFormat::BC6Ufloat,
        GpuImageAssetDataFormatEnum::BC6H_SFloat => image_dds::ImageFormat::BC6Sfloat,
        GpuImageAssetDataFormatEnum::BC7_Unorm_Linear => image_dds::ImageFormat::BC7Unorm,
        GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb => image_dds::ImageFormat::BC7Srgb,
        _ => Err(PipelineError::ThumbnailUnavailable)?,
    };

    let surface = image_dds::Surface::<&[u8]> {
        width,
        height,
        depth: 1,
        layers: 1,
        mipmaps: 1,
        image_format,
        data: &(**bytes),
    };

    Ok(surface
        .decode_rgba8()
        .map_err(|e| PipelineError::ThumbnailUnavailable)?
        .to_image(0)
        .map_err(|e| PipelineError::ThumbnailUnavailable)?)
}

#[derive(Default)]
pub struct GpuCompressedImageThumbnailProvider {}

impl ThumbnailProvider for GpuCompressedImageThumbnailProvider {
    type GatheredDataT = ();

    fn asset_type(&self) -> &'static str {
        GpuCompressedImageAssetRecord::schema_name()
    }

    fn version(&self) -> u32 {
        1
    }

    fn gather(
        &self,
        context: ThumbnailProviderGatherContext,
    ) -> Self::GatheredDataT {
        //println!("Gather data to make thumbnail for {}", context.asset_id);
        context.add_import_data_dependency(context.asset_id);
    }

    fn render<'a>(
        &'a self,
        context: &'a ThumbnailProviderRenderContext<'a>,
        gathered_data: Self::GatheredDataT,
    ) -> PipelineResult<ThumbnailImage> {
        let import_data =
            context.imported_data::<GpuCompressedImageImportedDataRecord>(context.asset_id)?;

        //let uncompressed = vec![0u8; (width * height * 4) as usize];

        let format = import_data.format().get()?;

        let image = match format {
            GpuImageAssetDataFormatEnum::Basis_Linear | GpuImageAssetDataFormatEnum::Basis_Srgb => {
                let bytes = import_data.data_single_buffer().get()?.clone();
                decode_basis(format, bytes)
            }
            GpuImageAssetDataFormatEnum::BC1_UNorm_Linear
            | GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb
            | GpuImageAssetDataFormatEnum::BC2_UNorm_Linear
            | GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb
            | GpuImageAssetDataFormatEnum::BC3_UNorm_Linear
            | GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb
            | GpuImageAssetDataFormatEnum::BC4_UNorm
            | GpuImageAssetDataFormatEnum::BC4_SNorm
            | GpuImageAssetDataFormatEnum::BC5_UNorm
            | GpuImageAssetDataFormatEnum::BC5_SNorm
            | GpuImageAssetDataFormatEnum::BC6H_UFloat
            | GpuImageAssetDataFormatEnum::BC6H_SFloat
            | GpuImageAssetDataFormatEnum::BC7_Unorm_Linear
            | GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb => {
                let layer_entries = import_data.data_layers().resolve_entries()?;
                if layer_entries.len() == 0 {
                    Err(PipelineError::ThumbnailUnavailable)?;
                }

                let mip_entries = import_data
                    .data_layers()
                    .entry(layer_entries[0])
                    .mip_levels()
                    .resolve_entries()?;
                if mip_entries.len() == 0 {
                    Err(PipelineError::ThumbnailUnavailable)?;
                }

                let layer0 = import_data.data_layers().entry(layer_entries[0]);
                let layer0_mip0 = layer0.mip_levels().entry(mip_entries[0]);
                let width = layer0_mip0.width().get()?;
                let height = layer0_mip0.height().get()?;
                let bytes = layer0_mip0.bytes().get()?.clone();

                decode_bcn(width, height, format, bytes)
            }
            GpuImageAssetDataFormatEnum::RGBA32_Linear => unimplemented!(),
            GpuImageAssetDataFormatEnum::RGBA32_Srgb => unimplemented!(),
        }?;

        let resized_image =
            ::image::imageops::resize(&image, 256, 256, ::image::imageops::FilterType::Lanczos3);

        // This is a very wasteful way to do this..
        let mut pixel_data = Vec::default();
        for (x, y, color) in resized_image.enumerate_pixels() {
            let (r, g, b, a) = color.channels4();
            pixel_data.push(r);
            pixel_data.push(g);
            pixel_data.push(b);
            pixel_data.push(a);
        }

        Ok(ThumbnailImage {
            width: 256,
            height: 256,
            pixel_data,
        })
    }
}
