use image::GenericImageView;
use image::ImageFormat;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecodedImageColorSpace {
    Srgb,
    Linear,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecodedImageMips {
    // No mips - this should only be set if mip_level_count == 1
    None,

    // Mips should be generated from the loaded data at runtime
    Runtime(u32),

    // Mips, if any, are already computed and included in the loaded data
    Precomputed(u32),
}

impl DecodedImageMips {
    pub fn mip_level_count(&self) -> u32 {
        match self {
            DecodedImageMips::None => 1,
            DecodedImageMips::Runtime(mip_count) => *mip_count,
            DecodedImageMips::Precomputed(mip_count) => *mip_count,
        }
    }
}

#[derive(Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub color_space: DecodedImageColorSpace,
    pub mips: DecodedImageMips,
    pub data: Vec<u8>,
}

impl DecodedImage {
    pub fn new_1x1(
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        color_space: DecodedImageColorSpace,
    ) -> DecodedImage {
        DecodedImage {
            width: 1,
            height: 1,
            color_space,
            mips: DecodedImageMips::None,
            data: vec![r, g, b, a],
        }
    }

    pub fn new_from_buffer(
        buf: &[u8],
        format: ImageFormat,
    ) -> DecodedImage {
        let image_data = image::load_from_memory_with_format(buf, format).unwrap();
        let dimensions = image_data.dimensions();
        let image_data = image_data.to_rgba8().into_raw();
        let decoded_image_mip_info =
            DecodedImage::default_mip_settings_for_image_size(dimensions.0, dimensions.1);

        DecodedImage {
            width: dimensions.0,
            height: dimensions.1,
            mips: decoded_image_mip_info,
            data: image_data,
            color_space: DecodedImageColorSpace::Srgb,
        }
    }

    // Provides default settings for an image that's loaded without metadata specifying mip settings
    pub fn default_mip_settings_for_image_size(
        width: u32,
        height: u32,
    ) -> DecodedImageMips {
        let max_dimension = std::cmp::max(width, height);
        let mip_level_count = (max_dimension as f32).log2().floor() as u32 + 1;
        DecodedImageMips::Runtime(mip_level_count)
    }
}
