use super::SpriteRenderFeature;
use glam::Vec3;
use hydrate_base::handle::Handle;
use rafx::assets::ImageAsset;
use rafx::render_features::RenderObjectSet;

#[derive(Clone)]
pub struct SpriteRenderObject {
    pub tint: Vec3,
    pub alpha: f32,
    pub image: Handle<ImageAsset>,
}

pub type SpriteRenderObjectSet = RenderObjectSet<SpriteRenderFeature, SpriteRenderObject>;
