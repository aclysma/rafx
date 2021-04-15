use glam::Vec3;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundingSphere {
    pub radius: f32,
    pub position: Vec3,
}

impl BoundingSphere {
    pub fn new(
        position: Vec3,
        radius: f32,
    ) -> Self {
        BoundingSphere { radius, position }
    }
}
