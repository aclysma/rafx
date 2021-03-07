use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::features::mesh::{LightId, ShadowMapRenderView};
use crate::phases::ShadowMapRenderPhase;
use arrayvec::ArrayVec;
use fnv::FnvHashMap;
use legion::*;
use rafx::framework::{ImageViewResource, ResourceArc};
use rafx::graph::{PreparedRenderGraph, RenderGraphImageUsageId};
use rafx::nodes::{
    ExtractResources, FramePacketBuilder, RenderPhaseMask, RenderPhaseMaskBuilder, RenderView,
    RenderViewDepthRange, RenderViewSet, VisibilityResult,
};
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};

struct RenderViewVisibility {
    render_view: RenderView,
    static_visibility: VisibilityResult,
    dynamic_visibility: VisibilityResult,
}

enum ShadowMapVisibility {
    Single(RenderViewVisibility),
    Cube(ArrayVec<[RenderViewVisibility; 6]>),
}

#[derive(Default)]
pub struct ShadowMapResource {
    // These are populated by recalculate_shadow_map_views()
    pub(super) shadow_map_lookup: FnvHashMap<LightId, usize>,
    pub(super) shadow_map_render_views: Vec<ShadowMapRenderView>,

    // Populated by set_shadow_map_image_resources, during construction of the render graph
    pub(super) image_usage_ids: Vec<RenderGraphImageUsageId>,

    // Populated by set_shadow_map_image_views, after the render graph is constructed and image
    // resources are allocated
    pub(super) shadow_map_image_views: Vec<ResourceArc<ImageViewResource>>,
}

impl ShadowMapResource {
    pub fn shadow_map_render_views(&self) -> &[ShadowMapRenderView] {
        &self.shadow_map_render_views
    }

    pub fn append_render_views(
        &self,
        render_views: &mut Vec<RenderView>,
    ) {
        for shadow_map_view in &self.shadow_map_render_views {
            match shadow_map_view {
                ShadowMapRenderView::Single(view) => {
                    render_views.push(view.clone());
                }
                ShadowMapRenderView::Cube(views) => {
                    for view in views {
                        render_views.push(view.clone());
                    }
                }
            }
        }
    }

    fn clear(&mut self) {
        self.shadow_map_lookup.clear();
        self.shadow_map_render_views.clear();
        self.image_usage_ids.clear();
        self.shadow_map_image_views.clear();
    }

    pub fn recalculate_shadow_map_views(
        &mut self,
        render_view_set: &RenderViewSet,
        extract_resources: &ExtractResources,
        frame_packet_builder: &FramePacketBuilder,
        static_visibility_node_set: &mut StaticVisibilityNodeSet,
        dynamic_visibility_node_set: &mut DynamicVisibilityNodeSet,
    ) {
        self.clear();

        //
        // Determine shadowmap views
        //
        let (shadow_map_lookup, shadow_map_render_views) =
            crate::features::mesh::shadow_map_resource::calculate_shadow_map_views(
                &render_view_set,
                extract_resources,
            );

        self.shadow_map_lookup = shadow_map_lookup;
        self.shadow_map_render_views = shadow_map_render_views;
        self.shadow_map_image_views.clear();

        let mut shadow_map_visibility_results = Vec::default();
        for render_view in &self.shadow_map_render_views {
            match render_view {
                ShadowMapRenderView::Single(view) => shadow_map_visibility_results.push(
                    ShadowMapVisibility::Single(create_render_view_visibility(
                        static_visibility_node_set,
                        dynamic_visibility_node_set,
                        view,
                    )),
                ),
                ShadowMapRenderView::Cube(views) => {
                    shadow_map_visibility_results.push(ShadowMapVisibility::Cube(
                        [
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[0],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[1],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[2],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[3],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[4],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[5],
                            ),
                        ]
                        .into(),
                    ));
                }
            }
        }

