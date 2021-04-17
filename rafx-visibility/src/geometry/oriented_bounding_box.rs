use glam::{Quat, Vec3};
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrientedBoundingBox {
    pub min: Vec3,
    pub max: Vec3,
    pub rotation: Quat,
}
