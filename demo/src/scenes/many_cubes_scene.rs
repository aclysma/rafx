// NOTE(dvd): Inspired by Bevy `spawner` example (MIT licensed) https://github.com/bevyengine/bevy/blob/20673dbe0e935d9b7b4fdc8947830bfcff6bc071/examples/3d/spawner.rs
use crate::scenes::util::SpawnableMesh;
use crate::time::TimeState;
use crate::RenderOptions;
use legion::IntoQuery;
use legion::{Read, Resources, World, Write};
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::RenderViewDepthRange;
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{ViewFrustumArc, VisibilityResource};
use rafx_plugins::components::{MeshComponent, TransformComponent, VisibilityComponent};
use rand::{thread_rng, Rng};

const NUM_CUBES: usize = 10000;

pub(super) struct ManyCubesScene {
    main_view_frustum: ViewFrustumArc,
}

impl ManyCubesScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();
        render_options.enable_textures = false;
        render_options.show_shadows = false;

        super::util::setup_skybox(resources, "textures/skybox.basis");

        let spawnable_mesh =
            SpawnableMesh::blocking_load_from_path(resources, "blender/storage_container1.glb");

        //
        // Add some meshes
        //

        let mut rng = thread_rng();
        for _ in 0..NUM_CUBES {
            let transform_component = TransformComponent {
                translation: glam::Vec3::new(
                    rng.gen_range(-50.0..50.0),
                    rng.gen_range(-50.0..50.0),
                    0.0,
                ),
                scale: glam::Vec3::new(0.5, 0.5, 0.5),
                ..Default::default()
            };

            spawnable_mesh.spawn(resources, world, transform_component);
        }

        super::util::add_point_light(
            resources,
            world,
            [-4., -4., 10.].into(),
            [1.0, 1.0, 1.0, 1.0].into(),
            200.0,
            true,
        );

        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        let main_view_frustum = visibility_resource.register_view_frustum();

        ManyCubesScene { main_view_frustum }
    }
}

impl super::TestScene for ManyCubesScene {
    #[profiling::function]
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        {
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            let render_options = resources.get::<RenderOptions>().unwrap();

            update_main_view_3d(
                &*render_options,
                &mut self.main_view_frustum,
                &mut *viewports_resource,
            );
        }

        {
            // Move the cubes.

            let time_state = resources.get::<TimeState>().unwrap();
            let mut query = <(
                Write<TransformComponent>,
                Read<VisibilityComponent>,
                Read<MeshComponent>,
            )>::query();

            for (transform, visibility, _mesh) in query.iter_mut(world) {
                transform.translation +=
                    glam::Vec3::new(-1.0, 0.0, 0.0) * time_state.previous_update_dt();

                visibility.visibility_object_handle.set_transform(
                    transform.translation,
                    transform.rotation,
                    transform.scale,
                );
            }
        }
    }
}

#[profiling::function]
fn update_main_view_3d(
    render_options: &RenderOptions,
    main_view_frustum: &mut ViewFrustumArc,
    viewports_resource: &mut ViewportsResource,
) {
    let (phase_mask_builder, feature_mask_builder, feature_flag_mask_builder) =
        super::util::default_main_view_masks(render_options);

    const CAMERA_Z: f32 = 150.0;
    let eye = glam::Vec3::new(0., 15., CAMERA_Z);

    let aspect_ratio = viewports_resource.main_window_size.width as f32
        / viewports_resource.main_window_size.height as f32;

    let look_at = glam::Vec3::ZERO;
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
