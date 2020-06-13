use crate::features::sprite::{ExtractedSpriteData, SpriteRenderNodeSet, SpriteRenderFeature, SpriteRenderNode};
use crate::{ExtractSource, PositionComponent, SpriteComponent, CommandWriterContext};
use renderer_base::{DefaultExtractJobImpl, FramePacket, RenderView, PerViewNode, PrepareJob, DefaultPrepareJob, RenderFeatureIndex, RenderFeature, PerFrameNode};
use renderer_base::slab::RawSlabKey;
use crate::features::sprite::prepare::SpritePrepareJobImpl;
use renderer_shell_vulkan::VkDeviceContext;

pub struct SpriteExtractJobImpl {
    device_context: VkDeviceContext,
    extracted_sprite_data: Vec<ExtractedSpriteData>,
}

impl SpriteExtractJobImpl {
    pub fn new(device_context: VkDeviceContext) -> Self {
        SpriteExtractJobImpl {
            device_context,
            extracted_sprite_data: Default::default()
        }
    }
}

impl DefaultExtractJobImpl<ExtractSource, CommandWriterContext> for SpriteExtractJobImpl {
    fn extract_begin(
        &mut self,
        _source: &ExtractSource,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) {
        self.extracted_sprite_data
            .reserve(frame_packet.frame_node_count(self.feature_index()) as usize);
    }

    fn extract_frame_node(
        &mut self,
        source: &ExtractSource,
        frame_node: PerFrameNode,
        frame_node_index: u32,
    ) {
        let render_node_index = frame_node.render_node_index();
        let render_node_handle = RawSlabKey::<SpriteRenderNode>::new(render_node_index);

        let sprite_nodes = source.resources.get::<SpriteRenderNodeSet>().unwrap();
        let sprite_render_node = sprite_nodes.sprites.get(render_node_handle).unwrap();

        let position_component = source
            .world
            .get_component::<PositionComponent>(sprite_render_node.entity)
            .unwrap();
        let sprite_component = source
            .world
            .get_component::<SpriteComponent>(sprite_render_node.entity)
            .unwrap();

        self.extracted_sprite_data.push(ExtractedSpriteData {
            position: position_component.position,
            texture_size: glam::Vec2::new(100.0, 100.0),
            scale: 1.0,
            rotation: 0.0,
            texture_descriptor_index: 0,
            alpha: sprite_component.alpha,
        });
    }

    fn extract_view_node(
        &mut self,
        _source: &ExtractSource,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    ) {

    }

    fn extract_view_finalize(
        &mut self,
        _source: &ExtractSource,
        _view: &RenderView,
    ) {

    }

    fn extract_frame_finalize(
        self,
        _source: &ExtractSource,
    ) -> Box<dyn PrepareJob<CommandWriterContext>> {
        let prepare_impl = SpritePrepareJobImpl::new(
            self.device_context,
            self.extracted_sprite_data
        );

        Box::new(DefaultPrepareJob::new(prepare_impl))
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}
