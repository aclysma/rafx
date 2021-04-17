use glam::Vec3;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisAlignedBoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}
