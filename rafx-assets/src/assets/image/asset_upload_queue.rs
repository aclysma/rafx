use super::ImageAsset;
use super::ImageAssetData;
use crate::assets::image::ImageAssetDataFormat;
use crate::assets::load_queue_hydrate::LoadRequest;
use crate::assets::upload_asset_op::{UploadAssetOp, UploadAssetOpResult};
use crate::{ImageAssetDataPayload, ImageAssetDataPayloadSubresources};
#[cfg(feature = "basis-universal")]
use basis_universal::{TranscodeParameters, TranscoderTextureFormat};
use crossbeam_channel::{Receiver, Sender};
use rafx_api::{RafxDeviceContext, RafxFormat, RafxResult, RafxTexture};
use rafx_framework::upload::UploadQueueContext;
use rafx_framework::upload::{
    GpuImageData, GpuImageDataColorSpace, GpuImageDataLayer, GpuImageDataMipLevel,
};

pub type ImageAssetUploadOpResult = UploadAssetOpResult<RafxTexture, ImageAsset>;

//pub fn new_rgba8_from_image(
//    buf: &[u8],
//    format: ImageFormat,
//    color_space: GpuImageDataColorSpace,
//) -> GpuImageData {
//    let image_data = image::load_from_memory_with_format(buf, format).unwrap();
//    let dimensions = image_data.dimensions();
//    let image_data = image_data.to_rgba8().into_raw();
//
//    GpuImageData::new_simple_image_from_bytes(
//        dimensions.0,
//        dimensions.1,
//        color_space.rgba8(),
//        image_data,
//    )
//}

pub fn new_gpu_image_data_from_image_asset_data_subresources(
    width: u32,
    height: u32,
    format: RafxFormat,
    subresources: ImageAssetDataPayloadSubresources,
) -> GpuImageData {
    #[cfg(debug_assertions)]
    {
        debug_assert_eq!(width, subresources.layers[0].mip_levels[0].width);
        debug_assert_eq!(height, subresources.layers[0].mip_levels[0].height);
        for i in 1..subresources.layers.len() {
            let layers = &subresources.layers;
            let layer_0 = &layers[0];
            debug_assert_eq!(layer_0.mip_levels.len(), layers[i].mip_levels.len());
            for j in 1..layer_0.mip_levels.len() {
                debug_assert_eq!(layer_0.mip_levels[j].width, layers[i].mip_levels[j].width);
                debug_assert_eq!(layer_0.mip_levels[j].height, layers[i].mip_levels[j].height);
                debug_assert_eq!(layer_0.mip_levels[j].width, layers[i].mip_levels[j].width);
            }
        }
    }

    let layers = subresources
        .layers
        .into_iter()
        .map(|layer| {
            let mip_levels = layer
                .mip_levels
                .into_iter()
                .map(|mip_level| GpuImageDataMipLevel {
                    width: mip_level.width,
                    height: mip_level.height,
                    data: mip_level.bytes,
                })
                .collect();

            GpuImageDataLayer { mip_levels }
        })
        .collect();

    GpuImageData {
        width,
        height,
        format,
        layers,
    }
}

pub struct ImageAssetUploadQueue {
    pub upload_queue_context: UploadQueueContext,

    pub image_upload_result_tx: Sender<ImageAssetUploadOpResult>,
    pub image_upload_result_rx: Receiver<ImageAssetUploadOpResult>,

    //DX12TODO: Temporary, disable mips on dx12 because it requries a compute shader and our
    // barriers aren't properly set up for that yet
    pub allow_generate_mips: bool,

    pub upload_texture_alignment: u32,
    pub upload_texture_row_alignment: u32,

    pub astc4x4_supported: bool,
    pub bc7_supported: bool,
}

