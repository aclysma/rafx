use crate::features::sprite::{ExtractedSpriteData, SpriteRenderNodeSet, SpriteRenderFeature, SpriteRenderNode};
use crate::{RenderJobExtractContext, PositionComponent, SpriteComponent, RenderJobWriteContext, RenderJobPrepareContext};
use renderer_base::{DefaultExtractJobImpl, FramePacket, RenderView, PerViewNode, PrepareJob, DefaultPrepareJob, RenderFeatureIndex, RenderFeature, PerFrameNode};
use renderer_base::slab::RawSlabKey;
use crate::features::sprite::prepare::SpritePrepareJobImpl;
use renderer_shell_vulkan::VkDeviceContext;
use crate::resource_managers::{PipelineSwapchainInfo, ResourceManager};
use ash::vk;
use crate::pipeline::pipeline::MaterialAsset;
use atelier_assets::loader::handle::Handle;
use crate::pipeline::image::ImageAsset;
use ash::prelude::VkResult;
use crate::resource_managers::DescriptorSetArc;

//TODO: Some of the work being done during extraction may be ok (like looking up an image handle)
// but I'd prefer descriptor set creation occur during the prepare phase
fn create_per_image_descriptor(
    resource_manager: &mut ResourceManager,
    sprite_material: &Handle<MaterialAsset>,
    image_handle: &Handle<ImageAsset>,
) -> VkResult<DescriptorSetArc> {
    let descriptor_set_info = resource_manager.get_descriptor_set_info(sprite_material, 0, 1);
    let mut sprite_texture_descriptor = resource_manager.create_dyn_descriptor_set_uninitialized(
        &descriptor_set_info.descriptor_set_layout_def,
    )?;

    let image_info = resource_manager.get_image_info(image_handle);
    sprite_texture_descriptor.set_image(0, image_info.image_view);
    sprite_texture_descriptor.flush(resource_manager);
    Ok(sprite_texture_descriptor.descriptor_set().clone())
}

pub struct SpriteExtractJobImpl {
    device_context: VkDeviceContext,
    pipeline_info: PipelineSwapchainInfo,
    sprite_material: Handle<MaterialAsset>,
    descriptor_set_per_pass: vk::DescriptorSet,
    extracted_sprite_data: Vec<ExtractedSpriteData>,
}

impl SpriteExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        pipeline_info: PipelineSwapchainInfo,
        sprite_material: &Handle<MaterialAsset>,
        descriptor_set_per_pass: vk::DescriptorSet,
    ) -> Self {
        SpriteExtractJobImpl {
            device_context,
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
        _extract_context: &mut RenderJobExtractContext,
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

        //TODO: Consider having cached descriptor set in sprite_render_node
        //sprite_render_node.

        // let image = sprite_component.image.clone();
        // //extract_context.resources.get_mut::<Res>
        // // make descriptor set?
        //let mut resource_manager = extract_context.resources.get_mut::<ResourceManager>().unwrap();
        //
        // resource_manager.create_dyn_descriptor_set_uninitialized();
        // resource_manager.get_image_info();

        //let mut resource_manager = extract_context.resources.get_mut::<ResourceManager>().unwrap();
        let texture_descriptor_set_arc = create_per_image_descriptor(
            &mut extract_context.resource_manager, //TODO: We need a thread-safe way to create descriptor sets..
            //&mut *resource_manager,
            &self.sprite_material,
            &sprite_component.image
        ).unwrap();


        let texture_descriptor_set = texture_descriptor_set_arc.get();
        //let texture_descriptor_set = texture_descriptor_set_arc.get_raw_for_gpu_read(&*resource_manager);

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
