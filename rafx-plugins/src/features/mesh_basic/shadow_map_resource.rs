use super::MeshBasicRenderFeature;
use super::MeshBasicRenderOptions;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use crate::phases::ShadowMapRenderPhase;
use fnv::FnvHashMap;
use legion::*;
use rafx::framework::render_features::RenderFeatureFlagMask;
use rafx::framework::{ImageViewResource, ResourceArc};
use rafx::graph::{PreparedRenderGraph, RenderGraphImageUsageId};
use rafx::rafx_visibility::{
    DepthRange, OrthographicParameters, PerspectiveParameters, Projection,
};
use rafx::render_features::{
    ExtractResources, RenderFeatureMask, RenderFeatureMaskBuilder, RenderPhaseMask,
    RenderPhaseMaskBuilder, RenderView, RenderViewDepthRange, RenderViewSet,
};
use rafx::visibility::{ObjectId, ViewFrustumArc};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum MeshBasicLightId {
    PointLight(ObjectId), // u32 is a face index
    SpotLight(ObjectId),
    DirectionalLight(ObjectId),
}

#[derive(Clone)]
pub enum MeshBasicShadowMapRenderView {
    Single(RenderView), // width, height of texture
    Cube([RenderView; 6]),
}

#[derive(Default)]
pub struct MeshBasicShadowMapResource {
    // These are populated by recalculate_shadow_map_views()
    pub(super) shadow_map_lookup: FnvHashMap<MeshBasicLightId, usize>,
    pub(super) shadow_map_render_views: Vec<MeshBasicShadowMapRenderView>,

    // Populated by set_shadow_map_image_resources, during construction of the render graph
    pub(super) image_usage_ids: Vec<RenderGraphImageUsageId>,

    // Populated by set_shadow_map_image_views, after the render graph is constructed and image
    // resources are allocated
    pub(super) shadow_map_image_views: Vec<ResourceArc<ImageViewResource>>,
}

impl MeshBasicShadowMapResource {
    pub fn shadow_map_render_views(&self) -> &[MeshBasicShadowMapRenderView] {
        &self.shadow_map_render_views
    }

