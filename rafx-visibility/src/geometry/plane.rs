use glam::{Vec3, Vec4};

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vec4,
}

impl Plane {
    pub fn new(
        normal: Vec3,
        point: Vec3,
    ) -> Self {
        let normal = normal.normalize();
        let d = -normal.dot(point);

        Plane {
            normal: normal.extend(d),
        }
    }

    pub fn get_normal(&self) -> Vec3 {
        self.normal.truncate()
    }

    pub fn dot(
        &self,
        vec: Vec3,
    ) -> f32 {
        self.normal.x * vec.x + self.normal.y * vec.y + self.normal.z * vec.z
    }

    pub fn distance(
        &self,
        p: Vec3,
    ) -> f32 {
        return self.normal.w + self.normal.truncate().dot(p);
    }
}
