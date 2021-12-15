use crate::input::{InputResource, InputState, KeyboardKey};
use crate::time::TimeState;
use crate::RenderOptions;
use distill::loader::handle::{AssetHandle, Handle};
use legion::{Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection, VisibleBounds};
use rafx::render_features::{
    RenderFeatureFlagMaskBuilder, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder,
    RenderViewDepthRange,
};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{CullModel, ObjectId, ViewFrustumArc, VisibilityRegion};
use rafx_plugins::assets::mesh_basic::prefab_asset::PrefabBasicAssetDataObjectLightKind;
use rafx_plugins::assets::mesh_basic::{MeshBasicAsset, PrefabBasicAsset};
use rafx_plugins::components::{
    DirectionalLightComponent, MeshComponent, PointLightComponent, TransformComponent,
};
use rafx_plugins::components::{SpotLightComponent, VisibilityComponent};
use rafx_plugins::features::debug3d::Debug3DRenderFeature;
use rafx_plugins::features::mesh_basic::{
    MeshBasicNoShadowsRenderFeatureFlag, MeshBasicRenderFeature, MeshBasicRenderObject,
    MeshBasicRenderObjectSet, MeshBasicRenderOptions, MeshBasicUnlitRenderFeatureFlag,
    MeshBasicUntexturedRenderFeatureFlag, MeshBasicWireframeRenderFeatureFlag,
};
use rafx_plugins::features::skybox::SkyboxRenderFeature;
use rafx_plugins::features::sprite::SpriteRenderFeature;
use rafx_plugins::features::text::TextRenderFeature;
use rafx_plugins::features::tile_layer::TileLayerRenderFeature;
use rafx_plugins::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase,
    WireframeRenderPhase,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct FlyCamera {
    position: glam::Vec3,
    look_dir: glam::Vec3,
    right_dir: glam::Vec3,
    pitch: f32,
    yaw: f32,
    lock_view: bool,
}

impl FlyCamera {
    fn update(
        &mut self,
        input_state: &InputState,
        time_state: &TimeState,
    ) {
        if input_state.is_key_just_down(KeyboardKey::F) {
            self.lock_view = !self.lock_view;
        }

        const NORMAL_MOVE_SPEED: f32 = 10.0;
        const FAST_MOVE_SPEED: f32 = 30.0;
        const LOOK_SPEED: f32 = 0.1;

        let move_speed = if input_state.is_key_down(KeyboardKey::LShift)
            || input_state.is_key_down(KeyboardKey::RShift)
        {
            FAST_MOVE_SPEED
        } else {
            NORMAL_MOVE_SPEED
        };

        //+x = forward
        //+y = right
        let mut velocity = glam::Vec3::default();
        if input_state.is_key_down(KeyboardKey::W) {
            velocity.x += move_speed;
        }

        if input_state.is_key_down(KeyboardKey::S) {
            velocity.x -= move_speed;
        }

        if input_state.is_key_down(KeyboardKey::A) {
            velocity.y -= move_speed;
        }

        if input_state.is_key_down(KeyboardKey::D) {
            velocity.y += move_speed;
        }

        const TWO_PI: f32 = 2.0 * std::f32::consts::PI;

        let yaw_dt = input_state.mouse_motion().x as f32 * LOOK_SPEED * -1.0;
        let pitch_dt = input_state.mouse_motion().y as f32 * LOOK_SPEED * -1.0;

        if !self.lock_view {
            self.yaw += yaw_dt * time_state.previous_update_dt();
            while self.yaw > std::f32::consts::PI {
                self.yaw -= TWO_PI;
            }

            while self.yaw < -std::f32::consts::PI {
                self.yaw += TWO_PI
            }

            self.pitch += pitch_dt * time_state.previous_update_dt();
            self.pitch = self.pitch.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.01,
                std::f32::consts::FRAC_PI_2 - 0.01,
            );
        }

        let z = self.pitch.sin();
        let z_inv = 1.0 - z.abs();
        let x = self.yaw.cos() * z_inv;
        let y = self.yaw.sin() * z_inv;
        let look_dir = glam::Vec3::new(x, y, z).normalize();
        let up_dir = glam::Vec3::Z;
        let right_dir = look_dir.cross(up_dir).normalize();

        if !self.lock_view {
            self.position += velocity.x * self.look_dir * time_state.previous_update_dt();
            self.position += velocity.y * self.right_dir * time_state.previous_update_dt();
        }
        self.look_dir = look_dir;
        self.right_dir = right_dir;

        // println!(
        //     "pos: {} pitch: {} yaw: {}",
        //     self.position, self.pitch, self.yaw
        // );
    }
}

