use rafx::render_feature_extract_job_predule::*;

use super::*;
use crate::components::TransformComponent;
use legion::{EntityStore, World};
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_ref_map::ResourceRefBorrow;
use rafx::distill::loader::handle::Handle;

pub struct SpriteExtractJob<'extract> {
    world: ResourceRefBorrow<'extract, World>,
    asset_manager: AssetManagerExtractRef,
    sprite_material: Handle<MaterialAsset>,
    render_objects: SpriteRenderObjectSet,
}

impl<'extract> SpriteExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<SpriteFramePacket>,
        sprite_material: Handle<MaterialAsset>,
        render_objects: SpriteRenderObjectSet,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                world: extract_context.extract_resources.fetch::<World>(),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                sprite_material,
                render_objects,
            },
            extract_context,
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for SpriteExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        context
            .frame_packet()
            .per_frame_data()
            .set(SpritePerFrameData {
                sprite_material_pass: self
                    .asset_manager
                    .committed_asset(&self.sprite_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
            });
    }

    fn extract_render_object_instance(
        &self,
        job_context: &mut RenderObjectsJobContext<'extract, SpriteRenderObject>,
        context: &ExtractRenderObjectInstanceContext<'extract, '_, Self>,
    ) {
        let render_object_static_data = job_context
            .render_objects
            .get_id(context.render_object_id());

        let image_asset = self
            .asset_manager
            .committed_asset(&render_object_static_data.image);

        context.set_render_object_instance_data(image_asset.and_then(|image_asset| {
            let entry = self.world.entry_ref(context.object_id().into()).unwrap();
            let transform_component = entry.get_component::<TransformComponent>().unwrap();
            let texture_extents = image_asset.image.get_raw().image.texture_def().extents;
            Some(SpriteRenderObjectInstanceData {
                position: transform_component.translation,
                texture_size: glam::Vec2::new(
                    texture_extents.width as f32,
                    texture_extents.height as f32,
                ),
                scale: transform_component.scale,
                rotation: transform_component.rotation,
                color: render_object_static_data
                    .tint
                    .extend(render_object_static_data.alpha),
                image_view: image_asset.image_view.clone(),
            })
        }));
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    fn new_render_object_instance_job_context(
        &'extract self
    ) -> Option<RenderObjectsJobContext<'extract, SpriteRenderObject>> {
        Some(RenderObjectsJobContext::new(self.render_objects.read()))
    }

    type RenderObjectInstanceJobContextT = RenderObjectsJobContext<'extract, SpriteRenderObject>;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = SpriteRenderFeatureTypes;
}
