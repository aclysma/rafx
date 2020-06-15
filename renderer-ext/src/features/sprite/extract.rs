use crate::features::sprite::{ExtractedSpriteData, SpriteRenderNodeSet, SpriteRenderFeature, SpriteRenderNode};
use crate::{RenderJobExtractContext, PositionComponent, SpriteComponent, RenderJobWriteContext, RenderJobPrepareContext};
use renderer_base::{DefaultExtractJobImpl, FramePacket, RenderView, PerViewNode, PrepareJob, DefaultPrepareJob, RenderFeatureIndex, RenderFeature, PerFrameNode};
use renderer_base::slab::RawSlabKey;
use crate::features::sprite::prepare::SpritePrepareJobImpl;
use renderer_shell_vulkan::VkDeviceContext;
use crate::resource_managers::PipelineSwapchainInfo;
use ash::vk;

pub struct SpriteExtractJobImpl {
    device_context: VkDeviceContext,
    pipeline_info: PipelineSwapchainInfo,
    descriptor_set_per_pass: vk::DescriptorSet,
    extracted_sprite_data: Vec<ExtractedSpriteData>,
    descriptor_set_per_texture: vk::DescriptorSet,
}

impl SpriteExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        pipeline_info: PipelineSwapchainInfo,
        descriptor_set_per_pass: vk::DescriptorSet,
        descriptor_set_per_texture: vk::DescriptorSet, //TODO: TEMPORARY
    ) -> Self {
        SpriteExtractJobImpl {
            device_context,
            pipeline_info,
            descriptor_set_per_pass,
            extracted_sprite_data: Default::default(),
            descriptor_set_per_texture
        }
    }
}

impl DefaultExtractJobImpl<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext> for SpriteExtractJobImpl {
    fn extract_begin(
        &mut self,
        _extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) {
        self.extracted_sprite_data
            .reserve(frame_packet.frame_node_count(self.feature_index()) as usize);
    }

    fn extract_frame_node(
        &mut self,
        extract_context: &RenderJobExtractContext,
        frame_node: PerFrameNode,
        frame_node_index: u32,
    ) {
        let render_node_index = frame_node.render_node_index();
        let render_node_handle = RawSlabKey::<SpriteRenderNode>::new(render_node_index);

        let sprite_nodes = extract_context.resources.get::<SpriteRenderNodeSet>().unwrap();
        let sprite_render_node = sprite_nodes.sprites.get(render_node_handle).unwrap();

        let position_component = extract_context
            .world
            .get_component::<PositionComponent>(sprite_render_node.entity)
            .unwrap();
        let sprite_component = extract_context
            .world
            .get_component::<SpriteComponent>(sprite_render_node.entity)
            .unwrap();

        //TODO: Consider having cached descriptor set in sprite_render_node
        //sprite_render_node.

        let image = sprite_component.image.clone();
        //extract_context.resources.get_mut::<Res>
        // make descriptor set?

        self.extracted_sprite_data.push(ExtractedSpriteData {
            position: position_component.position,
            texture_size: glam::Vec2::new(100.0, 100.0),
            scale: 1.0,
            rotation: 0.0,
            alpha: sprite_component.alpha,
            texture_descriptor_set: self.descriptor_set_per_texture
        });
    }

    fn extract_view_node(
        &mut self,
        _extract_context: &RenderJobExtractContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    ) {

    }

    fn extract_view_finalize(
        &mut self,
        _extract_context: &RenderJobExtractContext,
        _view: &RenderView,
    ) {

    }

    fn extract_frame_finalize(
        self,
        _extract_context: &RenderJobExtractContext,
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let prepare_impl = SpritePrepareJobImpl::new(
            self.device_context,
            self.pipeline_info,
            self.descriptor_set_per_pass,
            self.extracted_sprite_data,
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