        for shadow_map_visibility_result in shadow_map_visibility_results {
            match shadow_map_visibility_result {
                ShadowMapVisibility::Single(view) => {
                    frame_packet_builder.add_view(
                        &view.render_view,
                        &[view.static_visibility, view.dynamic_visibility],
                    );
                }
                ShadowMapVisibility::Cube(views) => {
                    for view in views {
                        let static_visibility = view.static_visibility;
                        frame_packet_builder.add_view(
                            &view.render_view,
                            &[static_visibility, view.dynamic_visibility],
                        );
                    }
                }
            }
        }
    }

    pub fn set_shadow_map_image_usage_ids(
        &mut self,
        image_usage_ids: Vec<RenderGraphImageUsageId>,
    ) {
        assert_eq!(self.shadow_map_render_views.len(), image_usage_ids.len());
        self.image_usage_ids = image_usage_ids;
    }

    pub fn set_shadow_map_image_views(
        &mut self,
        prepared_render_graph: &PreparedRenderGraph,
    ) {
        let shadow_map_image_views: Vec<_> = self
            .image_usage_ids
            .iter()
            .map(|&x| prepared_render_graph.image_view(x).unwrap())
            .collect();

        assert_eq!(
            self.shadow_map_render_views.len(),
            shadow_map_image_views.len()
        );
        self.shadow_map_image_views = shadow_map_image_views;
    }
}

fn create_render_view_visibility(
    static_visibility_node_set: &mut StaticVisibilityNodeSet,
    dynamic_visibility_node_set: &mut DynamicVisibilityNodeSet,
    render_view: &RenderView,
) -> RenderViewVisibility {
    let static_visibility = static_visibility_node_set.calculate_static_visibility(&render_view);
    let dynamic_visibility = dynamic_visibility_node_set.calculate_dynamic_visibility(&render_view);

    log::trace!(
        "shadow view static node count: {}",
        static_visibility.handles.len()
    );

    log::trace!(
        "shadow view dynamic node count: {}",
        dynamic_visibility.handles.len()
    );

    RenderViewVisibility {
        render_view: render_view.clone(),
        static_visibility,
        dynamic_visibility,
    }
}

/// Creates a right-handed perspective projection matrix with [0,1] depth range.
pub fn perspective_rh(
    fov_y_radians: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
) -> glam::Mat4 {
    debug_assert!(z_near > 0.0 && z_far > 0.0);
    let (sin_fov, cos_fov) = (0.5 * fov_y_radians).sin_cos();
    let h = cos_fov / sin_fov;
    let w = h / aspect_ratio;
    let r = z_far / (z_near - z_far);
    glam::Mat4::from_cols(
        glam::Vec4::new(w, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, h, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, r, -1.0),
        glam::Vec4::new(0.0, 0.0, r * z_near, 0.0),
    )
}

