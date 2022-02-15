use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::TransparentRenderPhase;
use crate::shaders::tile_layer::tile_layer_vert;

pub struct TileLayerPrepareJob {
    render_objects: TileLayerRenderObjectSet,
}

impl TileLayerPrepareJob {
    pub fn new<'prepare>(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<TileLayerFramePacket>,
        submit_packet: Box<TileLayerSubmitPacket>,
        render_objects: TileLayerRenderObjectSet,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self { render_objects },
            prepare_context,
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for TileLayerPrepareJob {
    fn prepare_render_object_instance_per_view(
        &self,
        job_context: &mut RenderObjectsJobContext<'prepare, TileLayerRenderObject>,
        context: &PrepareRenderObjectInstancePerViewContext<'prepare, '_, Self>,
    ) {
        // TODO(dvd): This should probably just be a loop in end per-view.
        let render_object_id = context.render_object_id();
        let render_object = job_context.render_objects.get_id(render_object_id);
        let layer_z_position = render_object.z_position;
        let distance = (layer_z_position - context.view().eye_position().z).abs();
        context.push_submit_node::<TransparentRenderPhase>(
            TileLayerSubmitNodeData {
                render_object_id: *render_object_id,
            },
            0,
            distance,
        );
    }

    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        if per_frame_data.tile_layer_material_pass.is_none() {
            return;
        }

        let tile_layer_material_pass = per_frame_data.tile_layer_material_pass.as_ref().unwrap();
        let per_view_descriptor_set_layout = &tile_layer_material_pass
            .get_raw()
            .descriptor_set_layouts[tile_layer_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX];

        let mut descriptor_set_allocator =
            context.resource_context().create_descriptor_set_allocator();
        let view_submit_packet = context.view_submit_packet();
        view_submit_packet
            .per_view_submit_data()
            .set(TileLayerPerViewSubmitData {
                descriptor_set_arc: descriptor_set_allocator
                    .create_descriptor_set_with_writer(
                        per_view_descriptor_set_layout,
                        tile_layer_vert::DescriptorSet0Args {
                            uniform_buffer: &tile_layer_vert::ArgsUniform {
                                mvp: context.view().view_proj().to_cols_array_2d(),
                            },
                        },
                    )
                    .ok(),
            })
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    fn new_render_object_instance_per_view_job_context(
        &'prepare self
    ) -> Option<RenderObjectsJobContext<'prepare, TileLayerRenderObject>> {
        Some(RenderObjectsJobContext::new(self.render_objects.read()))
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT =
        RenderObjectsJobContext<'prepare, TileLayerRenderObject>;

    type FramePacketDataT = TileLayerRenderFeatureTypes;
    type SubmitPacketDataT = TileLayerRenderFeatureTypes;
}
