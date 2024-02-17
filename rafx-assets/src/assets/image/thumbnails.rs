use crate::schema::{
    GpuCompressedImageAssetRecord, GpuCompressedImageImportedDataRecord,
    GpuImageAssetDataFormatEnum, GpuImageAssetRecord, GpuImageImportedDataRecord,
};
#[cfg(feature = "basis-universal")]
use basis_universal::TranscodeParameters;

use hydrate_data::Record;
use hydrate_pipeline::{
    PipelineError, PipelineResult, ThumbnailImage, ThumbnailProvider,
    ThumbnailProviderGatherContext, ThumbnailProviderRenderContext,
};
use image::RgbaImage;
use std::sync::Arc;

#[derive(Default)]
pub struct GpuImageThumbnailProvider {}

impl ThumbnailProvider for GpuImageThumbnailProvider {
    type GatheredDataT = ();

    fn asset_type(&self) -> &'static str {
        GpuImageAssetRecord::schema_name()
    }

    fn version(&self) -> u32 {
        1
    }

    fn gather(
        &self,
        context: ThumbnailProviderGatherContext,
    ) -> Self::GatheredDataT {
        context.add_import_data_dependency(context.asset_id);
    }

    fn render<'a>(
        &'a self,
        context: &'a ThumbnailProviderRenderContext<'a>,
        _gathered_data: Self::GatheredDataT,
    ) -> PipelineResult<ThumbnailImage> {
        let import_data = context.imported_data::<GpuImageImportedDataRecord>(context.asset_id)?;
        let width = import_data.width().get()?;
        let height = import_data.height().get()?;
        let image_bytes = import_data.image_bytes().get()?.clone();

        let image = ::image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
            width,
            height,
            (*image_bytes).clone(),
        )
        .unwrap();

        resize_image_for_thumbnail(context, &image)
    }
}

#[cfg(not(feature = "basis-universal"))]
fn decode_basis(
    _format: GpuImageAssetDataFormatEnum,
    _bytes: Arc<Vec<u8>>,
) -> PipelineResult<RgbaImage> {
    Err("Cannot decode basis-universal image, the basis-universal feature was not enabled when compiling".into())
}

#[cfg(feature = "basis-universal")]
fn decode_basis(
    _format: GpuImageAssetDataFormatEnum,
    bytes: Arc<Vec<u8>>,
) -> PipelineResult<RgbaImage> {
    let mut transcoder = basis_universal::Transcoder::new();
    transcoder.prepare_transcoding(&**bytes).unwrap();

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

#[cfg(not(feature = "ddsfile"))]
fn decode_bcn(
    _width: u32,
    _height: u32,
    _format: GpuImageAssetDataFormatEnum,
    _bytes: Arc<Vec<u8>>,
) -> PipelineResult<RgbaImage> {
    Err("Cannot decode BCn image, the ddsfile feature was not enabled when compiling".into())
}

#[cfg(feature = "ddsfile")]
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
        .map_err(|_| PipelineError::ThumbnailUnavailable)?
        .to_image(0)
        .map_err(|_| PipelineError::ThumbnailUnavailable)?)
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
        context.add_import_data_dependency(context.asset_id);
    }

    fn render<'a>(
        &'a self,
        context: &'a ThumbnailProviderRenderContext<'a>,
        _gathered_data: Self::GatheredDataT,
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

        resize_image_for_thumbnail(context, &image)
    }
}

fn resize_image_for_thumbnail(
    context: &ThumbnailProviderRenderContext,
    image: &RgbaImage,
) -> PipelineResult<ThumbnailImage> {
    let resize_ratio_x = image.width() as f32 / context.desired_thumbnail_width as f32;
    let resize_ratio_y = image.height() as f32 / context.desired_thumbnail_height as f32;

    let resize_ratio = resize_ratio_x.max(resize_ratio_y);
    let new_size_x = ((image.width() as f32 / resize_ratio).round() as u32).max(1);
    let new_size_y = ((image.height() as f32 / resize_ratio).round() as u32).max(1);

    let resized_image = ::image::imageops::resize(
        image,
        new_size_x,
        new_size_y,
        ::image::imageops::FilterType::Lanczos3,
    );

    Ok(ThumbnailImage {
        width: resized_image.width(),
        height: resized_image.height(),
        pixel_data: resized_image.into_raw(),
    })
}
