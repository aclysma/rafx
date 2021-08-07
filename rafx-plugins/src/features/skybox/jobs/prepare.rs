use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::OpaqueRenderPhase;
use rafx::framework::ResourceContext;

pub struct SkyboxPrepareJob {
    resource_context: ResourceContext,
}

impl SkyboxPrepareJob {
    pub fn new<'prepare>(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<SkyboxFramePacket>,
        submit_packet: Box<SkyboxSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self {
                resource_context: prepare_context.resource_context.clone(),
            },
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for SkyboxPrepareJob {
    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();

        if let Some(skybox_material) = &per_frame_data.skybox_material_pass {
            if let Some(skybox_texture) = &per_frame_data.skybox_texture {
                let view = context.view();

                // Skyboxes assume Y up and we're Z up, so "fix" it by adding a rotation about X axis.
                // This effectively applies a rotation to the skybox
                let skybox_rotation = glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);

                let descriptor_set_layouts = skybox_material.get_raw().descriptor_set_layouts;

                context
                    .view_submit_packet()
                    .per_view_submit_data()
                    .set(SkyboxPerViewSubmitData {
                        descriptor_set_arc: descriptor_set_allocator
                            .create_descriptor_set_with_writer(
                                &descriptor_set_layouts
                                    [shaders::skybox_frag::SKYBOX_TEX_DESCRIPTOR_SET_INDEX],
                                shaders::skybox_frag::DescriptorSet0Args {
                                    skybox_tex: &skybox_texture,
                                    uniform_buffer: &shaders::skybox_frag::ArgsUniform {
                                        inverse_view: (view.view_matrix() * skybox_rotation)
                                            .inverse()
                                            .to_cols_array_2d(),
                                        inverse_projection: view
                                            .projection_matrix()
                                            .inverse()
                                            .to_cols_array_2d(),
                                    },
                                },
                            )
                            .ok(),
                    });

                context
                    .view_submit_packet()
                    .push_submit_node::<OpaqueRenderPhase>((), 0, 0.);
            }
        }
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = SkyboxRenderFeatureTypes;
    type SubmitPacketDataT = SkyboxRenderFeatureTypes;
}
