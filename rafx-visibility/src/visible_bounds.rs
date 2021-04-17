use crate::geometry::{AxisAlignedBoundingBox, BoundingSphere, OrientedBoundingBox};
use crate::PolygonSoup;
use glam::Vec3;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct VisibleBounds {
    pub aabb: AxisAlignedBoundingBox,
    pub obb: OrientedBoundingBox,
    pub bounding_sphere: BoundingSphere,
    pub hash: u64,
}

impl VisibleBounds {
    pub fn from(mesh_data: PolygonSoup) -> Self {
        let hash = mesh_data.calculate_hash();
        VisibleBounds::new(hash, mesh_data)
    }

    pub(crate) fn new(
        hash: u64,
        mesh_data: PolygonSoup,
    ) -> Self {
        let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for vertex in mesh_data.vertex_positions.iter() {
            min = Vec3::min(min, *vertex);
            max = Vec3::max(max, *vertex);
        }

        min = Vec3::min(min, Vec3::splat(-0.005));
        max = Vec3::max(max, Vec3::splat(0.005));

        let sphere_center = Vec3::new(
            min.x + (max.x - min.x) / 2.,
            min.y + (max.y - min.y) / 2.,
            min.z + (max.z - min.z) / 2.,
        );

        let mut max_distance_squared = f32::MIN;
        for vertex in mesh_data.vertex_positions.iter() {
            let distance_squared = sphere_center.distance_squared(*vertex);
            max_distance_squared = f32::max(max_distance_squared, distance_squared);
            if distance_squared > max_distance_squared {
                max_distance_squared = distance_squared;
            }
        }

        let sphere_radius = f32::sqrt(max_distance_squared);

        let bounding_sphere = BoundingSphere::new(sphere_center, sphere_radius);
        let aabb = AxisAlignedBoundingBox { min, max };

        VisibleBounds {
            hash,
            aabb,
            bounding_sphere,
            obb: Default::default(), // TODO(dvd): Calculate an OBB.
        }
    }
}
