use crate::features::sprite::{ExtractedSpriteData, SpriteRenderNodeSet, SpriteRenderFeature, SpriteRenderNode};
use crate::{RenderJobExtractContext, PositionComponent, SpriteComponent, RenderJobWriteContext, RenderJobPrepareContext};
use renderer_base::{DefaultExtractJobImpl, FramePacket, RenderView, PerViewNode, PrepareJob, DefaultPrepareJob, RenderFeatureIndex, RenderFeature, PerFrameNode};
use renderer_base::slab::RawSlabKey;
use crate::features::sprite::prepare::SpritePrepareJobImpl;
use renderer_shell_vulkan::VkDeviceContext;
use crate::resource_managers::{PipelineSwapchainInfo, ResourceManager, DescriptorSetAllocatorRef};
use ash::vk;
use crate::pipeline::pipeline::MaterialAsset;
use atelier_assets::loader::handle::Handle;
use crate::pipeline::image::ImageAsset;
use ash::prelude::VkResult;
use crate::resource_managers::DescriptorSetArc;

pub struct SpriteExtractJobImpl {
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    sprite_material: Handle<MaterialAsset>,
    descriptor_set_per_pass: DescriptorSetArc,
    extracted_sprite_data: Vec<ExtractedSpriteData>,
}

impl SpriteExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        descriptor_set_allocator: DescriptorSetAllocatorRef,
        pipeline_info: PipelineSwapchainInfo,
        sprite_material: &Handle<MaterialAsset>,
        descriptor_set_per_pass: DescriptorSetArc,
    ) -> Self {
        SpriteExtractJobImpl {
            device_context,
            descriptor_set_allocator,
            pipeline_info,
            sprite_material: sprite_material.clone(),
            descriptor_set_per_pass,
            extracted_sprite_data: Default::default(),
        }
    }
}

impl DefaultExtractJobImpl<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext> for SpriteExtractJobImpl {
    fn extract_begin(
        &mut self,
        extract_context: &mut RenderJobExtractContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) {
        self.extracted_sprite_data
            .reserve(frame_packet.frame_node_count(self.feature_index()) as usize);
    }

    fn extract_frame_node(
        &mut self,
        extract_context: &mut RenderJobExtractContext,
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

        let descriptor_set_info = extract_context.resource_manager.get_descriptor_set_info(&self.sprite_material, 0, 1);
        let mut sprite_texture_descriptor = self.descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &descriptor_set_info.descriptor_set_layout,
        ).unwrap();

        let image_info = extract_context.resource_manager.get_image_info(&sprite_component.image);
        sprite_texture_descriptor.set_image(0, image_info.image_view);
        sprite_texture_descriptor.flush(&mut self.descriptor_set_allocator).unwrap();
        let texture_descriptor_set = sprite_texture_descriptor.descriptor_set().clone();

        self.extracted_sprite_data.push(ExtractedSpriteData {
            position: position_component.position,
            texture_size: glam::Vec2::new(100.0, 100.0),
            scale: 1.0,
            rotation: 0.0,
            alpha: sprite_component.alpha,
            texture_descriptor_set
        });
    }

    fn extract_view_node(
        &mut self,
        _extract_context: &mut RenderJobExtractContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    ) {

    }

    fn extract_view_finalize(
        &mut self,
        _extract_context: &mut RenderJobExtractContext,
        _view: &RenderView,
    ) {

    }

    fn extract_frame_finalize(
        self,
        _extract_context: &mut RenderJobExtractContext,
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
