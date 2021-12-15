// NOTE(dvd): Inspired by Bevy `spawner` example (MIT licensed) https://github.com/bevyengine/bevy/blob/20673dbe0e935d9b7b4fdc8947830bfcff6bc071/examples/3d/spawner.rs

use crate::phases::{
    OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase, WireframeRenderPhase,
};
use crate::time::TimeState;
use crate::RenderOptions;
use distill::loader::handle::Handle;
use legion::IntoQuery;
use legion::{Read, Resources, World, Write};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::{
    RenderFeatureFlagMaskBuilder, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder,
    RenderViewDepthRange,
};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{CullModel, ObjectId, ViewFrustumArc, VisibilityRegion};
use rafx_plugins::assets::mesh_basic::MeshBasicAsset;
use rafx_plugins::components::{
    MeshComponent, PointLightComponent, TransformComponent, VisibilityComponent,
};
use rafx_plugins::features::debug3d::Debug3DRenderFeature;
#[cfg(feature = "egui")]
use rafx_plugins::features::egui::EguiRenderFeature;
use rafx_plugins::features::mesh_basic::{
    MeshBasicNoShadowsRenderFeatureFlag, MeshBasicRenderFeature, MeshBasicRenderObject,
    MeshBasicRenderObjectSet, MeshBasicUnlitRenderFeatureFlag,
    MeshBasicUntexturedRenderFeatureFlag, MeshBasicWireframeRenderFeatureFlag,
};
use rafx_plugins::features::skybox::SkyboxRenderFeature;
use rafx_plugins::features::text::TextRenderFeature;
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
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();
        render_options.enable_textures = false;
        render_options.show_shadows = false;

        let mut mesh_render_objects = resources.get_mut::<MeshBasicRenderObjectSet>().unwrap();

        let visibility_region = resources.get::<VisibilityRegion>().unwrap();

        let container_1_asset = asset_resource.load_asset_path("blender/storage_container1.glb");
        let cube_render_object =
            mesh_render_objects.register_render_object(MeshBasicRenderObject {
                mesh: container_1_asset.clone(),
            });

        let mut load_visible_bounds = |asset_handle: &Handle<MeshBasicAsset>| {
            asset_manager
                .wait_for_asset_to_load(asset_handle, &mut asset_resource, "")
                .unwrap();

            asset_manager
                .committed_asset(asset_handle)
                .unwrap()
                .inner
                .asset_data
                .visible_bounds
        };

        let visible_bounds = load_visible_bounds(&container_1_asset);

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

            let mesh_component = MeshComponent {
                render_object_handle: cube_render_object.clone(),
            };

            let entity = world.push((transform_component.clone(), mesh_component));
            let mut entry = world.entry(entity).unwrap();
            entry.add_component(VisibilityComponent {
                visibility_object_handle: {
                    let handle = visibility_region.register_dynamic_object(
                        ObjectId::from(entity),
                        CullModel::VisibleBounds(visible_bounds.clone()),
                    );
                    handle.set_transform(
                        transform_component.translation,
                        transform_component.rotation,
                        transform_component.scale,
                    );
                    handle.add_render_object(&cube_render_object);
                    handle
                },
            });
        }

        //
        // POINT LIGHT
        //
        let view_frustums = [
            visibility_region.register_view_frustum(),
            visibility_region.register_view_frustum(),
            visibility_region.register_view_frustum(),
            visibility_region.register_view_frustum(),
            visibility_region.register_view_frustum(),
            visibility_region.register_view_frustum(),
        ];

        super::util::add_point_light(
            resources,
            world,
            glam::Vec3::new(-4., -4., 10.),
            PointLightComponent {
                color: [1.0, 1.0, 1.0, 1.0].into(),
                intensity: 200.0,
                range: 20.0,
                view_frustums,
            },
        );

        let main_view_frustum = visibility_region.register_view_frustum();

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
    let phase_mask_builder = RenderPhaseMaskBuilder::default()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<WireframeRenderPhase>()
        .add_render_phase::<UiRenderPhase>();

    let mut feature_mask_builder =
        RenderFeatureMaskBuilder::default().add_render_feature::<MeshBasicRenderFeature>();

    #[cfg(feature = "egui")]
    {
        feature_mask_builder = feature_mask_builder.add_render_feature::<EguiRenderFeature>();
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
