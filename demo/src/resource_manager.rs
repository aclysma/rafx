use renderer::resources::resource_managers::{
    DescriptorSetArc, AssetLookup, ResourceArc, LoadQueues, GenericLoadHandler, ResourceManager,
};
use renderer::vulkan::VkBufferRaw;
use crate::asset_lookup::{
    GameLoadedAssetMetrics, GameLoadedAssetLookupSet, LoadedMesh, LoadedMeshPart,
};
use atelier_assets::loader::handle::Handle;
use atelier_assets::loader::handle::AssetHandle;
use ash::prelude::VkResult;
use atelier_assets::loader::AssetLoadOp;
use crate::assets::gltf::MeshAsset;

pub struct MeshPartInfo {
    pub material_instance: Vec<Vec<DescriptorSetArc>>,
}

pub struct MeshInfo {
    pub vertex_buffer: ResourceArc<VkBufferRaw>,
    pub index_buffer: ResourceArc<VkBufferRaw>,
    pub mesh_asset: MeshAsset,
    pub mesh_parts: Vec<MeshPartInfo>,
}

#[derive(Debug)]
pub struct GameResourceManagerMetrics {
    pub game_loaded_asset_metrics: GameLoadedAssetMetrics,
}

#[derive(Default)]
pub struct GameLoadQueueSet {
    pub meshes: LoadQueues<MeshAsset>,
}

pub struct GameResourceManager {
    loaded_assets: GameLoadedAssetLookupSet,
    load_queues: GameLoadQueueSet,
}

impl GameResourceManager {
    pub fn new() -> Self {
        GameResourceManager {
            loaded_assets: Default::default(),
            load_queues: Default::default(),
        }
    }

    pub fn create_mesh_load_handler(&self) -> GenericLoadHandler<MeshAsset> {
        self.load_queues.meshes.create_load_handler()
    }

    pub fn get_mesh_info(
        &self,
        handle: &Handle<MeshAsset>,
    ) -> Option<MeshInfo> {
        self.loaded_assets
            .meshes
            .get_committed(handle.load_handle())
            .map(|loaded_mesh| {
                let mesh_parts: Vec<_> = loaded_mesh
                    .mesh_parts
                    .iter()
                    .map(|x| MeshPartInfo {
                        material_instance: x.material_instance.clone(),
                    })
                    .collect();

                MeshInfo {
                    vertex_buffer: loaded_mesh.vertex_buffer.clone(),
                    index_buffer: loaded_mesh.index_buffer.clone(),
                    mesh_asset: loaded_mesh.asset.clone(),
                    mesh_parts,
                }
            })
    }

    // Call whenever you want to handle assets loading/unloading
    pub fn update_resources(
        &mut self,
        resource_manager: &ResourceManager,
    ) -> VkResult<()> {
        self.process_mesh_load_requests(resource_manager);
        Ok(())
    }

    pub fn metrics(&self) -> GameResourceManagerMetrics {
        let game_loaded_asset_metrics = self.loaded_assets.metrics();

        GameResourceManagerMetrics {
            game_loaded_asset_metrics,
        }
    }

    fn process_mesh_load_requests(
        &mut self,
        resource_manager: &ResourceManager,
    ) {
        for request in self.load_queues.meshes.take_load_requests() {
            log::trace!("Create mesh {:?}", request.load_handle);
            let loaded_asset = self.load_mesh(resource_manager, &request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.meshes,
            );
        }

        Self::handle_commit_requests(&mut self.load_queues.meshes, &mut self.loaded_assets.meshes);
        Self::handle_free_requests(&mut self.load_queues.meshes, &mut self.loaded_assets.meshes);
    }

    fn handle_load_result<LoadedAssetT>(
        load_op: AssetLoadOp,
        loaded_asset: VkResult<LoadedAssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>,
    ) {
        match loaded_asset {
            Ok(loaded_asset) => {
                asset_lookup.set_uncommitted(load_op.load_handle(), loaded_asset);
                load_op.complete()
            }
            Err(err) => {
                load_op.error(err);
            }
        }
    }

    fn handle_commit_requests<AssetT, LoadedAssetT>(
        load_queues: &mut LoadQueues<AssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>,
    ) {
        for request in load_queues.take_commit_requests() {
            log::info!(
                "commit asset {:?} {}",
                request.load_handle,
                core::any::type_name::<AssetT>()
            );
            asset_lookup.commit(request.load_handle);
        }
    }

    fn handle_free_requests<AssetT, LoadedAssetT>(
        load_queues: &mut LoadQueues<AssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>,
    ) {
        for request in load_queues.take_commit_requests() {
            asset_lookup.commit(request.load_handle);
        }
    }

    fn load_mesh(
        &mut self,
        resource_manager: &ResourceManager,
        mesh_asset: &MeshAsset,
    ) -> VkResult<LoadedMesh> {
        let vertex_buffer = resource_manager
            .loaded_assets()
            .buffers
            .get_latest(mesh_asset.vertex_buffer.load_handle())
            .unwrap()
            .buffer
            .clone();
        let index_buffer = resource_manager
            .loaded_assets()
            .buffers
            .get_latest(mesh_asset.index_buffer.load_handle())
            .unwrap()
            .buffer
            .clone();

        let mesh_parts: Vec<_> = mesh_asset
            .mesh_parts
            .iter()
            .map(|mesh_part| {
                let material_instance_info =
                    resource_manager.get_material_instance_info(&mesh_part.material_instance);

                LoadedMeshPart {
                    material_instance: material_instance_info.descriptor_sets.clone(),
                }
            })
            .collect();

        Ok(LoadedMesh {
            vertex_buffer,
            index_buffer,
            asset: mesh_asset.clone(),
            mesh_parts,
        })
    }
}

impl Drop for GameResourceManager {
    fn drop(&mut self) {
        log::info!("Cleaning up game resource manager");
        log::trace!("Game Resource Manager Metrics:\n{:#?}", self.metrics());

        // Wipe out any loaded assets. This will potentially drop ref counts on resources
        self.loaded_assets.destroy();

        log::info!("Dropping game resource manager");
        log::trace!("Resource Game Manager Metrics:\n{:#?}", self.metrics());
    }
}
