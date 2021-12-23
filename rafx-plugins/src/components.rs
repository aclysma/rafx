use glam::f32::Vec3;
use glam::Quat;
use rafx::framework::visibility::VisibilityObjectArc;
use rafx::render_features::RenderObjectHandle;
use rafx::visibility::ViewFrustumArc;

#[derive(Clone)]
pub struct SpriteComponent {
    pub render_object_handle: RenderObjectHandle,
}

#[derive(Clone)]
pub struct MeshComponent {
    pub render_object_handle: RenderObjectHandle,
}

#[derive(Clone)]
pub struct VisibilityComponent {
    pub visibility_object_handle: VisibilityObjectArc,
}

#[derive(Clone, Copy)]
pub struct TransformComponent {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for TransformComponent {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl TransformComponent {
    pub fn rotate(
        &mut self,
        rotation: Quat,
    ) {
        self.rotation *= rotation;
    }

    pub fn mul_transform(
        &self,
        transform: TransformComponent,
    ) -> Self {
        let translation = self.mul_vec3(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        TransformComponent {
            translation,
            rotation,
            scale,
        }
    }

    pub fn mul_vec3(
        &self,
        mut value: Vec3,
    ) -> Vec3 {
        value = self.rotation * value;
        value = self.scale * value;
        value += self.translation;
        value
    }
}

const LIGHT_MIN_INTENSITY_TO_RENDER: f32 = 0.2;

#[derive(Clone)]
pub struct PointLightComponent {
    pub color: glam::Vec4,
    pub range: Option<f32>,
    pub intensity: f32,
    pub view_frustums: [ViewFrustumArc; 6],
}

impl PointLightComponent {
    pub fn range_sq_from_intensity(intensity: f32) -> f32 {
        intensity / LIGHT_MIN_INTENSITY_TO_RENDER
    }

    pub fn range(&self) -> f32 {
        if let Some(range) = self.range {
            range
        } else {
            Self::range_sq_from_intensity(self.intensity).sqrt()
        }
    }

    pub fn range_sq(&self) -> f32 {
        if let Some(range) = self.range {
            range * range
        } else {
            Self::range_sq_from_intensity(self.intensity)
        }
    }
}

#[derive(Clone)]
pub struct DirectionalLightComponent {
    pub direction: glam::Vec3,
    pub color: glam::Vec4,
    pub intensity: f32,
    pub view_frustum: ViewFrustumArc,
}

#[derive(Clone)]
pub struct SpotLightComponent {
    pub direction: glam::Vec3,
    pub color: glam::Vec4,
    pub spotlight_half_angle: f32,
    pub range: Option<f32>,
    pub intensity: f32,
    pub view_frustum: ViewFrustumArc,
}

impl SpotLightComponent {
    pub fn range_sq_from_intensity(intensity: f32) -> f32 {
        intensity / LIGHT_MIN_INTENSITY_TO_RENDER
    }

    pub fn range(&self) -> f32 {
        if let Some(range) = self.range {
            range
        } else {
            Self::range_sq_from_intensity(self.intensity).sqrt()
        }
    }

    pub fn range_sq(&self) -> f32 {
        if let Some(range) = self.range {
            range * range
        } else {
            Self::range_sq_from_intensity(self.intensity)
        }
    }
}
