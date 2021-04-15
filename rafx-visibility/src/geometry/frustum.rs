use crate::geometry::plane::Plane;
use crate::geometry::BoundingSphere;
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct Frustum {
    pub planes: Vec<Plane>,
    is_invalid: bool,
}

impl Frustum {
    pub fn new(num_planes: usize) -> Self {
        Frustum {
            planes: Vec::with_capacity(num_planes),
            is_invalid: true,
        }
    }

    pub fn update(&mut self) {
        self.is_invalid = false;
    }

    pub fn invalidate(&mut self) {
        self.is_invalid = true;
    }

    pub fn is_invalid(&self) -> bool {
        self.is_invalid
    }

    pub fn contains_point(
        &self,
        point: Vec3,
    ) -> bool {
        assert_ne!(self.is_invalid, true);

        for plane in &self.planes {
            if plane.distance(point) < 0. {
                return false;
            }
        }

        return true;
    }

    #[inline(never)]
    pub fn contains_sphere_slow(
        &self,
        sphere: &BoundingSphere,
    ) -> bool {
        assert_ne!(self.is_invalid, true);

        let negative_radius = -sphere.radius;

        for plane in &self.planes {
            if plane.distance(sphere.position) <= negative_radius {
                return false;
            }
        }

        return true;
    }

    #[inline(never)]
    pub fn contains_sphere_fast(
        &self,
        sphere: &BoundingSphere,
    ) -> bool {
        assert_ne!(self.is_invalid, true);
        assert_eq!(self.planes.len(), 6);

        let radius = sphere.radius;
        let spx = sphere.position.x;
        let spy = sphere.position.y;
        let spz = sphere.position.z;

        let p1 = self.planes[0].normal;
        let p2 = self.planes[1].normal;
        let p3 = self.planes[2].normal;
        let p4 = self.planes[3].normal;
        let p5 = self.planes[4].normal;
        let p6 = self.planes[5].normal;

        let mut bitmask = 0;
        bitmask |= ((p1.w + p1.x * spx + p1.y * spy + p1.z * spz + radius <= 0.) as i32) << 0;
        bitmask |= ((p2.w + p2.x * spx + p2.y * spy + p2.z * spz + radius <= 0.) as i32) << 1;
        bitmask |= ((p3.w + p3.x * spx + p3.y * spy + p3.z * spz + radius <= 0.) as i32) << 2;
        bitmask |= ((p4.w + p4.x * spx + p4.y * spy + p4.z * spz + radius <= 0.) as i32) << 3;
        bitmask |= ((p5.w + p5.x * spx + p5.y * spy + p5.z * spz + radius <= 0.) as i32) << 4;
        bitmask |= ((p6.w + p6.x * spx + p6.y * spy + p6.z * spz + radius <= 0.) as i32) << 5;
        return bitmask <= 0;
    }
}