#[profiling::function]
fn calculate_shadow_map_views(
    render_view_set: &RenderViewSet,
    extract_resources: &ExtractResources,
) -> (FnvHashMap<LightId, usize>, Vec<ShadowMapRenderView>) {
    let world_fetch = extract_resources.fetch::<World>();
    let world = &*world_fetch;

    let mut shadow_map_render_views = Vec::default();
    let mut shadow_map_lookup = FnvHashMap::default();

    let shadow_map_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<ShadowMapRenderPhase>()
        .build();

    //TODO: The look-at calls in this fn will fail if the light is pointed straight down

    const SHADOW_MAP_RESOLUTION: u32 = 1024;

    let mut query = <(Entity, Read<SpotLightComponent>, Read<PositionComponent>)>::query();
    for (entity, light, position) in query.iter(world) {
        //TODO: Transform direction by rotation
        let eye_position = position.position;
        let light_to = position.position + light.direction;

        let view = glam::Mat4::look_at_rh(eye_position, light_to, glam::Vec3::new(0.0, 0.0, 1.0));

        let near_plane = 0.25;
        let far_plane = 100.0;
        let proj = perspective_rh(light.spotlight_half_angle * 2.0, 1.0, far_plane, near_plane);

        let view = render_view_set.create_view(
            eye_position,
            view,
            proj,
            (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
            RenderViewDepthRange::new_reverse(near_plane, far_plane),
            shadow_map_phase_mask,
            "shadow_map".to_string(),
        );

        let index = shadow_map_render_views.len();
        shadow_map_render_views.push(ShadowMapRenderView::Single(view));
        let old = shadow_map_lookup.insert(LightId::SpotLight(*entity), index);
        assert!(old.is_none());
    }

    let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
    for (entity, light) in query.iter(world) {
        let eye_position = light.direction * -40.0;
        let view = glam::Mat4::look_at_rh(
            eye_position,
            glam::Vec3::zero(),
            glam::Vec3::new(0.0, 0.0, 1.0),
        );

        let near_plane = 0.25;
        let far_plane = 100.0;
        let ortho_projection_size = 10.0;
        let proj = glam::Mat4::orthographic_rh(
            -ortho_projection_size,
            ortho_projection_size,
            -ortho_projection_size,
            ortho_projection_size,
            far_plane,
            near_plane,
        );

        let view = render_view_set.create_view(
            eye_position,
            view,
            proj,
            (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
            RenderViewDepthRange::new_reverse(near_plane, far_plane),
            shadow_map_phase_mask,
            "shadow_map".to_string(),
        );

        let index = shadow_map_render_views.len();
        shadow_map_render_views.push(ShadowMapRenderView::Single(view));
        let old = shadow_map_lookup.insert(LightId::DirectionalLight(*entity), index);
        assert!(old.is_none());
    }

    #[rustfmt::skip]
        // The eye offset and up vector. The directions are per the specification of cubemaps
        let cube_map_view_directions = [
        (glam::Vec3::unit_x(), glam::Vec3::unit_y()),
        (glam::Vec3::unit_x() * -1.0, glam::Vec3::unit_y()),
        (glam::Vec3::unit_y(), glam::Vec3::unit_z() * -1.0),
        (glam::Vec3::unit_y() * -1.0, glam::Vec3::unit_z()),
        (glam::Vec3::unit_z(), glam::Vec3::unit_y()),
        (glam::Vec3::unit_z() * -1.0, glam::Vec3::unit_y()),
    ];

    let mut query = <(Entity, Read<PointLightComponent>, Read<PositionComponent>)>::query();
    for (entity, light, position) in query.iter(world) {
        fn cube_map_face(
            phase_mask: RenderPhaseMask,
            render_view_set: &RenderViewSet,
            light: &PointLightComponent,
            position: glam::Vec3,
            cube_map_view_directions: &(glam::Vec3, glam::Vec3),
        ) -> RenderView {
            //NOTE: Cubemaps always use LH
            let view = glam::Mat4::look_at_lh(
                position,
                position + cube_map_view_directions.0,
                cube_map_view_directions.1,
            );

            let near = 0.25;
            let far = light.range;
            let proj = glam::Mat4::perspective_lh(std::f32::consts::FRAC_PI_2, 1.0, far, near);

            render_view_set.create_view(
                position,
                view,
                proj,
                (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
                RenderViewDepthRange::new_reverse(near, far),
                phase_mask,
                "shadow_map".to_string(),
            )
        }

        #[rustfmt::skip]
            let cube_map_views = [
            cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[0]),
            cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[1]),
            cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[2]),
            cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[3]),
            cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[4]),
            cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[5]),
        ];

        let index = shadow_map_render_views.len();
        shadow_map_render_views.push(ShadowMapRenderView::Cube(cube_map_views));
        let old = shadow_map_lookup.insert(LightId::PointLight(*entity), index);
        assert!(old.is_none());
    }

    (shadow_map_lookup, shadow_map_render_views)
}
