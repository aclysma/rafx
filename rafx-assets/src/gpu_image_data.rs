use crate::ImageAssetDataPayloadSubresources;
use image::GenericImageView;
use image::ImageFormat;
use rafx_api::RafxFormat;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GpuImageDataColorSpace {
    Srgb,
    Linear,
}

impl GpuImageDataColorSpace {
    pub fn rgba8(self) -> RafxFormat {
        match self {
            GpuImageDataColorSpace::Srgb => RafxFormat::R8G8B8A8_SRGB,
            GpuImageDataColorSpace::Linear => RafxFormat::R8G8B8A8_UNORM,
        }
    }

    pub fn astc4x4(self) -> RafxFormat {
        match self {
            GpuImageDataColorSpace::Srgb => RafxFormat::ASTC_4X4_SRGB_BLOCK,
            GpuImageDataColorSpace::Linear => RafxFormat::ASTC_4X4_UNORM_BLOCK,
        }
    }

    pub fn bc7(self) -> RafxFormat {
        match self {
            GpuImageDataColorSpace::Srgb => RafxFormat::BC7_SRGB_BLOCK,
            GpuImageDataColorSpace::Linear => RafxFormat::BC7_UNORM_BLOCK,
        }
    }
}

#[derive(Debug)]
pub struct GpuImageData {
    pub width: u32,
    pub height: u32,
    pub format: RafxFormat,
    pub layers: Vec<GpuImageDataLayer>,
}

impl GpuImageData {
    pub fn new(
        layers: Vec<GpuImageDataLayer>,
        format: RafxFormat,
    ) -> Self {
        #[cfg(debug_assertions)]
        {
            for i in 1..layers.len() {
                debug_assert_eq!(layers[0].mip_levels.len(), layers[i].mip_levels.len());
                for j in 1..layers[0].mip_levels.len() {
                    debug_assert_eq!(layers[0].mip_levels[j].width, layers[i].mip_levels[j].width);
                    debug_assert_eq!(
                        layers[0].mip_levels[j].height,
                        layers[i].mip_levels[j].height
                    );
                    debug_assert_eq!(layers[0].mip_levels[j].width, layers[i].mip_levels[j].width);
                }
            }
        }

        GpuImageData {
            width: layers[0].mip_levels[0].width,
            height: layers[0].mip_levels[0].height,
            format,
            layers,
        }
    }

    pub fn new_simple_image_from_bytes(
        width: u32,
        height: u32,
        format: RafxFormat,
        data: Vec<u8>,
    ) -> Self {
        GpuImageData {
            width,
            height,
            format,
            layers: vec![GpuImageDataLayer::new_single_level(width, height, data)],
        }
    }

    pub fn new_from_image_asset_data_subresources(
        width: u32,
        height: u32,
        format: RafxFormat,
        subresources: ImageAssetDataPayloadSubresources,
    ) -> Self {
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

    pub fn total_size(
        &self,
        required_alignment: u64,
    ) -> u64 {
        let mut bytes_required = 0;
        for layer in &self.layers {
            for level in &layer.mip_levels {
                bytes_required += rafx_base::memory::round_size_up_to_alignment_u64(
                    level.data.len() as u64,
                    required_alignment as u64,
                )
            }
        }

        bytes_required
    }

    #[cfg(debug_assertions)]
    pub fn verify_state(&self) {
        let first_layer = &self.layers[0];
        let first_level = &first_layer.mip_levels[0];
        assert_eq!(first_level.width, self.width);
        assert_eq!(first_level.height, self.height);

        for layer in &self.layers {
            assert_eq!(first_layer.mip_levels.len(), layer.mip_levels.len());
            for (i, level) in layer.mip_levels.iter().enumerate() {
                assert_eq!(first_layer.mip_levels[i].width, level.width);
                assert_eq!(first_layer.mip_levels[i].height, level.height);
            }
        }
    }
}

#[derive(Debug)]
pub struct GpuImageDataLayer {
    pub mip_levels: Vec<GpuImageDataMipLevel>,
}

impl GpuImageDataLayer {
    pub fn new(mip_levels: Vec<GpuImageDataMipLevel>) -> Self {
        GpuImageDataLayer { mip_levels }
    }

    pub fn new_single_level(
        width: u32,
        height: u32,
        data: Vec<u8>,
    ) -> Self {
        GpuImageDataLayer {
            mip_levels: vec![GpuImageDataMipLevel {
                width,
                height,
                data,
            }],
        }
    }
}

pub struct GpuImageDataMipLevel {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl std::fmt::Debug for GpuImageDataMipLevel {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("GpuImageDataMipLevel")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("data_length", &self.data.len())
            .finish()
    }
}

impl GpuImageData {
    pub fn new_1x1_rgba8(
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        color_space: GpuImageDataColorSpace,
    ) -> Self {
        GpuImageData::new_simple_image_from_bytes(1, 1, color_space.rgba8(), vec![r, g, b, a])
    }

    pub fn new_rgba8_from_image(
        buf: &[u8],
        format: ImageFormat,
        color_space: GpuImageDataColorSpace,
    ) -> GpuImageData {
        let image_data = image::load_from_memory_with_format(buf, format).unwrap();
        let dimensions = image_data.dimensions();
        let image_data = image_data.to_rgba8().into_raw();

        GpuImageData::new_simple_image_from_bytes(
            dimensions.0,
            dimensions.1,
            color_space.rgba8(),
            image_data,
        )
    }

    pub fn new_1x1_d32(d: f32) -> Self {
        let bytes = d.to_bits().to_ne_bytes().to_vec();
        GpuImageData::new_simple_image_from_bytes(1, 1, RafxFormat::D32_SFLOAT, bytes)
    }
}
