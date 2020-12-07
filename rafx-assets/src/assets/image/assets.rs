use rafx_resources::{ImageViewResource, ResourceArc};
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use type_uuid::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ImageAssetColorSpace {
    Srgb,
    Linear,
}

impl Into<crate::DecodedImageColorSpace> for ImageAssetColorSpace {
    fn into(self) -> crate::DecodedImageColorSpace {
        match self {
            ImageAssetColorSpace::Srgb => crate::DecodedImageColorSpace::Srgb,
            ImageAssetColorSpace::Linear => crate::DecodedImageColorSpace::Linear,
        }
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "e6166902-8716-401b-9d2e-8b01701c5626"]
pub struct ImageAssetData {
    pub width: u32,
    pub height: u32,
    pub color_space: ImageAssetColorSpace,

    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl std::fmt::Debug for ImageAssetData {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("width", &self.width)
            .field("width", &self.height)
            .field("byte_count", &self.data.len())
            .finish()
    }
}

#[derive(TypeUuid, Clone)]
#[uuid = "7a67b850-17f9-4877-8a6e-293a1589bbd8"]
pub struct ImageAsset {
    pub image_view: ResourceArc<ImageViewResource>,
}
