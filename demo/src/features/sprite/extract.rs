use crate::components::{PositionComponent, SpriteComponent};
use crate::features::sprite::plugin::SpriteStaticResources;
use crate::features::sprite::prepare::SpritePrepareJob;
use crate::features::sprite::{
    ExtractedSpriteData, SpriteRenderFeature, SpriteRenderNode, SpriteRenderNodeSet,
};
use legion::*;
use rafx::assets::AssetManagerRenderResource;
use rafx::base::slab::RawSlabKey;
use rafx::nodes::{
    ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex,
    RenderJobExtractContext, RenderView,
};
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet};

pub struct SpriteExtractJob {}

impl SpriteExtractJob {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob for SpriteExtractJob {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!("Sprite Extract");
        let mut legion_world = extract_context.extract_resources.fetch_mut::<World>();
        let world = &mut *legion_world;

        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        // Update the mesh render nodes. This could be done earlier as part of a system
        let mut sprite_render_nodes = extract_context
            .render_resources
            .fetch_mut::<SpriteRenderNodeSet>();

        let mut dynamic_visibility_node_set = extract_context
            .render_resources
            .fetch_mut::<DynamicVisibilityNodeSet>();

        let mut query = <(Read<PositionComponent>, Write<SpriteComponent>)>::query();
        for (position_component, mut sprite_component) in query.iter_mut(world) {
            if let Some(sprite_handle) = &sprite_component.render_node {
                let render_node = sprite_render_nodes.get_mut(sprite_handle).unwrap();
                render_node.image = sprite_component.image.clone();
                render_node.alpha = sprite_component.alpha;
                render_node.position = position_component.position;
            } else {
                let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
                    position: position_component.position,
                    alpha: sprite_component.alpha,
                    image: sprite_component.image.clone(),
                });

                let visibility_node =
                    dynamic_visibility_node_set.register_dynamic_aabb(DynamicAabbVisibilityNode {
                        handle: render_node.as_raw_generic_handle(),
                        // aabb bounds
                    });

                sprite_component.render_node = Some(render_node);
                sprite_component.visibility_node = Some(visibility_node);
            }
        }

        let mut extracted_frame_node_sprite_data =
            Vec::<Option<ExtractedSpriteData>>::with_capacity(
                frame_packet.frame_node_count(self.feature_index()) as usize,
            );

        for frame_node in frame_packet.frame_nodes(self.feature_index()) {
            let render_node_index = frame_node.render_node_index();
            let render_node_handle = RawSlabKey::<SpriteRenderNode>::new(render_node_index);
            let sprite_render_node = sprite_render_nodes
                .sprites
                .get_raw(render_node_handle)
                .unwrap();

            let image_asset = asset_manager.committed_asset(&sprite_render_node.image);

            let extracted_frame_node = image_asset.and_then(|image_asset| {
                Some(ExtractedSpriteData {
                    position: sprite_render_node.position,
                    texture_size: glam::Vec2::new(50.0, 50.0),
                    scale: 1.0,
                    rotation: 0.0,
                    alpha: sprite_render_node.alpha,
                    image_view: image_asset.image_view.clone(),
                })
            });

            extracted_frame_node_sprite_data.push(extracted_frame_node);
        }

        let static_resources = extract_context
            .render_resources
            .fetch::<SpriteStaticResources>();

        let sprite_material = asset_manager
            .committed_asset(&static_resources.sprite_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        let prepare_impl = SpritePrepareJob::new(extracted_frame_node_sprite_data, sprite_material);

        Box::new(prepare_impl)
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}