impl ImageAssetUploadQueue {
    pub fn new(
        upload_queue_context: UploadQueueContext,
        device_context: &RafxDeviceContext,
    ) -> RafxResult<Self> {
        let (image_upload_result_tx, image_upload_result_rx) = crossbeam_channel::unbounded();
        let device_info = device_context.device_info();

        Ok(ImageAssetUploadQueue {
            upload_queue_context,
            image_upload_result_rx,
            image_upload_result_tx,
            allow_generate_mips: !device_context.is_dx12(),
            astc4x4_supported: false,
            bc7_supported: true,
            upload_texture_alignment: device_info.upload_texture_alignment,
            upload_texture_row_alignment: device_info.upload_texture_row_alignment,
        })
    }

    fn get_rafx_format(format: ImageAssetDataFormat) -> RafxFormat {
        match format {
            ImageAssetDataFormat::RGBA32_Linear => GpuImageDataColorSpace::Linear.rgba8(),
            ImageAssetDataFormat::RGBA32_Srgb => GpuImageDataColorSpace::Srgb.rgba8(),
            ImageAssetDataFormat::BC1_UNorm_Linear => RafxFormat::BC1_RGBA_UNORM_BLOCK,
            ImageAssetDataFormat::BC1_UNorm_Srgb => RafxFormat::BC1_RGBA_SRGB_BLOCK,
            ImageAssetDataFormat::BC2_UNorm_Linear => RafxFormat::BC2_UNORM_BLOCK,
            ImageAssetDataFormat::BC2_UNorm_Srgb => RafxFormat::BC2_SRGB_BLOCK,
            ImageAssetDataFormat::BC3_UNorm_Linear => RafxFormat::BC3_UNORM_BLOCK,
            ImageAssetDataFormat::BC3_UNorm_Srgb => RafxFormat::BC3_SRGB_BLOCK,
            ImageAssetDataFormat::BC4_UNorm => RafxFormat::BC4_UNORM_BLOCK,
            ImageAssetDataFormat::BC4_SNorm => RafxFormat::BC4_SNORM_BLOCK,
            ImageAssetDataFormat::BC5_UNorm => RafxFormat::BC5_UNORM_BLOCK,
            ImageAssetDataFormat::BC5_SNorm => RafxFormat::BC5_SNORM_BLOCK,
            ImageAssetDataFormat::BC6H_UFloat => RafxFormat::BC6H_UFLOAT_BLOCK,
            ImageAssetDataFormat::BC6H_SFloat => RafxFormat::BC6H_SFLOAT_BLOCK,
            ImageAssetDataFormat::BC7_Unorm_Linear => RafxFormat::BC7_UNORM_BLOCK,
            ImageAssetDataFormat::BC7_Unorm_Srgb => RafxFormat::BC7_SRGB_BLOCK,

            // We choose format of basis at runtime depending on what our hardware supports
            ImageAssetDataFormat::Basis_Linear => unimplemented!(),
            ImageAssetDataFormat::Basis_Srgb => unimplemented!(),
        }
    }