#[derive(Serialize, Deserialize)]
struct PathData {
    position: [f32; 3],
    rotation: [f32; 4],
}

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

        let bistro_prefab: Handle<PrefabBasicAsset> =
            asset_resource.load_asset_path("bistro/bistro_merged/Scene.blender_prefab");
        asset_manager
            .wait_for_asset_to_load(&bistro_prefab, &mut asset_resource, "bistro scene")
            .unwrap();
        let bistro_prefab_asset = asset_resource.asset(&bistro_prefab).unwrap().clone();

        fn load_visible_bounds(
            asset_manager: &mut AssetManager,
            asset_resource: &mut AssetResource,
            asset_handle: &Handle<MeshBasicAsset>,
            asset_name: &str,
        ) -> Option<VisibleBounds> {
            asset_manager
                .wait_for_asset_to_load(asset_handle, asset_resource, asset_name)
                .unwrap();

            asset_manager
                .committed_asset(asset_handle)
                .map(|x| x.inner.asset_data.visible_bounds)
        }

        let mut point_light_count = 0;
        let mut spot_light_count = 0;
        let mut directional_light_count = 0;

        for object in &bistro_prefab_asset.inner.objects {
            log::debug!("create object {:?}", object);
            if let Some(model) = &object.model {
                let model_asset_handle = asset_resource.asset(&model.model);
                if model_asset_handle.is_none() {
                    continue;
                }
                let model_asset = model_asset_handle.unwrap();
                let mesh_asset = model_asset.inner.lods[0].mesh.clone();

                let render_object =
                    mesh_render_objects.register_render_object(MeshBasicRenderObject {
                        mesh: mesh_asset.clone(),
                    });

                let transform_component = TransformComponent {
                    translation: object.transform.position,
                    rotation: object.transform.rotation,
                    scale: object.transform.scale,
                    ..Default::default()
                };

                let mesh_component = MeshComponent {
                    render_object_handle: render_object.clone(),
                };

                let entity = world.push((transform_component.clone(), mesh_component));
                let mut entry = world.entry(entity).unwrap();

                let visible_bounds = load_visible_bounds(
                    &mut *asset_manager,
                    &mut *asset_resource,
                    &mesh_asset,
                    &format!("visible bounds for {:?}", model.model),
                );

                if let Some(visible_bounds) = visible_bounds {
                    entry.add_component(VisibilityComponent {
                        visibility_object_handle: {
                            let handle = visibility_region.register_static_object(
                                ObjectId::from(entity),
                                CullModel::VisibleBounds(visible_bounds),
                            );
                            handle.set_transform(
                                transform_component.translation,
                                transform_component.rotation,
                                transform_component.scale,
                            );
                            handle.add_render_object(&render_object);
                            handle
                        },
                    });
                } else {
                    let load_info = asset_resource
                        .loader()
                        .get_load_info(model.model.load_handle());
                    log::warn!(
                        "Did not find committed asset for {:?} load_info: {:?}",
                        model.model,
                        load_info
                    );
                }
            }

            if let Some(light) = &object.light {
                match light.kind {
                    PrefabBasicAssetDataObjectLightKind::Point => {
                        if point_light_count < 15 {
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
                                object.transform.position,
                                PointLightComponent {
                                    color: light.color.extend(1.0),
                                    intensity: light.intensity * 0.15,
                                    range: 25.0,
                                    view_frustums,
                                },
                            );
                            point_light_count += 1;
                        }
                    }
                    PrefabBasicAssetDataObjectLightKind::Spot => {
                        if spot_light_count < 15 {
                            let view_frustum = visibility_region.register_view_frustum();
                            super::add_spot_light(
                                resources,
                                world,
                                //glam::Vec3::new(-3.0, 3.0, 2.0),
                                object.transform.position,
                                SpotLightComponent {
                                    color: light.color.extend(1.0),
                                    intensity: light.intensity * 0.15,
                                    range: 25.0,
                                    view_frustum,
                                    spotlight_half_angle: light.spot.as_ref().unwrap().outer_angle,
                                    direction: object.transform.rotation * -glam::Vec3::Z,
                                },
                            );
                            spot_light_count += 1;
                        }
                    }
                    PrefabBasicAssetDataObjectLightKind::Directional => {
                        if directional_light_count < 15 {
                            let view_frustum = visibility_region.register_view_frustum();
                            super::add_directional_light(
                                resources,
                                world,
                                DirectionalLightComponent {
                                    color: light.color.extend(1.0),
                                    intensity: light.intensity * 0.15,
                                    view_frustum,
                                    direction: object.transform.rotation * -glam::Vec3::Z,
                                },
                            );
                            directional_light_count += 1;
                        }
                    }
                }
            }
        }

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
