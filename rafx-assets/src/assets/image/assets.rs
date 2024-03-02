use rafx_api::{RafxResourceType, RafxResult};
use rafx_framework::upload::GpuImageDataColorSpace;
use rafx_framework::{ImageResource, ImageViewResource, ResourceArc};
use serde::{Deserialize, Serialize};
use type_uuid::*;

//NOTE: This is serialized in image asset options, so may require asset schema change if modifying it
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ImageAssetColorSpaceConfig {
    Srgb,
    Linear,
}

//NOTE: This is serialized in image asset options, so may require asset schema change if modifying it
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ImageAssetMipGeneration {
    NoMips,
    Precomupted,
    Runtime,
}

impl Into<GpuImageDataColorSpace> for ImageAssetColorSpaceConfig {
    fn into(self) -> GpuImageDataColorSpace {
        match self {
            ImageAssetColorSpaceConfig::Srgb => GpuImageDataColorSpace::Srgb,
            ImageAssetColorSpaceConfig::Linear => GpuImageDataColorSpace::Linear,
        }
    }
}

//NOTE: This is serialized in image asset options, so may require asset schema change if modifying it
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ImageAssetBasisCompressionType {
    Etc1S,
    Uastc,
}

#[cfg(feature = "basis-universal")]
impl Into<basis_universal::BasisTextureFormat> for ImageAssetBasisCompressionType {
    fn into(self) -> basis_universal::BasisTextureFormat {
        match self {
            ImageAssetBasisCompressionType::Etc1S => basis_universal::BasisTextureFormat::ETC1S,
            ImageAssetBasisCompressionType::Uastc => basis_universal::BasisTextureFormat::UASTC4x4,
        }
    }
}

//NOTE: This is serialized in image asset options, so may require asset schema change if modifying it
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ImageAssetBasisCompressionSettings {
    pub compression_type: ImageAssetBasisCompressionType,
    pub quality: u32,
}

#[cfg(feature = "basis-universal")]
impl ImageAssetBasisCompressionSettings {
    pub fn default_uastc() -> Self {
        ImageAssetBasisCompressionSettings {
            compression_type: ImageAssetBasisCompressionType::Uastc,
            quality: basis_universal::UASTC_QUALITY_DEFAULT,
        }
    }

