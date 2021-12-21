use distill::loader::handle::{AssetHandle, Handle};
use legion::{Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::rafx_visibility::VisibleBounds;
use rafx::visibility::{CullModel, ObjectId, VisibilityRegion};
use rafx_plugins::components::{
    DirectionalLightComponent, MeshComponent, PointLightComponent, SpotLightComponent,
    TransformComponent, VisibilityComponent,
};

#[cfg(feature = "basic-pipeline")]
use rafx_plugins::assets::mesh_basic::prefab_asset::PrefabBasicAssetDataObjectLightKind as PrefabAssetDataObjectLightKind;
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::assets::mesh_basic::{
    MeshBasicAsset as MeshAsset, PrefabBasicAsset as PrefabAsset,
};
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::features::mesh_basic::{
    MeshBasicRenderObject as MeshRenderObject, MeshBasicRenderObjectSet as MeshRenderObjectSet,
};

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::assets::mesh_adv::prefab_asset::PrefabBasicAssetDataObjectLightKind as PrefabAssetDataObjectLightKind;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::assets::mesh_adv::{
    MeshBasicAsset as MeshAsset, PrefabBasicAsset as PrefabAsset,
};
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::features::mesh_adv::{
    MeshBasicRenderObject as MeshRenderObject, MeshBasicRenderObjectSet as MeshRenderObjectSet,
};

pub struct SpawnablePrefab {
    prefab_asset_handle: Handle<PrefabAsset>,
}

impl SpawnablePrefab {
    pub fn blocking_load_from_path<T: Into<String>>(
        resources: &Resources,
        path: T,
    ) -> Self {
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        let prefab_asset_handle: Handle<PrefabAsset> = asset_resource.load_asset_path(path);
        asset_manager
            .wait_for_asset_to_load(
                &prefab_asset_handle,
                &mut asset_resource,
                "spawnable prefab",
            )
            .unwrap();

        SpawnablePrefab {
            prefab_asset_handle,
        }
    }

    pub fn spawn_prefab(
        &self,
        world: &mut World,
        resources: &Resources,
    ) {
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
        let mut mesh_render_objects = resources.get_mut::<MeshRenderObjectSet>().unwrap();
        let visibility_region = resources.get::<VisibilityRegion>().unwrap();

        let prefab_asset = asset_resource
            .asset(&self.prefab_asset_handle)
            .unwrap()
            .clone();

        fn load_visible_bounds(
            asset_manager: &mut AssetManager,
            asset_resource: &mut AssetResource,
            asset_handle: &Handle<MeshAsset>,
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

        for object in &prefab_asset.inner.objects {
            log::debug!("create object {:?}", object);
            if let Some(model) = &object.model {
                let model_asset_handle = asset_resource.asset(&model.model);
                if model_asset_handle.is_none() {
                    continue;
                }
                let model_asset = model_asset_handle.unwrap();
                let mesh_asset = model_asset.inner.lods[0].mesh.clone();

                let render_object = mesh_render_objects.register_render_object(MeshRenderObject {
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
                    PrefabAssetDataObjectLightKind::Point => {
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
                    PrefabAssetDataObjectLightKind::Spot => {
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
                    PrefabAssetDataObjectLightKind::Directional => {
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
    }
}
