use crate::features::sprite::{ExtractedSpriteData, SpriteRenderNodeSet, SpriteRenderFeature, SpriteRenderNode};
use crate::{RenderJobExtractContext, PositionComponent, SpriteComponent, RenderJobWriteContext, RenderJobPrepareContext};
use renderer_base::{DefaultExtractJobImpl, FramePacket, RenderView, PerViewNode, PrepareJob, DefaultPrepareJob, RenderFeatureIndex, RenderFeature, PerFrameNode};
use renderer_base::slab::RawSlabKey;
use crate::features::sprite::prepare::SpritePrepareJobImpl;
use renderer_shell_vulkan::VkDeviceContext;
use renderer_resources::resource_managers::{PipelineSwapchainInfo, ResourceManager, DescriptorSetAllocatorRef};
use ash::vk;
use renderer_assets::pipeline::pipeline::MaterialAsset;
use atelier_assets::loader::handle::Handle;
use renderer_assets::pipeline::image::ImageAsset;
use ash::prelude::VkResult;
use renderer_resources::resource_managers::DescriptorSetArc;

// This is almost copy-pasted from glam. I wanted to avoid pulling in the entire library for a
// single function
pub fn orthographic_rh_gl(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    let a = 2.0 / (right - left);
    let b = 2.0 / (top - bottom);
    let c = -2.0 / (far - near);
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -(far + near) / (far - near);

    [
        [a, 0.0, 0.0, 0.0],
        [0.0, b, 0.0, 0.0],
        [0.0, 0.0, c, 0.0],
        [tx, ty, tz, 1.0],
    ]
}

pub struct SpriteExtractJobImpl {
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    sprite_material: Handle<MaterialAsset>,
    extracted_sprite_data: Vec<Option<ExtractedSpriteData>>,
    per_view_descriptors: Vec<DescriptorSetArc>,
}

impl SpriteExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        descriptor_set_allocator: DescriptorSetAllocatorRef,
        pipeline_info: PipelineSwapchainInfo,
        sprite_material: &Handle<MaterialAsset>,
    ) -> Self {
        SpriteExtractJobImpl {
            device_context,
            descriptor_set_allocator,
            pipeline_info,
            sprite_material: sprite_material.clone(),
            //descriptor_set_per_pass,
            extracted_sprite_data: Default::default(),
            per_view_descriptors: Default::default()
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

        // for view in views {
        //     let layout = extract_context.resource_manager.get_descriptor_set_info(&self.sprite_material, 0, 0);
        //     let mut descriptor_set = self.descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&layout.descriptor_set_layout).unwrap();
        //
        //     let view_proj = view.projection_matrix() * view.view_matrix();
        //
        //     descriptor_set.set_buffer_data(0, &view_proj);
        //     descriptor_set.flush(&mut self.descriptor_set_allocator);
        //
        //     self.per_view_descriptors.push(descriptor_set.descriptor_set().clone());
        // }

        //TODO: Multi-view support for sprites. Not clear on if we want to do a screen-space view specifically
        // for sprites
        let extents_width = 900;
        let extents_height = 600;
        let aspect_ration = extents_width as f32 / extents_height as f32;
        let half_width = 400.0;
        let half_height = 400.0 / aspect_ration;
        let view_proj = orthographic_rh_gl(
            -half_width,
            half_width,
            -half_height,
            half_height,
            -100.0,
            100.0,
        );

        let layout = extract_context.resource_manager.get_descriptor_set_info(&self.sprite_material, 0, 0);
        let mut descriptor_set = self.descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&layout.descriptor_set_layout).unwrap();

        descriptor_set.set_buffer_data(0, &view_proj);
        descriptor_set.flush(&mut self.descriptor_set_allocator);

        self.per_view_descriptors.push(descriptor_set.descriptor_set().clone());
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

        let image_info = extract_context.resource_manager.get_image_info(&sprite_component.image);
        if image_info.is_none() {
            self.extracted_sprite_data.push(None);
            return;
        }
        let image_info = image_info.unwrap();

        let descriptor_set_info = extract_context.resource_manager.get_descriptor_set_info(&self.sprite_material, 0, 1);
        let mut sprite_texture_descriptor = self.descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &descriptor_set_info.descriptor_set_layout,
        ).unwrap();

        sprite_texture_descriptor.set_image(0, image_info.image_view);
        sprite_texture_descriptor.flush(&mut self.descriptor_set_allocator).unwrap();
        let texture_descriptor_set = sprite_texture_descriptor.descriptor_set().clone();

        self.extracted_sprite_data.push(Some(ExtractedSpriteData {
            position: position_component.position,
            texture_size: glam::Vec2::new(50.0, 50.0),
            scale: 1.0,
            rotation: 0.0,
            alpha: sprite_component.alpha,
            texture_descriptor_set
        }));
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
        extract_context: &mut RenderJobExtractContext,
        view: &RenderView,
    ) {
    }

    fn extract_frame_finalize(
        self,
        _extract_context: &mut RenderJobExtractContext,
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let prepare_impl = SpritePrepareJobImpl::new(
            self.device_context,
            self.pipeline_info,
            self.per_view_descriptors[0].clone(),
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
