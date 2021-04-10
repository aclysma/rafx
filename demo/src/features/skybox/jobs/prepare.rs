use rafx::render_feature_prepare_job_predule::*;

use super::{RenderFeatureType, SkyboxWriteJob};
use crate::phases::OpaqueRenderPhase;
use rafx::framework::{ImageViewResource, MaterialPassResource, ResourceArc};

pub struct SkyboxPrepareJob {
    skybox_material: ResourceArc<MaterialPassResource>,
    skybox_texture: ResourceArc<ImageViewResource>,
}

impl SkyboxPrepareJob {
    pub fn new(
        skybox_material: ResourceArc<MaterialPassResource>,
        skybox_texture: ResourceArc<ImageViewResource>,
    ) -> Self {
        SkyboxPrepareJob {
            skybox_material,
            skybox_texture,
        }
    }
}

impl PrepareJob for SkyboxPrepareJob {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn WriteJob>, FeatureSubmitNodes) {
        profiling::scope!(super::PREPARE_SCOPE_NAME);

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        let mut writer = Box::new(SkyboxWriteJob::new(self.skybox_material.clone()));

        // Skyboxes assume Y up and we're Z up, so "fix" it by adding a rotation about X axis.
        // This effectively applies a rotation to the skybox
        let skybox_rotation = glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);

        let mut submit_nodes = FeatureSubmitNodes::default();
        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());

            if view.is_relevant::<OpaqueRenderPhase, RenderFeatureType>() {
                // Set up a descriptor set pointing at the image so we can sample from it
                let descriptor_set_layouts = self.skybox_material.get_raw().descriptor_set_layouts;
                let skybox_material_dyn_set0 = descriptor_set_allocator
                    .create_descriptor_set(
                        &descriptor_set_layouts
                            [shaders::skybox_frag::SKYBOX_TEX_DESCRIPTOR_SET_INDEX],
                        shaders::skybox_frag::DescriptorSet0Args {
                            skybox_tex: &self.skybox_texture,
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
                    .unwrap();

                let submit_node_id = writer.push_submit_node(skybox_material_dyn_set0.clone());
                view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(submit_node_id, 0, 0.0);
            }

            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
