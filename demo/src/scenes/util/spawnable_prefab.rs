use hydrate_base::handle::ArtifactHandle;
use hydrate_base::Handle;
use legion::{Resources, World};
use rafx::assets::AssetManager;
use rafx::assets::AssetResource;
use rafx::rafx_visibility::VisibleBounds;
use rafx::renderer::Renderer;
use rafx::visibility::{CullModel, ObjectId, VisibilityResource};
use rafx_plugins::components::{MeshComponent, TransformComponent, VisibilityComponent};

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::assets::mesh_adv::prefab_asset::PrefabAdvAssetDataObjectLightKind as PrefabAssetDataObjectLightKind;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::assets::mesh_adv::{MeshAdvAsset as MeshAsset, PrefabAdvAsset as PrefabAsset};
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::features::mesh_adv::{
    MeshAdvRenderObject as MeshRenderObject, MeshAdvRenderObjectSet as MeshRenderObjectSet,
};

pub struct SpawnablePrefab {
    prefab_asset_handle: Handle<PrefabAsset>,
}

impl SpawnablePrefab {
    pub fn blocking_load_from_symbol_name(
        resources: &Resources,
        symbol_name: &'static str,
    ) -> Self {
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
        let renderer = resources.get::<Renderer>().unwrap();

        let prefab_asset_handle: Handle<PrefabAsset> =
            asset_resource.load_artifact_symbol_name(symbol_name);
        renderer
            .wait_for_asset_to_load(
                &mut asset_manager,
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
        let renderer = resources.get::<Renderer>().unwrap();

        let prefab_asset = self
            .prefab_asset_handle
            .artifact(asset_resource.storage())
            .unwrap()
            .clone();

        fn load_visible_bounds(
            renderer: &Renderer,
            asset_manager: &mut AssetManager,
            asset_resource: &mut AssetResource,
            asset_handle: &Handle<MeshAsset>,
            asset_name: &str,
        ) -> Option<VisibleBounds> {
            renderer
                .wait_for_asset_to_load(asset_manager, asset_handle, asset_resource, asset_name)
                .unwrap();

            asset_manager
                .committed_asset(asset_handle)
                .map(|x| x.inner.asset_data.visible_bounds)
        }

        for object in &prefab_asset.inner.objects {
            //log::debug!("create object {:?}", object);
            if let Some(model) = &object.model {
                let model_asset_handle = model.model.artifact(asset_resource.storage());
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
                    &renderer,
                    &mut *asset_manager,
                    &mut *asset_resource,
                    &mesh_asset,
                    &format!("visible bounds for {:?}", model.model),
                );

                if let Some(visible_bounds) = visible_bounds {
                    let mut visibility_resource =
                        resources.get_mut::<VisibilityResource>().unwrap();
                    entry.add_component(VisibilityComponent {
                        visibility_object_handle: {
                            let handle = visibility_resource.register_static_object(
                                ObjectId::from(entity),
                                CullModel::VisibleBounds(visible_bounds),
                                vec![render_object],
                            );
                            handle.set_transform(
                                transform_component.translation,
                                transform_component.rotation,
                                transform_component.scale,
                            );
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
                        super::add_point_light(
                            resources,
                            world,
                            object.transform.position,
                            light.color.extend(1.0),
                            light.intensity * 0.15,
                            true,
                        );
                    }
                    PrefabAssetDataObjectLightKind::Spot => {
                        super::add_spot_light(
                            resources,
                            world,
                            //glam::Vec3::new(-3.0, 3.0, 2.0),
                            object.transform.position,
                            object.transform.rotation * -glam::Vec3::Z,
                            light.spot.as_ref().unwrap().outer_angle,
                            light.color.extend(1.0),
                            light.intensity * 0.15,
                            true,
                        );
                    }
                    PrefabAssetDataObjectLightKind::Directional => {
                        super::add_directional_light(
                            resources,
                            world,
                            object.transform.rotation * -glam::Vec3::Z,
                            light.color.extend(1.0),
                            light.intensity * 0.15,
                            true,
                        );
                    }
                }
            }
        }
    }
}
