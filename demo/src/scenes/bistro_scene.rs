use crate::input::InputResource;
use crate::time::TimeState;
use crate::RenderOptions;
use distill::loader::handle::Handle;
use legion::{Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::{
    RenderFeatureFlagMaskBuilder, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder,
    RenderViewDepthRange,
};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{ViewFrustumArc, VisibilityRegion};
use rafx_plugins::assets::mesh_basic::PrefabBasicAsset;
use rafx_plugins::features::debug3d::Debug3DRenderFeature;
use rafx_plugins::features::mesh_basic::{
    MeshBasicNoShadowsRenderFeatureFlag, MeshBasicRenderFeature, MeshBasicRenderObjectSet,
    MeshBasicRenderOptions, MeshBasicUnlitRenderFeatureFlag, MeshBasicUntexturedRenderFeatureFlag,
    MeshBasicWireframeRenderFeatureFlag,
};
use rafx_plugins::features::skybox::SkyboxRenderFeature;
use rafx_plugins::features::sprite::SpriteRenderFeature;
use rafx_plugins::features::text::TextRenderFeature;
use rafx_plugins::features::tile_layer::TileLayerRenderFeature;
use rafx_plugins::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase,
    WireframeRenderPhase,
};

use super::util::FlyCamera;

pub(super) struct BistroScene {
    main_view_frustum: ViewFrustumArc,
    fly_camera: FlyCamera,
}

impl BistroScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();

        let mut mesh_render_options = resources.get_mut::<MeshBasicRenderOptions>().unwrap();
        mesh_render_options.ambient_light = glam::Vec3::new(0.005, 0.005, 0.005);

        let mut mesh_render_objects = resources.get_mut::<MeshBasicRenderObjectSet>().unwrap();

        let visibility_region = resources.get::<VisibilityRegion>().unwrap();

        let mut fly_camera = FlyCamera::default();
        fly_camera.position = glam::Vec3::new(-15.510543, 2.3574839, 5.751496);
        fly_camera.pitch = -0.23093751;
        fly_camera.yaw = -0.16778418;
        fly_camera.lock_view = true;

        let prefab_asset_handle: Handle<PrefabBasicAsset> =
            asset_resource.load_asset_path("bistro/bistro_merged/Scene.blender_prefab");
        asset_manager
            .wait_for_asset_to_load(&prefab_asset_handle, &mut asset_resource, "bistro scene")
            .unwrap();
        let prefab_asset = asset_resource.asset(&prefab_asset_handle).unwrap().clone();

        super::util::spawn_prefab(
            world,
            resources,
            &mut *asset_manager,
            &mut *asset_resource,
            &mut *mesh_render_objects,
            &*visibility_region,
            &prefab_asset,
        );

        let main_view_frustum = visibility_region.register_view_frustum();

        BistroScene {
            main_view_frustum,
            fly_camera,
        }
    }
}

impl super::TestScene for BistroScene {
    fn update(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
    ) {
        //let mut debug_draw = resources.get_mut::<Debug3DResource>().unwrap();
        //super::add_light_debug_draw(&resources, &world);

        {
            let input_resource = resources.get::<InputResource>().unwrap();
            let time_state = resources.get::<TimeState>().unwrap();
            self.fly_camera
                .update(input_resource.input_state(), &*time_state);
        }

        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            let render_options = resources.get::<RenderOptions>().unwrap();

            update_main_view_3d(
                &*time_state,
                &*render_options,
                &mut self.main_view_frustum,
                &mut *viewports_resource,
                &self.fly_camera,
            );
        }
    }

    fn process_input(
        &mut self,
        _world: &mut World,
        _resources: &Resources,
        _event: &winit::event::Event<()>,
    ) {
    }
}

#[profiling::function]
fn update_main_view_3d(
    _time_state: &TimeState,
    render_options: &RenderOptions,
    main_view_frustum: &mut ViewFrustumArc,
    viewports_resource: &mut ViewportsResource,
    fly_camera: &FlyCamera,
) {
    let phase_mask_builder = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DepthPrepassRenderPhase>()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<WireframeRenderPhase>()
        .add_render_phase::<UiRenderPhase>();

    let mut feature_mask_builder = RenderFeatureMaskBuilder::default()
        .add_render_feature::<MeshBasicRenderFeature>()
        .add_render_feature::<SpriteRenderFeature>()
        .add_render_feature::<TileLayerRenderFeature>();

    #[cfg(feature = "egui")]
    {
        feature_mask_builder = feature_mask_builder
            .add_render_feature::<rafx_plugins::features::egui::EguiRenderFeature>();
    }

    if render_options.show_text {
        feature_mask_builder = feature_mask_builder.add_render_feature::<TextRenderFeature>();
    }

    if render_options.show_debug3d {
        feature_mask_builder = feature_mask_builder.add_render_feature::<Debug3DRenderFeature>();
    }

    if render_options.show_skybox {
        feature_mask_builder = feature_mask_builder.add_render_feature::<SkyboxRenderFeature>();
    }

    let mut feature_flag_mask_builder = RenderFeatureFlagMaskBuilder::default();

    if render_options.show_wireframes {
        feature_flag_mask_builder = feature_flag_mask_builder
            .add_render_feature_flag::<MeshBasicWireframeRenderFeatureFlag>();
    }

    if !render_options.enable_lighting {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshBasicUnlitRenderFeatureFlag>();
    }

    if !render_options.enable_textures {
        feature_flag_mask_builder = feature_flag_mask_builder
            .add_render_feature_flag::<MeshBasicUntexturedRenderFeatureFlag>();
    }

    if !render_options.show_shadows {
        feature_flag_mask_builder = feature_flag_mask_builder
            .add_render_feature_flag::<MeshBasicNoShadowsRenderFeatureFlag>();
    }

    let aspect_ratio = viewports_resource.main_window_size.width as f32
        / viewports_resource.main_window_size.height as f32;

    let eye = fly_camera.position;
    let look_at = fly_camera.position + fly_camera.look_dir;
    let up = glam::Vec3::Z;

    let view = glam::Mat4::look_at_rh(eye, look_at, up);

    let fov_y_radians = std::f32::consts::FRAC_PI_4;
    let near_plane = 0.01;

    let projection = Projection::Perspective(PerspectiveParameters::new(
        fov_y_radians,
        aspect_ratio,
        near_plane,
        10000.,
        DepthRange::InfiniteReverse,
    ));

    main_view_frustum
        .set_projection(&projection)
        .set_transform(eye, look_at, up);

    viewports_resource.main_view_meta = Some(RenderViewMeta {
        view_frustum: main_view_frustum.clone(),
        eye_position: eye,
        view,
        proj: projection.as_rh_mat4(),
        depth_range: RenderViewDepthRange::from_projection(&projection),
        render_phase_mask: phase_mask_builder.build(),
        render_feature_mask: feature_mask_builder.build(),
        render_feature_flag_mask: feature_flag_mask_builder.build(),
        debug_name: "main".to_string(),
    });
}
