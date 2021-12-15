use crate::input::{InputState, KeyboardKey};
use crate::time::TimeState;
use distill::loader::handle::{AssetHandle, Handle};
use legion::IntoQuery;
use legion::{Read, Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::rafx_visibility::VisibleBounds;
use rafx::visibility::{CullModel, ObjectId, VisibilityRegion};
use rafx_plugins::assets::mesh_basic::prefab_asset::PrefabBasicAssetDataObjectLightKind;
use rafx_plugins::assets::mesh_basic::{MeshBasicAsset, PrefabBasicAsset};
use rafx_plugins::components::{
    DirectionalLightComponent, MeshComponent, PointLightComponent, SpotLightComponent,
    TransformComponent, VisibilityComponent,
};
use rafx_plugins::features::debug3d::Debug3DResource;
use rafx_plugins::features::mesh_basic::{MeshBasicRenderObject, MeshBasicRenderObjectSet};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct PathData {
    pub(super) position: [f32; 3],
    pub(super) rotation: [f32; 4],
}

//
// Camera by default points along +X axis, +Z up
//
#[derive(Default)]
pub(super) struct FlyCamera {
    pub(super) position: glam::Vec3,
    pub(super) look_dir: glam::Vec3,
    pub(super) right_dir: glam::Vec3,
    pub(super) up_dir: glam::Vec3,
    pub(super) pitch: f32,
    pub(super) yaw: f32,
    pub(super) lock_view: bool,
}

impl FlyCamera {
    pub(super) fn update(
        &mut self,
        input_state: &InputState,
        time_state: &TimeState,
    ) {
        // Allow locking camera position/rotation
        if input_state.is_key_just_down(KeyboardKey::F) {
            self.lock_view = !self.lock_view;
        }

        const NORMAL_MOVE_SPEED: f32 = 10.0;
        const FAST_MOVE_SPEED: f32 = 30.0;
        const LOOK_SPEED: f32 = 0.1;
        const TWO_PI: f32 = 2.0 * std::f32::consts::PI;

        // Use mouse motion to rotate the camera
        if !self.lock_view {
            let yaw_dt = input_state.mouse_motion().x as f32 * LOOK_SPEED * -1.0;
            let pitch_dt = input_state.mouse_motion().y as f32 * LOOK_SPEED * -1.0;

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
            self.pitch += pitch_dt * time_state.previous_update_dt();
            self.pitch = self.pitch.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.01,
                std::f32::consts::FRAC_PI_2 - 0.01,
            );
        }

        // Recalculate frenet frame, do this even if the camera is locked so that if the pitch/yaw
        // is set manually, the directions refresh
        // Z-Up
        let z = self.pitch.sin();
        let z_inv = 1.0 - z.abs();
        let x = self.yaw.cos() * z_inv;
        let y = self.yaw.sin() * z_inv;
        let look_dir = glam::Vec3::new(x, y, z).normalize();
        let up_dir = glam::Vec3::Z;
        let right_dir = look_dir.cross(up_dir).normalize();

        self.look_dir = look_dir;
        self.right_dir = right_dir;
        self.up_dir = up_dir;

        // Use wasd to move the camera
        if !self.lock_view {
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

            self.position += velocity.x * self.look_dir * time_state.previous_update_dt();
            self.position += velocity.y * self.right_dir * time_state.previous_update_dt();
        }

        //println!("move speed {:?}", velocity);
        //println!("mouse delta {:?}", input_state.mouse_position_delta())
        //println!("pitch: {:?} yaw: {:?} velocity: {:?}", pitch_dt, yaw_dt, velocity);
        //println!("pitch: {:?} yaw: {:?} velocity: {:?}", self.pitch, self.yaw, self.position);
        //println!("yaw: {} pitch: {} look: {:?} up: {:?} right: {:?}", self.yaw.to_degrees(), self.pitch.to_degrees(), look_dir, up_dir, right_dir);
        // println!(
        //     "pos: {} pitch: {} yaw: {}",
        //     self.position, self.pitch, self.yaw
        // );
    }
}

pub(super) fn spawn_prefab(
    world: &mut World,
    resources: &Resources,
    asset_manager: &mut AssetManager,
    asset_resource: &mut AssetResource,
    mesh_render_objects: &mut MeshBasicRenderObjectSet,
    visibility_region: &VisibilityRegion,
    bistro_prefab_asset: &PrefabBasicAsset,
) {
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

            let render_object = mesh_render_objects.register_render_object(MeshBasicRenderObject {
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
                        add_point_light(
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
                        add_spot_light(
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
                        add_directional_light(
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
}

pub(super) fn add_light_debug_draw(
    resources: &Resources,
    world: &World,
) {
    let mut debug_draw = resources.get_mut::<Debug3DResource>().unwrap();

    let mut query = <Read<DirectionalLightComponent>>::query();
    for light in query.iter(world) {
        let light_from = light.direction * -10.0;
        let light_to = glam::Vec3::ZERO;

        debug_draw.add_line(light_from, light_to, light.color);
    }

    let mut query = <(Read<TransformComponent>, Read<PointLightComponent>)>::query();
    for (transform, light) in query.iter(world) {
        debug_draw.add_sphere(transform.translation, 0.1, light.color, 12);
        debug_draw.add_sphere(transform.translation, light.range, light.color, 12);
    }

    let mut query = <(Read<TransformComponent>, Read<SpotLightComponent>)>::query();
    for (transform, light) in query.iter(world) {
        let light_from = transform.translation;
        let light_to = transform.translation + light.direction;
        let light_direction = (light_to - light_from).normalize();

        debug_draw.add_cone(
            light_from,
            light_from + (light.range * light_direction),
            light.range * light.spotlight_half_angle.tan(),
            light.color,
            10,
        );
    }
}

pub(super) fn add_directional_light(
    _resources: &Resources,
    world: &mut World,
    light_component: DirectionalLightComponent,
) {
    world.extend(vec![(light_component,)]);
}

pub(super) fn add_spot_light(
    _resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: SpotLightComponent,
) {
    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    world.extend(vec![(position_component, light_component)]);
}

pub(super) fn add_point_light(
    _resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: PointLightComponent,
) {
    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    world.extend(vec![(position_component, light_component)]);
}
