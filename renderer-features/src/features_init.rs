use legion::prelude::*;
use renderer_nodes::{RenderRegistryBuilder, RenderRegistry};
use crate::features::sprite::SpriteRenderFeature;
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use crate::renderpass::debug_renderpass::DebugDraw3DResource;

pub fn create_default_registry_builder() -> RenderRegistryBuilder {
    RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_render_phase::<DrawOpaqueRenderPhase>()
        .register_render_phase::<DrawTransparentRenderPhase>()
}

pub fn init_renderer_features(resources: &mut Resources) {
    resources.insert(DebugDraw3DResource::new());
}

pub fn destroy_renderer_features(resources: &mut Resources) {
    resources.remove::<DebugDraw3DResource>();
}
