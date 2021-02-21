use rafx_api::{RafxResourceType, RafxResult};
use rafx_framework::{ImageResource, ImageViewResource, ResourceArc};
use serde::{Deserialize, Serialize};
use type_uuid::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ImageAssetColorSpace {
    Srgb,
    Linear,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ImageAssetMipGeneration {
    NoMips,
    Precomupted,
    Runtime,
}

impl Into<crate::GpuImageDataColorSpace> for ImageAssetColorSpace {
    fn into(self) -> crate::GpuImageDataColorSpace {
        match self {
            ImageAssetColorSpace::Srgb => crate::GpuImageDataColorSpace::Srgb,
            ImageAssetColorSpace::Linear => crate::GpuImageDataColorSpace::Linear,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ImageAssetBasisCompressionType {
    Etc1S,
    Uastc,
}

impl Into<basis_universal::BasisTextureFormat> for ImageAssetBasisCompressionType {
    fn into(self) -> basis_universal::BasisTextureFormat {
        match self {
            ImageAssetBasisCompressionType::Etc1S => basis_universal::BasisTextureFormat::ETC1S,
            ImageAssetBasisCompressionType::Uastc => basis_universal::BasisTextureFormat::UASTC4x4,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ImageAssetBasisCompressionSettings {
    compression_type: ImageAssetBasisCompressionType,
    quality: u32,
}

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
pub enum ImageAssetDataFormat {
    RawRGBA32,
    BasisCompressed,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ImageAssetDataFormatConfig {
    RawRGBA32,
    BasisCompressed(ImageAssetBasisCompressionSettings),
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "e6166902-8716-401b-9d2e-8b01701c5626"]
pub struct ImageAssetData {
    pub width: u32,
    pub height: u32,
    pub color_space: ImageAssetColorSpace,
    pub format: ImageAssetDataFormat,
    pub resource_type: RafxResourceType,
    pub generate_mips_at_runtime: bool,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl std::fmt::Debug for ImageAssetData {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("width", &self.width)
            .field("width", &self.height)
            .field("byte_count", &self.data.len())
            .field("color_space", &self.color_space)
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
            let basis_settings = ImageAssetBasisCompressionSettings::default_uastc();
            let format_config = ImageAssetDataFormatConfig::BasisCompressed(basis_settings);
            let mipmap_generation = ImageAssetMipGeneration::Precomupted;
            (format_config, mipmap_generation)
        } else {
            let format_config = ImageAssetDataFormatConfig::RawRGBA32;
            let mipmap_generation = ImageAssetMipGeneration::Runtime;
            (format_config, mipmap_generation)
        }
    }

    pub fn from_raw_rgba32(
        width: u32,
        height: u32,
        color_space: ImageAssetColorSpace,
        format_config: ImageAssetDataFormatConfig,
        mip_generation: ImageAssetMipGeneration,
        resource_type: RafxResourceType,
        raw_rgba32: &[u8],
    ) -> RafxResult<ImageAssetData> {
        match format_config {
            ImageAssetDataFormatConfig::RawRGBA32 => {
                let generate_mips_at_runtime = match mip_generation {
                    ImageAssetMipGeneration::NoMips => false,
                    ImageAssetMipGeneration::Precomupted => {
                        Err("RawRGBA32 cannot store precomputed mipmaps")?
                    }
                    ImageAssetMipGeneration::Runtime => true,
                };

                Ok(ImageAssetData {
                    width,
                    height,
                    color_space,
                    format: ImageAssetDataFormat::RawRGBA32,
                    generate_mips_at_runtime,
                    resource_type,
                    data: raw_rgba32.to_vec(),
                })
            }
            ImageAssetDataFormatConfig::BasisCompressed(settings) => {
                let generate_mips_at_runtime = match mip_generation {
                    ImageAssetMipGeneration::NoMips => false,
                    ImageAssetMipGeneration::Precomupted => false,
                    ImageAssetMipGeneration::Runtime => true,
                };

                let basis_color_space = match color_space {
                    ImageAssetColorSpace::Srgb => basis_universal::ColorSpace::Srgb,
                    ImageAssetColorSpace::Linear => basis_universal::ColorSpace::Linear,
                };

                let mut compressor_params = basis_universal::CompressorParams::new();
                compressor_params.set_basis_format(settings.compression_type.into());
                compressor_params
                    .set_generate_mipmaps(mip_generation == ImageAssetMipGeneration::Precomupted);
                compressor_params.set_color_space(basis_color_space);

                match settings.compression_type {
                    ImageAssetBasisCompressionType::Etc1S => {
                        compressor_params.set_etc1s_quality_level(settings.quality)
                    }
                    ImageAssetBasisCompressionType::Uastc => {
                        compressor_params.set_uastc_quality_level(settings.quality)
                    }
                }

                let mut source_image = compressor_params.source_image_mut(0);
                source_image.init(raw_rgba32, width, height, 4);

                let mut compressor = basis_universal::Compressor::new(4);
                unsafe {
                    compressor.init(&compressor_params);
                    println!("compressing");
                    compressor.process().unwrap();
                    println!("compressed");
                }
                let compressed_basis_data = compressor.basis_file();

                Ok(ImageAssetData {
                    width,
                    height,
                    color_space,
                    format: ImageAssetDataFormat::BasisCompressed,
                    generate_mips_at_runtime,
                    resource_type,
                    data: compressed_basis_data.to_vec(),
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
