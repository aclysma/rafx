use super::util::SpawnableMesh;
use crate::time::TimeState;
use crate::RenderOptions;
use glam::Vec3;
use legion::{IntoQuery, Read};
use legion::{Resources, World, Write};
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::RenderViewDepthRange;
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{ViewFrustumArc, VisibilityResource};
use rafx_plugins::components::DirectionalLightComponent;
use rafx_plugins::components::{PointLightComponent, TransformComponent};
use std::time::Duration;

pub(super) struct TaaTestScene {
    main_view_frustum: ViewFrustumArc,
}

impl TaaTestScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();
        render_options.show_skybox = false;
        render_options.enable_lighting = false;
        super::util::setup_skybox(resources, "textures/skybox.basis");
        super::util::set_ambient_light(resources, glam::Vec3::new(0.005, 0.005, 0.005));

        let container_1 =
            SpawnableMesh::blocking_load_from_path(resources, "blender/storage_container1.glb");
        let container_2 =
            SpawnableMesh::blocking_load_from_path(resources, "blender/storage_container2.glb");
        let blue_icosphere = SpawnableMesh::blocking_load_from_uuid(
            resources,
            "1af1ca58-49a6-4ef7-ac8f-20be3b75b48b".into(),
        );
        //
        // Add some meshes
        //
        {
            let example_meshes = vec![container_1, container_2, blue_icosphere];

            for i in 0..1 {
                let position = Vec3::new(((i / 9) * 3) as f32, ((i % 9) * 3) as f32, 0.0);
                let example_mesh = &example_meshes[i % example_meshes.len()];

                let rand_scale = 0.2;
                let offset = rand_scale - 1.;
                let transform_component = TransformComponent {
                    translation: position + Vec3::new(0., 0., offset),
                    scale: Vec3::new(rand_scale, rand_scale, rand_scale),
                    ..Default::default()
                };

                example_mesh.spawn(resources, world, transform_component);
            }
        }

        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        let main_view_frustum = visibility_resource.register_view_frustum();

        TaaTestScene { main_view_frustum }
    }
}

impl super::TestScene for TaaTestScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        super::util::add_light_debug_draw(&resources, &world);

        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            let render_options = resources.get::<RenderOptions>().unwrap();

            update_main_view_3d(
                &*time_state,
                &*render_options,
                &mut self.main_view_frustum,
                &mut *viewports_resource,
            );
        }

        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut query = <Write<DirectionalLightComponent>>::query();
            for mut light in query.iter_mut(world) {
                const LIGHT_XY_DISTANCE: f32 = 50.0;
                const LIGHT_Z: f32 = 50.0;
                const LIGHT_ROTATE_SPEED: f32 = 0.0;
                const LIGHT_LOOP_OFFSET: f32 = 2.0;
                let loop_time = time_state.total_time().as_secs_f32();
                let light_from = glam::Vec3::new(
                    LIGHT_XY_DISTANCE
                        * f32::cos(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_XY_DISTANCE
                        * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_Z,
                    //LIGHT_Z// * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET).abs(),
                    //0.2
                    //2.0
                );
                let light_to = glam::Vec3::default();

                light.direction = (light_to - light_from).normalize();
            }
        }

        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut query = <(Write<TransformComponent>, Read<PointLightComponent>)>::query();
            for (transform, _light) in query.iter_mut(world) {
                const LIGHT_XY_DISTANCE: f32 = 6.0;
                const LIGHT_Z: f32 = 3.5;
                const LIGHT_ROTATE_SPEED: f32 = 0.5;
                const LIGHT_LOOP_OFFSET: f32 = 2.0;
                let loop_time = time_state.total_time().as_secs_f32();
                let light_from = glam::Vec3::new(
                    LIGHT_XY_DISTANCE
                        * f32::cos(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_XY_DISTANCE
                        * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_Z,
                    //LIGHT_Z// * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET).abs(),
                    //0.2
                    //2.0
                );
                transform.translation = light_from;
            }
        }
    }
}

#[profiling::function]
fn update_main_view_3d(
    time_state: &TimeState,
    render_options: &RenderOptions,
    main_view_frustum: &mut ViewFrustumArc,
    viewports_resource: &mut ViewportsResource,
) {
    let (phase_mask_builder, feature_mask_builder, feature_flag_mask_builder) =
        super::util::default_main_view_masks(render_options);

    let aspect_ratio = viewports_resource.main_window_size.width as f32
        / viewports_resource.main_window_size.height as f32;

    const CAMERA_XY_DISTANCE: f32 = 5.0;
    const CAMERA_Z: f32 = 1.0;
    const CAMERA_ROTATE_SPEED: f32 = -0.40;
    const CAMERA_LOOP_OFFSET: f32 = -0.3;
    let loop_time = time_state.total_time().as_secs_f32();

    // Slide across X
    //let eye = glam::Vec3::new(
    //    CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
    //    -10.0,
    //    //CAMERA_XY_DISTANCE * f32::sin(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
    //    CAMERA_Z,
    //);
    //let look_at = glam::Vec3::new(eye.x, 0.0, 0.0);

    // Slide across XY
    let eye = glam::Vec3::new(
        CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        -10.0,
        //CAMERA_XY_DISTANCE * f32::sin(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
    );
    let look_at = glam::Vec3::new(eye.x, 0.0, eye.z);

    let up = glam::Vec3::new(0.0, 0.0, 1.0);
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

    std::thread::sleep(Duration::from_secs_f32(0.050));

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
