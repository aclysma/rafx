use crate::assets::gltf::MeshAsset;
use crate::features::mesh::MeshRenderNodeHandle;
use crate::features::sprite::SpriteRenderNodeHandle;
use distill::loader::handle::Handle;
use glam::f32::Vec3;
use rafx::assets::ImageAsset;
use rafx::visibility::DynamicAabbVisibilityNodeHandle;

#[derive(Clone)]
pub struct MeshComponent {
    pub render_node: MeshRenderNodeHandle,
    pub visibility_node: DynamicAabbVisibilityNodeHandle,
    pub mesh: Option<Handle<MeshAsset>>,
}

#[derive(Copy, Clone)]
pub struct PositionComponent {
    pub position: Vec3,
}

#[derive(Clone)]
pub struct PointLightComponent {
    pub color: glam::Vec4,
    pub range: f32,
    pub intensity: f32,
}

#[derive(Clone)]
pub struct DirectionalLightComponent {
    pub direction: glam::Vec3,
    pub color: glam::Vec4,
    pub intensity: f32,
}

#[derive(Clone)]
pub struct SpotLightComponent {
    pub direction: glam::Vec3,
    pub color: glam::Vec4,
    pub spotlight_half_angle: f32,
    pub range: f32,
    pub intensity: f32,
}

#[derive(Clone)]
pub struct SpriteComponent {
    pub render_node: SpriteRenderNodeHandle,
    pub visibility_node: DynamicAabbVisibilityNodeHandle,
    pub alpha: f32,
    pub image: Handle<ImageAsset>,
    //pub texture_material: ResourceArc<>
}
