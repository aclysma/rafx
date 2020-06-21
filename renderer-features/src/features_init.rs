use legion::prelude::*;
use renderer_base::{RenderRegistryBuilder, RenderRegistry};
use crate::features::sprite::SpriteRenderFeature;
use crate::features::mesh::MeshRenderFeature;
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use crate::renderpass::debug_renderpass::DebugDraw3DResource;

pub fn init_renderer_features(
    resources: &mut Resources,
) {
    //
    // Register features/phases
    //
    let render_registry = RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<MeshRenderFeature>()
        .register_render_phase::<DrawOpaqueRenderPhase>()
        .register_render_phase::<DrawTransparentRenderPhase>()
        .build();
    resources.insert(render_registry);
    resources.insert(DebugDraw3DResource::new());
}

pub fn destroy_renderer_features(
    resources: &mut Resources,
) {
    resources.remove::<RenderRegistry>();
    resources.remove::<DebugDraw3DResource>();
}


