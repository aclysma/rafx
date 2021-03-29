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
        let legion_world = extract_context.extract_resources.fetch::<World>();
        let world = &*legion_world;

        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        // Update the mesh render nodes. This could be done earlier as part of a system
        let mut sprite_render_nodes = extract_context
            .extract_resources
            .fetch_mut::<SpriteRenderNodeSet>();
        sprite_render_nodes.update();

        let mut query = <(Read<PositionComponent>, Read<SpriteComponent>)>::query();
        for (position_component, sprite_component) in query.iter(world) {
            let render_node = sprite_render_nodes
                .get_mut(&sprite_component.render_node)
                .unwrap();
            render_node.image = sprite_component.image.clone();
            render_node.alpha = sprite_component.alpha;
            render_node.position = position_component.position;
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
                let texture_extents = image_asset.image.get_raw().image.texture_def().extents;

                Some(ExtractedSpriteData {
                    position: sprite_render_node.position,
                    texture_size: glam::Vec2::new(
                        texture_extents.width as f32,
                        texture_extents.height as f32,
                    ),
                    scale: sprite_render_node.scale,
                    rotation: sprite_render_node.rotation,
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
