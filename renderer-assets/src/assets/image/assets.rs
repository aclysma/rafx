use serde::{Deserialize, Serialize};
use type_uuid::*;
use serde::export::Formatter;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ColorSpace {
    Srgb,
    Linear,
}

impl Into<crate::image_utils::ColorSpace> for ColorSpace {
    fn into(self) -> crate::image_utils::ColorSpace {
        match self {
            ColorSpace::Srgb => crate::image_utils::ColorSpace::Srgb,
            ColorSpace::Linear => crate::image_utils::ColorSpace::Linear,
        }
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "e6166902-8716-401b-9d2e-8b01701c5626"]
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub color_space: ColorSpace,

    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl std::fmt::Debug for ImageAsset {
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
