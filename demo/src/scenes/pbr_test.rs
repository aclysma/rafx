use crate::input::InputResource;
use crate::scenes::util::{FlyCamera, SpawnablePrefab};
use crate::time::TimeState;
use crate::RenderOptions;
use legion::{Resources, World};
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::RenderViewDepthRange;
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{ViewFrustumArc, VisibilityResource};

pub(super) struct PbrTestScene {
    main_view_frustum: ViewFrustumArc,
    fly_camera: FlyCamera,
}

impl PbrTestScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();
        render_options.show_skybox = false;

        let mut fly_camera = FlyCamera::default();
        fly_camera.position = glam::Vec3::new(15.0, -90.0, 15.0);
        fly_camera.yaw = std::f32::consts::FRAC_PI_2;
        fly_camera.lock_view = true;

        let prefab = SpawnablePrefab::blocking_load_from_symbol_name(
            resources,
            "assets://demo/pbr-test/Scene.001.blender_prefab",
        );
        prefab.spawn_prefab(world, resources);

        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        let main_view_frustum = visibility_resource.register_view_frustum();

        PbrTestScene {
            main_view_frustum,
            fly_camera,
        }
    }
}

impl super::TestScene for PbrTestScene {
    fn update(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
    ) {
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
    let (phase_mask_builder, feature_mask_builder, feature_flag_mask_builder) =
        super::util::default_main_view_masks(render_options);

    let aspect_ratio = viewports_resource.main_window_size.width as f32
        / viewports_resource.main_window_size.height as f32;

    //
    // Fly camera
    //

    let eye = fly_camera.position;
    let look_at = fly_camera.position + fly_camera.look_dir;
    let up = fly_camera.up_dir;

    let view = glam::Mat4::look_at_rh(eye, look_at, up);

    let fov_y_radians = 0.4;
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
