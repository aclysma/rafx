use glam::{Mat4, Quat, Vec3};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn as_mat4(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn look_at(
        &self,
        look_at: Vec3,
        up: Vec3,
    ) -> Mat4 {
        Mat4::look_at_lh(
            self.translation,
            look_at,
            Mat4::from_rotation_translation(self.rotation, self.translation).transform_vector3(up),
        )
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}