    pub fn upload_image(
        &self,
        request: LoadRequest<ImageAssetData, ImageAsset>,
    ) -> RafxResult<()> {
        let generate_mips = self.allow_generate_mips && request.asset.generate_mips_at_runtime;

        let t0 = rafx_base::Instant::now();
        let image_data = match request.asset.data {
            ImageAssetDataPayload::Subresources(subresources) => {
                profiling::scope!("prepare upload image");

                let rafx_format = Self::get_rafx_format(request.asset.format);
                new_gpu_image_data_from_image_asset_data_subresources(
                    request.asset.width,
                    request.asset.height,
                    rafx_format,
                    subresources,
                )
            }
            ImageAssetDataPayload::SingleBuffer(_single_buffer) => {
                profiling::scope!("prepare upload image");
                match request.asset.format {
                    #[cfg(not(feature = "basis-universal"))]
                    ImageAssetDataFormat::Basis_Linear | ImageAssetDataFormat::Basis_Srgb => {
                        unimplemented!("Not built with basis-universal feature");
                    }
                    #[cfg(feature = "basis-universal")]
                    ImageAssetDataFormat::Basis_Linear | ImageAssetDataFormat::Basis_Srgb => {
                        let data = _single_buffer.buffer;
                        let mut transcoder = basis_universal::Transcoder::new();
                        transcoder.prepare_transcoding(&data).unwrap();

                        let color_space = match request.asset.format {
                            ImageAssetDataFormat::Basis_Linear => GpuImageDataColorSpace::Linear,
                            ImageAssetDataFormat::Basis_Srgb => GpuImageDataColorSpace::Srgb,
                            _ => unreachable!(),
                        };

                        let (rafx_format, transcode_format) = if generate_mips {
                            // We can't do runtime mip generation with compresed formats, fall back to uncompressed data
                            (color_space.rgba8(), TranscoderTextureFormat::RGBA32)
                        } else if self.astc4x4_supported {
                            (
                                color_space.astc4x4(),
                                TranscoderTextureFormat::ASTC_4x4_RGBA,
                            )
                        } else if self.bc7_supported {
                            (color_space.bc7(), TranscoderTextureFormat::BC7_RGBA)
                        } else {
                            (color_space.rgba8(), TranscoderTextureFormat::RGBA32)
                        };

                        let layer_count = transcoder.image_count(&data);
                        if layer_count == 0 {
                            Err("BasisCompressed image asset has no images")?;
                        }

                        let level_count = transcoder.image_level_count(&data, 0);
                        if level_count == 0 {
                            Err("BasisCompressed image asset has image with no mip levels")?;
                        }

                        if level_count > 1 && generate_mips {
                            Err("BasisCompressed image asset configured to generate mips at runtime but has more than one mip layer stored")?;
                        }

                        log::trace!(
                            "Decompressing basis format: {:?} transcode format: {:?} layers: {} levels {}",
                            rafx_format,
                            transcode_format,
                            layer_count,
                            level_count
                        );

                        let mut layers = Vec::with_capacity(layer_count as usize);
                        for layer_index in 0..layer_count {
                            let image_level_count =
                                transcoder.image_level_count(&data, layer_index);
                            if image_level_count != level_count {
                                Err(format!("Two images in a BasisCompressed image asset has different mip level counts ({} and {})", level_count, image_level_count))?;
                            }

                            let mut levels = Vec::with_capacity(level_count as usize);
                            for level_index in 0..level_count {
                                let level_description = transcoder
                                    .image_level_description(&data, layer_index, level_index)
                                    .unwrap();

                                log::trace!(
                                    "transcoding layer {} level {} size: {}x{}",
                                    layer_index,
                                    level_index,
                                    level_description.original_width,
                                    level_description.original_height
                                );

                                let level_data = transcoder
                                    .transcode_image_level(
                                        &data,
                                        transcode_format,
                                        TranscodeParameters {
                                            image_index: layer_index,
                                            level_index,
                                            ..Default::default()
                                        },
                                    )
                                    .unwrap();

                                levels.push(GpuImageDataMipLevel {
                                    width: level_description.original_width,
                                    height: level_description.original_height,
                                    data: level_data,
                                });
                            }

                            layers.push(GpuImageDataLayer::new(levels));
                        }

                        GpuImageData::new(layers, rafx_format)
                    }
                    _ => unimplemented!(),
                }
            }
        };
        let t1 = rafx_base::Instant::now();

        #[cfg(debug_assertions)]
        image_data.verify_state();

        log::debug!(
            "GpuImageData layer count: {} format {:?} total bytes {} prepared in {}ms",
            image_data.layers.len(),
            image_data.format,
            image_data.total_size(
                self.upload_texture_alignment,
                self.upload_texture_row_alignment
            ),
            (t1 - t0).as_secs_f64() * 1000.0
        );
        let op = Box::new(UploadAssetOp::new(
            request.load_op,
            request.load_handle,
            request.result_tx,
            self.image_upload_result_tx.clone(),
        ));
        self.upload_queue_context.upload_new_image(
            op,
            image_data,
            request.asset.resource_type,
            generate_mips,
        )
    }
}