    pub fn default_etc1s() -> Self {
        ImageAssetBasisCompressionSettings {
            compression_type: ImageAssetBasisCompressionType::Etc1S,
            quality: basis_universal::ETC1S_QUALITY_DEFAULT,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub enum ImageAssetDataFormat {
    RGBA32_Linear,
    RGBA32_Srgb,
    Basis_Linear,
    Basis_Srgb,
    BC1_UNorm_Linear,
    BC1_UNorm_Srgb,
    BC2_UNorm_Linear,
    BC2_UNorm_Srgb,
    BC3_UNorm_Linear,
    BC3_UNorm_Srgb,
    BC4_UNorm,
    BC4_SNorm,
    BC5_UNorm,
    BC5_SNorm,
    BC6H_UFloat,
    BC6H_SFloat,
    BC7_Unorm_Linear,
    BC7_Unorm_Srgb,
}

//NOTE: This is serialized in image asset options, so may require asset schema change if modifying it
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ImageAssetDataFormatConfig {
    Uncompressed,
    BasisCompressed(ImageAssetBasisCompressionSettings),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageAssetDataMipLevel {
    pub width: u32,
    pub height: u32,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageAssetDataLayer {
    pub mip_levels: Vec<ImageAssetDataMipLevel>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageAssetDataPayloadSubresources {
    pub layers: Vec<ImageAssetDataLayer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageAssetDataPayloadSingleBuffer {
    #[serde(with = "serde_bytes")]
    pub buffer: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ImageAssetDataPayload {
    Subresources(ImageAssetDataPayloadSubresources),
    // Special case intended for things like basis where we don't unpack subresources until runtime
    SingleBuffer(ImageAssetDataPayloadSingleBuffer),
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "e6166902-8716-401b-9d2e-8b01701c5626"]
pub struct ImageAssetData {
    pub width: u32,
    pub height: u32,
    pub format: ImageAssetDataFormat,
    pub resource_type: RafxResourceType,
    pub generate_mips_at_runtime: bool,
    pub data: ImageAssetDataPayload,
}

impl std::fmt::Debug for ImageAssetData {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("width", &self.width)
            .field("width", &self.height)
            .field("format", &self.format)
            .finish()
    }
}

impl ImageAssetData {
    // Temporary - off by default because encoding textures is very slow
    pub fn default_format_and_mip_generation(
    ) -> (ImageAssetDataFormatConfig, ImageAssetMipGeneration) {
        let compress_textures = false;
        if compress_textures {
            #[cfg(feature = "basis-universal")]
            {
                let basis_settings = ImageAssetBasisCompressionSettings::default_uastc();
                let format_config = ImageAssetDataFormatConfig::BasisCompressed(basis_settings);
                let mipmap_generation = ImageAssetMipGeneration::Precomupted;
                (format_config, mipmap_generation)
            }

            #[cfg(not(feature = "basis-universal"))]
            {
                unimplemented!("Not built with basis-universal feature")
            }
        } else {
            let format_config = ImageAssetDataFormatConfig::Uncompressed;
            let mipmap_generation = ImageAssetMipGeneration::Runtime;
            (format_config, mipmap_generation)
        }
    }

    pub fn from_raw_rgba32(
        width: u32,
        height: u32,
        color_space: ImageAssetColorSpaceConfig,
        format_config: ImageAssetDataFormatConfig,
        mip_generation: ImageAssetMipGeneration,
        resource_type: RafxResourceType,
        raw_rgba32: &[u8],
    ) -> RafxResult<ImageAssetData> {
        match format_config {
            ImageAssetDataFormatConfig::Uncompressed => {
                let generate_mips_at_runtime = match mip_generation {
                    ImageAssetMipGeneration::NoMips => false,
                    ImageAssetMipGeneration::Precomupted => Err(
                        "Uncompressed ImageAssetDataFormatConfig cannot store precomputed mipmaps",
                    )?,
                    ImageAssetMipGeneration::Runtime => true,
                };

                let mip = ImageAssetDataMipLevel {
                    width,
                    height,
                    bytes: raw_rgba32.to_vec(),
                };

                let layer = ImageAssetDataLayer {
                    mip_levels: vec![mip],
                };

                let format = match color_space {
                    ImageAssetColorSpaceConfig::Linear => ImageAssetDataFormat::RGBA32_Linear,
                    ImageAssetColorSpaceConfig::Srgb => ImageAssetDataFormat::RGBA32_Srgb,
                };

                Ok(ImageAssetData {
                    width,
                    height,
                    format,
                    generate_mips_at_runtime,
                    resource_type,
                    data: ImageAssetDataPayload::Subresources(ImageAssetDataPayloadSubresources {
                        layers: vec![layer],
                    }),
                })
            }
            #[cfg(not(feature = "basis-universal"))]
            ImageAssetDataFormatConfig::BasisCompressed(_) => {
                unimplemented!("crate not built with basis-universal feature");
            }
            #[cfg(feature = "basis-universal")]
            ImageAssetDataFormatConfig::BasisCompressed(settings) => {
                let generate_mips_at_runtime = match mip_generation {
                    ImageAssetMipGeneration::NoMips => false,
                    ImageAssetMipGeneration::Precomupted => false,
                    ImageAssetMipGeneration::Runtime => true,
                };

                let basis_color_space = match color_space {
                    ImageAssetColorSpaceConfig::Srgb => basis_universal::ColorSpace::Srgb,
                    ImageAssetColorSpaceConfig::Linear => basis_universal::ColorSpace::Linear,
                };

                let mut compressor_params = basis_universal::CompressorParams::new();
                compressor_params.set_basis_format(settings.compression_type.into());
                compressor_params
                    .set_generate_mipmaps(mip_generation == ImageAssetMipGeneration::Precomupted);
                compressor_params.set_color_space(basis_color_space);

                match settings.compression_type {
                    ImageAssetBasisCompressionType::Etc1S => {
                        let quality_level = settings.quality.clamp(
                            basis_universal::ETC1S_QUALITY_MIN,
                            basis_universal::ETC1S_QUALITY_MAX,
                        );
                        compressor_params.set_etc1s_quality_level(quality_level)
                    }
                    ImageAssetBasisCompressionType::Uastc => {
                        let quality_level = settings.quality.clamp(
                            basis_universal::ETC1S_QUALITY_MIN,
                            basis_universal::ETC1S_QUALITY_MAX,
                        );
                        compressor_params.set_uastc_quality_level(quality_level)
                    }
                }

                let mut source_image = compressor_params.source_image_mut(0);
                source_image.init(raw_rgba32, width, height, 4);

                let mut compressor = basis_universal::Compressor::new(4);
                unsafe {
                    compressor.init(&compressor_params);
                    log::debug!("Compressing texture");
                    compressor.process().unwrap();
                    log::debug!("Compressed texture");
                }
                let compressed_basis_data = compressor.basis_file();

                let format = match color_space {
                    ImageAssetColorSpaceConfig::Linear => ImageAssetDataFormat::Basis_Linear,
                    ImageAssetColorSpaceConfig::Srgb => ImageAssetDataFormat::Basis_Srgb,
                };

                Ok(ImageAssetData {
                    width,
                    height,
                    format,
                    generate_mips_at_runtime,
                    resource_type,
                    data: ImageAssetDataPayload::SingleBuffer(ImageAssetDataPayloadSingleBuffer {
                        buffer: compressed_basis_data.to_vec(),
                    }),
                })
            }
        }
    }
}

#[derive(TypeUuid, Clone)]
#[uuid = "7a67b850-17f9-4877-8a6e-293a1589bbd8"]
pub struct ImageAsset {
    pub image: ResourceArc<ImageResource>,
    pub image_view: ResourceArc<ImageViewResource>,
}
