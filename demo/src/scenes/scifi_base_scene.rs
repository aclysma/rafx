use crate::input::InputResource;
use crate::scenes::util::{DemoCamera, PathData, SpawnablePrefab};
use crate::time::TimeState;
use crate::RenderOptions;
use legion::{Resources, World};
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::RenderViewDepthRange;
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{ViewFrustumArc, VisibilityResource};

pub(super) struct ScifiBaseScene {
    main_view_frustum: ViewFrustumArc,
    demo_camera: DemoCamera,
}

impl ScifiBaseScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();

        super::util::set_ambient_light(resources, glam::Vec3::new(0.005, 0.005, 0.005));

        let camera_path_data_str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/private/scifi_base/camera_path.json"
        ));
        let camera_path_data: Vec<PathData> = serde_json::from_str(camera_path_data_str).unwrap();

        let mut demo_camera = DemoCamera::new_with_path(camera_path_data.clone());
        demo_camera.fly_camera.position = glam::Vec3::new(-107.25943, 55.35553, 14.5860615);
        demo_camera.fly_camera.pitch = -0.14614205;
        demo_camera.fly_camera.yaw = 2.9897811;
        demo_camera.fly_camera.lock_view = true;

        let prefab = SpawnablePrefab::blocking_load_from_symbol_name(
            resources,
            "db:/assets/demo/scifi_base/base_full/Scene.blender_prefab",
        );
        prefab.spawn_prefab(world, resources);

        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        let main_view_frustum = visibility_resource.register_view_frustum();

        ScifiBaseScene {
            main_view_frustum,
            demo_camera,
        }
    }
}

impl super::TestScene for ScifiBaseScene {
    fn update(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
    ) {
        {
            let input_resource = resources.get::<InputResource>().unwrap();
            let time_state = resources.get::<TimeState>().unwrap();
            self.demo_camera
                .update(input_resource.input_state(), &*time_state);
        }

        {
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            let render_options = resources.get::<RenderOptions>().unwrap();

            update_main_view_3d(
                &*render_options,
                &mut self.main_view_frustum,
                &mut *viewports_resource,
                &self.demo_camera,
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
    render_options: &RenderOptions,
    main_view_frustum: &mut ViewFrustumArc,
    viewports_resource: &mut ViewportsResource,
    camera: &DemoCamera,
) {
    let (phase_mask_builder, feature_mask_builder, feature_flag_mask_builder) =
        super::util::default_main_view_masks(render_options);

    let aspect_ratio = viewports_resource.main_window_size.width as f32
        / viewports_resource.main_window_size.height as f32;

    let eye = camera.position();
    let look_at = camera.position() + camera.look_dir();
    let up = camera.up_dir();

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
