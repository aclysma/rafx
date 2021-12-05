use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase,
    WireframeRenderPhase,
};
use crate::time::TimeState;
use crate::RenderOptions;
use distill::loader::handle::Handle;
use glam::Vec3;
use legion::{IntoQuery, Read};
use legion::{Resources, World, Write};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, ImageAsset};
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::{
    RenderFeatureFlagMaskBuilder, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder,
    RenderViewDepthRange,
};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{CullModel, ObjectId, ViewFrustumArc, VisibilityRegion};
use rafx_plugins::assets::mesh::MeshAsset;
use rafx_plugins::components::{
    DirectionalLightComponent, SpotLightComponent, VisibilityComponent,
};
use rafx_plugins::components::{MeshComponent, PointLightComponent, TransformComponent};
use rafx_plugins::features::debug3d::Debug3DRenderFeature;
use rafx_plugins::features::mesh_basic::{
    MeshNoShadowsRenderFeatureFlag, MeshRenderFeature, MeshRenderObject, MeshRenderObjectSet,
    MeshRenderOptions, MeshUnlitRenderFeatureFlag, MeshUntexturedRenderFeatureFlag,
    MeshWireframeRenderFeatureFlag,
};
use rafx_plugins::features::skybox::{SkyboxRenderFeature, SkyboxResource};
use rafx_plugins::features::sprite::SpriteRenderFeature;
use rafx_plugins::features::text::TextRenderFeature;
use rafx_plugins::features::tile_layer::TileLayerRenderFeature;
use rand::{thread_rng, Rng};

pub(super) struct ShadowsScene {
    main_view_frustum: ViewFrustumArc,
}

