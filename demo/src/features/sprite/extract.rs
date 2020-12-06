use crate::components::{PositionComponent, SpriteComponent};
use crate::features::sprite::prepare::SpritePrepareJob;
use crate::features::sprite::{
    ExtractedSpriteData, SpriteRenderFeature, SpriteRenderNode, SpriteRenderNodeSet,
};
use crate::render_contexts::{
    RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext,
};
use atelier_assets::loader::handle::Handle;
use legion::*;
use rafx::assets::MaterialAsset;
use rafx::base::slab::RawSlabKey;
use rafx::nodes::{
    ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex, RenderView,
};

pub struct SpriteExtractJob {
    sprite_material: Handle<MaterialAsset>,
}

impl SpriteExtractJob {
    pub fn new(sprite_material: Handle<MaterialAsset>) -> Self {
        SpriteExtractJob { sprite_material }
    }
}

impl ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>
    for SpriteExtractJob
{
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        _views: &[&RenderView],
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        profiling::scope!("Sprite Extract");

        // Update the mesh render nodes. This could be done earlier as part of a system
        let mut sprite_render_nodes = extract_context
            .resources
            .get_mut::<SpriteRenderNodeSet>()
            .unwrap();

        let mut query = <(Read<PositionComponent>, Read<SpriteComponent>)>::query();
        for (position_component, sprite_component) in query.iter(extract_context.world) {
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

            let image_asset = extract_context
                .asset_manager
                .get_image_asset(&sprite_render_node.image);

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

        // For now just grab pass 0
        let sprite_material = extract_context
            .asset_manager
            .get_material_pass_by_index(&self.sprite_material, 0)
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