    pub fn append_render_views(
        &self,
        render_views: &mut Vec<RenderView>,
    ) {
        for shadow_map_view in &self.shadow_map_render_views {
            match shadow_map_view {
                MeshBasicShadowMapRenderView::Single(view) => {
                    render_views.push(view.clone());
                }
                MeshBasicShadowMapRenderView::Cube(views) => {
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
    ) {
        self.clear();

        //
        // Determine shadowmap views
        //
        let (shadow_map_lookup, shadow_map_render_views) =
            calculate_shadow_map_views(&render_view_set, extract_resources);

        self.shadow_map_lookup = shadow_map_lookup;
        self.shadow_map_render_views = shadow_map_render_views;
        self.shadow_map_image_views.clear();
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
) -> (
    FnvHashMap<MeshBasicLightId, usize>,
    Vec<MeshBasicShadowMapRenderView>,
) {
    let world_fetch = extract_resources.fetch::<World>();
    let world = &*world_fetch;

    let mut shadow_map_render_views = Vec::default();
    let mut shadow_map_lookup = FnvHashMap::default();

    let shadow_map_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<ShadowMapRenderPhase>()
        .build();

    let render_options = extract_resources.fetch::<MeshBasicRenderOptions>();

    let shadow_map_feature_mask = if render_options.show_surfaces
        && render_options.show_shadows
        && render_options.enable_lighting
    {
        RenderFeatureMaskBuilder::default()
            .add_render_feature::<MeshBasicRenderFeature>()
            .build()
    } else {
        RenderFeatureMask::empty()
    };

    //TODO: The look-at calls in this fn will fail if the light is pointed straight down

    const SHADOW_MAP_RESOLUTION: u32 = 1024;

    let mut query = <(Entity, Read<SpotLightComponent>, Read<TransformComponent>)>::query();
    for (entity, light, transform) in query.iter(world) {
        //TODO: Transform direction by rotation
        let eye_position = transform.translation;
        let light_to = transform.translation + light.direction;

        let view = glam::Mat4::look_at_rh(eye_position, light_to, glam::Vec3::new(0.0, 0.0, 1.0));

        let near_plane = 0.25;
        let far_plane = light.range();

        let view_frustum: ViewFrustumArc = light.view_frustum.clone();
        let projection = Projection::Perspective(PerspectiveParameters::new(
            light.spotlight_half_angle * 2.0,
            1.0,
            near_plane,
            far_plane,
            DepthRange::Reverse,
        ));
        view_frustum.set_projection(&projection).set_transform(
            eye_position,
            light_to,
            glam::Vec3::new(0.0, 0.0, 1.0),
        );

        let view = render_view_set.create_view(
            view_frustum.clone(),
            eye_position,
            view,
            projection.as_rh_mat4(),
            (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
            RenderViewDepthRange::from_projection(&projection),
            shadow_map_phase_mask,
            shadow_map_feature_mask,
            RenderFeatureFlagMask::empty(),
            "shadow_map_spotlight".to_string(),
        );

        let index = shadow_map_render_views.len();
        shadow_map_render_views.push(MeshBasicShadowMapRenderView::Single(view));
        let old =
            shadow_map_lookup.insert(MeshBasicLightId::SpotLight(ObjectId::from(*entity)), index);
        assert!(old.is_none());
    }

    let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
    for (entity, light) in query.iter(world) {
        let eye_position = light.direction * -40.0;
        let view = glam::Mat4::look_at_rh(
            eye_position,
            glam::Vec3::ZERO,
            glam::Vec3::new(0.0, 0.0, 1.0),
        );

        let near_plane = 0.25;
        let far_plane = 1000.0;
        let ortho_projection_size = 10.0;
        let view_frustum: ViewFrustumArc = light.view_frustum.clone();
        let projection = Projection::Orthographic(OrthographicParameters::new(
            -ortho_projection_size,
            ortho_projection_size,
            -ortho_projection_size,
            ortho_projection_size,
            near_plane,
            far_plane,
            DepthRange::Reverse,
        ));

        view_frustum.set_projection(&projection).set_transform(
            eye_position,
            glam::Vec3::ZERO,
            glam::Vec3::new(0.0, 0.0, 1.0),
        );

        let view = render_view_set.create_view(
            view_frustum,
            eye_position,
            view,
            projection.as_rh_mat4(),
            (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
            RenderViewDepthRange::from_projection(&projection),
            shadow_map_phase_mask,
            shadow_map_feature_mask,
            RenderFeatureFlagMask::empty(),
            "shadow_map_directional".to_string(),
        );

        let index = shadow_map_render_views.len();
        shadow_map_render_views.push(MeshBasicShadowMapRenderView::Single(view));
        let old = shadow_map_lookup.insert(
            MeshBasicLightId::DirectionalLight(ObjectId::from(*entity)),
            index,
        );
        assert!(old.is_none());
    }

    #[rustfmt::skip]
    // The eye offset and up vector. The directions are per the specification of cubemaps
    let cube_map_view_directions = [
        (glam::Vec3::X, glam::Vec3::Y),
        (glam::Vec3::X * -1.0, glam::Vec3::Y),
        (glam::Vec3::Y, glam::Vec3::Z * -1.0),
        (glam::Vec3::Y * -1.0, glam::Vec3::Z),
        (glam::Vec3::Z, glam::Vec3::Y),
        (glam::Vec3::Z * -1.0, glam::Vec3::Y),
    ];

    let mut query = <(Entity, Read<PointLightComponent>, Read<TransformComponent>)>::query();
    for (entity, light, transform) in query.iter(world) {
        fn cube_map_face(
            phase_mask: RenderPhaseMask,
            feature_mask: RenderFeatureMask,
            render_view_set: &RenderViewSet,
            light: &PointLightComponent,
            position: glam::Vec3,
            face_idx: usize,
            cube_map_view_directions: &(glam::Vec3, glam::Vec3),
        ) -> RenderView {
            let near = 0.25;
            let far = light.range();

            let view_frustum: ViewFrustumArc = light.view_frustums[face_idx].clone();
            let projection = Projection::Perspective(PerspectiveParameters::new(
                std::f32::consts::FRAC_PI_2,
                1.0,
                near,
                far,
                DepthRange::Reverse,
            ));

            view_frustum.set_projection(&projection).set_transform(
                position,
                position + cube_map_view_directions.0,
                cube_map_view_directions.1,
            );

            // NOTE: Cubemaps always use LH
            let view = glam::Mat4::look_at_lh(
                position,
                position + cube_map_view_directions.0,
                cube_map_view_directions.1,
            );

            render_view_set.create_view(
                view_frustum,
                position,
                view,
                projection.as_lh_mat4(),
                (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
                RenderViewDepthRange::from_projection(&projection),
                phase_mask,
                feature_mask,
                RenderFeatureFlagMask::empty(),
                format!("shadow_map_point_light_face_{}", face_idx),
            )
        }

        #[rustfmt::skip]
        let cube_map_views = [
            cube_map_face(shadow_map_phase_mask, shadow_map_feature_mask, &render_view_set, light, transform.translation, 0, &cube_map_view_directions[0]),
            cube_map_face(shadow_map_phase_mask, shadow_map_feature_mask, &render_view_set, light, transform.translation, 1, &cube_map_view_directions[1]),
            cube_map_face(shadow_map_phase_mask, shadow_map_feature_mask, &render_view_set, light, transform.translation, 2, &cube_map_view_directions[2]),
            cube_map_face(shadow_map_phase_mask, shadow_map_feature_mask, &render_view_set, light, transform.translation, 3, &cube_map_view_directions[3]),
            cube_map_face(shadow_map_phase_mask, shadow_map_feature_mask, &render_view_set, light, transform.translation, 4, &cube_map_view_directions[4]),
            cube_map_face(shadow_map_phase_mask, shadow_map_feature_mask, &render_view_set, light, transform.translation, 5, &cube_map_view_directions[5]),
        ];

        let index = shadow_map_render_views.len();
        shadow_map_render_views.push(MeshBasicShadowMapRenderView::Cube(cube_map_views));
        let old =
            shadow_map_lookup.insert(MeshBasicLightId::PointLight(ObjectId::from(*entity)), index);
        assert!(old.is_none());
    }

    (shadow_map_lookup, shadow_map_render_views)
}