impl ShadowsScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();

        let mut mesh_render_options = resources.get_mut::<MeshRenderOptions>().unwrap();
        mesh_render_options.ambient_light = glam::Vec3::new(0.005, 0.005, 0.005);

        let mut mesh_render_objects = resources.get_mut::<MeshRenderObjectSet>().unwrap();

        let visibility_region = resources.get::<VisibilityRegion>().unwrap();

        let floor_mesh_asset =
            asset_resource.load_asset_path::<MeshAsset, _>("blender/cement_floor.glb");
        let container_1_asset = asset_resource.load_asset_path("blender/storage_container1.glb");
        let container_2_asset = asset_resource.load_asset_path("blender/storage_container2.glb");
        let blue_icosphere_asset =
            asset_resource.load_asset::<MeshAsset>("d5aed900-1e31-4f47-94ba-e356b0b0b8b0".into());

        let skybox_texture =
            asset_resource.load_asset_path::<ImageAsset, _>("textures/skybox.basis");

        *resources
            .get_mut::<SkyboxResource>()
            .unwrap()
            .skybox_texture_mut() = Some(skybox_texture);

        let mut load_visible_bounds = |asset_handle: &Handle<MeshAsset>| {
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

        //
        // Add a floor
        //
        {
            let position = Vec3::new(0.0, 0.0, -1.0);

            let floor_mesh_render_object =
                mesh_render_objects.register_render_object(MeshRenderObject {
                    mesh: floor_mesh_asset.clone(),
                });

            let transform_component = TransformComponent {
                translation: position,
                ..Default::default()
            };

            let mesh_component = MeshComponent {
                render_object_handle: floor_mesh_render_object.clone(),
            };

            let entity = world.push((transform_component.clone(), mesh_component));
            let mut entry = world.entry(entity).unwrap();
            entry.add_component(VisibilityComponent {
                visibility_object_handle: {
                    let handle = visibility_region.register_static_object(
                        ObjectId::from(entity),
                        CullModel::VisibleBounds(load_visible_bounds(&floor_mesh_asset)),
                    );
                    handle.set_transform(
                        transform_component.translation,
                        transform_component.rotation,
                        transform_component.scale,
                    );
                    handle.add_render_object(&floor_mesh_render_object);
                    handle
                },
            });
        }

        //
        // Add some meshes
        //
        {
            let example_meshes = {
                let mut meshes = Vec::default();

                // container1
                meshes.push(
                    mesh_render_objects.register_render_object(MeshRenderObject {
                        mesh: container_1_asset,
                    }),
                );

                // container2
                meshes.push(
                    mesh_render_objects.register_render_object(MeshRenderObject {
                        mesh: container_2_asset,
                    }),
                );

                // blue icosphere - load by UUID since it's one of several meshes in the file
                meshes.push(
                    mesh_render_objects.register_render_object(MeshRenderObject {
                        mesh: blue_icosphere_asset,
                    }),
                );

                meshes
            };

            let mesh_render_objects = mesh_render_objects.read();
            let mut rng = thread_rng();
            for i in 0..250 {
                let position = Vec3::new(((i / 9) * 3) as f32, ((i % 9) * 3) as f32, 0.0);
                let mesh_render_object = example_meshes[i % example_meshes.len()].clone();
                let asset_handle = &mesh_render_objects.get(&mesh_render_object).mesh;

                let rand_scale = rng.gen_range(0.8..1.2);
                let offset = rand_scale - 1.;
                let transform_component = TransformComponent {
                    translation: position + Vec3::new(0., 0., offset),
                    scale: Vec3::new(rand_scale, rand_scale, rand_scale),
                    ..Default::default()
                };

                let mesh_component = MeshComponent {
                    render_object_handle: mesh_render_object.clone(),
                };

                let entity = world.push((transform_component.clone(), mesh_component));
                let mut entry = world.entry(entity).unwrap();
                entry.add_component(VisibilityComponent {
                    visibility_object_handle: {
                        let handle = visibility_region.register_dynamic_object(
                            ObjectId::from(entity),
                            CullModel::VisibleBounds(load_visible_bounds(asset_handle)),
                        );
                        handle.set_transform(
                            transform_component.translation,
                            transform_component.rotation,
                            transform_component.scale,
                        );
                        handle.add_render_object(&mesh_render_object);
                        handle
                    },
                });
            }
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
        super::add_point_light(
            resources,
            world,
            //glam::Vec3::new(-3.0, 3.0, 2.0),
            glam::Vec3::new(5.0, 5.0, 2.0),
            PointLightComponent {
                color: [0.0, 1.0, 0.0, 1.0].into(),
                intensity: 75.0,
                range: 25.0,
                view_frustums,
            },
        );

        //
        // DIRECTIONAL LIGHT
        //
        let light_from = glam::Vec3::new(-5.0, 5.0, 5.0);
        let light_to = glam::Vec3::ZERO;
        let light_direction = (light_to - light_from).normalize();
        super::add_directional_light(
            resources,
            world,
            DirectionalLightComponent {
                direction: light_direction,
                intensity: 1.0,
                color: [0.0, 0.0, 1.0, 1.0].into(),
                view_frustum: visibility_region.register_view_frustum(),
            },
        );

        //
        // SPOT LIGHT
        //
        let light_from = glam::Vec3::new(-3.0, -3.0, 5.0);
        let light_to = glam::Vec3::ZERO;
        let light_direction = (light_to - light_from).normalize();
        super::add_spot_light(
            resources,
            world,
            light_from,
            SpotLightComponent {
                direction: light_direction,
                spotlight_half_angle: 40.0 * (std::f32::consts::PI / 180.0),
                range: 12.0,
                color: [1.0, 0.0, 0.0, 1.0].into(),
                intensity: 150.0,
                view_frustum: visibility_region.register_view_frustum(),
            },
        );

        let main_view_frustum = visibility_region.register_view_frustum();

        ShadowsScene { main_view_frustum }
    }
}

impl super::TestScene for ShadowsScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        super::add_light_debug_draw(&resources, &world);

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
    let phase_mask_builder = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DepthPrepassRenderPhase>()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<WireframeRenderPhase>()
        .add_render_phase::<UiRenderPhase>();

    let mut feature_mask_builder = RenderFeatureMaskBuilder::default()
        .add_render_feature::<MeshRenderFeature>()
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
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshWireframeRenderFeatureFlag>();
    }

    if !render_options.enable_lighting {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshUnlitRenderFeatureFlag>();
    }

    if !render_options.enable_textures {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshUntexturedRenderFeatureFlag>();
    }

    if !render_options.show_shadows {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshNoShadowsRenderFeatureFlag>();
    }

    const CAMERA_XY_DISTANCE: f32 = 12.0;
    const CAMERA_Z: f32 = 6.0;
    const CAMERA_ROTATE_SPEED: f32 = -0.10;
    const CAMERA_LOOP_OFFSET: f32 = -0.3;
    let loop_time = time_state.total_time().as_secs_f32();
    let eye = glam::Vec3::new(
        CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        CAMERA_XY_DISTANCE * f32::sin(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        CAMERA_Z,
    );

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
