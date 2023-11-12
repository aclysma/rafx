use hydrate_base::handle::Handle;
use hydrate_base::ArtifactId;
use legion::{Resources, World};
use rafx::assets::AssetManager;
use rafx::assets::AssetResource;
use rafx::rafx_visibility::VisibleBounds;
use rafx::render_features::RenderObjectHandle;
use rafx::renderer::Renderer;
use rafx::visibility::{CullModel, ObjectId, VisibilityResource};
use rafx_plugins::components::{MeshComponent, TransformComponent, VisibilityComponent};

#[cfg(feature = "basic-pipeline")]
use rafx_plugins::assets::mesh_basic::MeshBasicAsset as MeshAsset;
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::features::mesh_basic::{
    MeshBasicRenderObject as MeshRenderObject, MeshBasicRenderObjectSet as MeshRenderObjectSet,
};

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::assets::mesh_adv::MeshAdvAsset as MeshAsset;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::features::mesh_adv::{
    MeshAdvRenderObject as MeshRenderObject, MeshAdvRenderObjectSet as MeshRenderObjectSet,
};

pub struct SpawnableMesh {
    render_object: RenderObjectHandle,
    visible_bounds: VisibleBounds,
}

impl SpawnableMesh {
    pub fn spawn(
        &self,
        resources: &Resources,
        world: &mut World,
        transform_component: TransformComponent,
    ) {
        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        let mesh_component = MeshComponent {
            render_object_handle: self.render_object.clone(),
        };

        let entity = world.push((transform_component.clone(), mesh_component));
        let mut entry = world.entry(entity).unwrap();
        entry.add_component(VisibilityComponent {
            visibility_object_handle: {
                let handle = visibility_resource.register_dynamic_object(
                    ObjectId::from(entity),
                    CullModel::VisibleBounds(self.visible_bounds.clone()),
                    vec![self.render_object.clone()],
                );
                handle.set_transform(
                    transform_component.translation,
                    transform_component.rotation,
                    transform_component.scale,
                );
                handle
            },
        });
    }

    fn do_load_spawnable_mesh(
        resources: &Resources,
        asset_resource: &mut AssetResource,
        asset_handle: Handle<MeshAsset>,
    ) -> SpawnableMesh {
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut mesh_render_objects = resources.get_mut::<MeshRenderObjectSet>().unwrap();
        let renderer = resources.get::<Renderer>().unwrap();

        let render_object = mesh_render_objects.register_render_object(MeshRenderObject {
            mesh: asset_handle.clone(),
        });

        renderer
            .wait_for_asset_to_load(
                &mut asset_manager,
                &asset_handle,
                asset_resource,
                "spawnable mesh",
            )
            .unwrap();

        let visible_bounds = asset_manager
            .committed_asset(&asset_handle)
            .unwrap()
            .inner
            .asset_data
            .visible_bounds;

        SpawnableMesh {
            render_object,
            visible_bounds,
        }
    }

    pub fn blocking_load_from_artifact_id(
        resources: &Resources,
        artifact_id: ArtifactId,
    ) -> SpawnableMesh {
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
        let handle = asset_resource.load_asset(artifact_id);
        Self::do_load_spawnable_mesh(resources, &mut *asset_resource, handle)
    }

    pub fn blocking_load_from_symbol_name(
        resources: &Resources,
        symbol_name: &'static str,
    ) -> SpawnableMesh {
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
        let handle = asset_resource.load_asset_symbol_name(symbol_name);
        Self::do_load_spawnable_mesh(resources, &mut *asset_resource, handle)
    }

    pub fn blocking_load_from_path<T: Into<String>>(
        resources: &Resources,
        path: T,
    ) -> SpawnableMesh {
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
        let handle = asset_resource.load_asset_path(path);
        Self::do_load_spawnable_mesh(resources, &mut *asset_resource, handle)
    }
}
