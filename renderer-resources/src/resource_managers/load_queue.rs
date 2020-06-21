use atelier_assets::loader::AssetLoadOp;
use atelier_assets::core::AssetUuid;
use type_uuid::TypeUuid;
use renderer_assets::asset_storage::{ResourceLoadHandler};
use std::marker::PhantomData;
use crossbeam_channel::{Sender, Receiver};
use renderer_assets::assets::shader::ShaderAsset;
use renderer_assets::assets::pipeline::{
    PipelineAsset, MaterialAsset, MaterialInstanceAsset, RenderpassAsset,
};
use renderer_assets::assets::image::ImageAsset;
use atelier_assets::loader::LoadHandle;
use renderer_assets::assets::gltf::MeshAsset;
use renderer_assets::assets::buffer::BufferAsset;

//
// Message handling for asset load/commit/free events
//
pub struct LoadRequest<T> {
    pub load_handle: LoadHandle,
    pub load_op: AssetLoadOp,
    pub asset: T,
}

pub struct CommitRequest<T> {
    pub load_handle: LoadHandle,
    phantom_data: PhantomData<T>,
}

pub struct FreeRequest<T> {
    pub load_handle: LoadHandle,
    phantom_data: PhantomData<T>,
}

pub struct LoadQueuesTx<T> {
    load_request_tx: Sender<LoadRequest<T>>,
    commit_request_tx: Sender<CommitRequest<T>>,
    free_request_tx: Sender<FreeRequest<T>>,
}

impl<T> Clone for LoadQueuesTx<T> {
    fn clone(&self) -> Self {
        LoadQueuesTx {
            load_request_tx: self.load_request_tx.clone(),
            commit_request_tx: self.commit_request_tx.clone(),
            free_request_tx: self.free_request_tx.clone(),
        }
    }
}

pub struct LoadQueuesRx<T> {
    load_request_rx: Receiver<LoadRequest<T>>,
    commit_request_rx: Receiver<CommitRequest<T>>,
    free_request_rx: Receiver<FreeRequest<T>>,
}

pub struct LoadQueues<T> {
    tx: LoadQueuesTx<T>,
    rx: LoadQueuesRx<T>,
}

impl<T> LoadQueues<T> {
    pub fn take_load_requests(&mut self) -> Vec<LoadRequest<T>> {
        self.rx.load_request_rx.try_iter().collect()
    }

    pub fn take_commit_requests(&mut self) -> Vec<CommitRequest<T>> {
        self.rx.commit_request_rx.try_iter().collect()
    }

    pub fn take_free_requests(&mut self) -> Vec<FreeRequest<T>> {
        self.rx.free_request_rx.try_iter().collect()
    }
}

impl<T> LoadQueues<T>
where
    T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
{
    pub fn create_load_handler(&self) -> GenericLoadHandler<T> {
        GenericLoadHandler {
            load_queues: self.tx.clone(),
        }
    }
}

impl<T> Default for LoadQueues<T> {
    fn default() -> Self {
        let (load_request_tx, load_request_rx) = crossbeam_channel::unbounded();
        let (commit_request_tx, commit_request_rx) = crossbeam_channel::unbounded();
        let (free_request_tx, free_request_rx) = crossbeam_channel::unbounded();

        let tx = LoadQueuesTx {
            load_request_tx,
            commit_request_tx,
            free_request_tx,
        };

        let rx = LoadQueuesRx {
            load_request_rx,
            commit_request_rx,
            free_request_rx,
        };

        LoadQueues { tx, rx }
    }
}

//
// A generic load handler that allows routing load/commit/free events
//
pub struct GenericLoadHandler<AssetT>
where
    AssetT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
{
    load_queues: LoadQueuesTx<AssetT>,
}

impl<AssetT> ResourceLoadHandler<AssetT> for GenericLoadHandler<AssetT>
where
    AssetT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
{
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        version: u32,
        asset: &AssetT,
        load_op: AssetLoadOp,
    ) {
        log::trace!(
            "ResourceLoadHandler update_asset {} {:?}",
            core::any::type_name::<AssetT>(),
            load_handle
        );
        let request = LoadRequest {
            load_handle,
            load_op,
            asset: asset.clone(),
        };

        self.load_queues.load_request_tx.send(request);
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        version: u32,
        asset: &AssetT,
    ) {
        log::trace!(
            "ResourceLoadHandler commit_asset_version {} {:?}",
            core::any::type_name::<AssetT>(),
            load_handle
        );
        let request = CommitRequest {
            load_handle,
            phantom_data: Default::default(),
        };

        self.load_queues.commit_request_tx.send(request);
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        version: u32,
    ) {
        log::trace!(
            "ResourceLoadHandler free {} {:?}",
            core::any::type_name::<AssetT>(),
            load_handle
        );
        let request = FreeRequest {
            load_handle,
            phantom_data: Default::default(),
        };

        self.load_queues.free_request_tx.send(request);
    }
}

#[derive(Default)]
pub struct LoadQueueSet {
    pub shader_modules: LoadQueues<ShaderAsset>,
    pub graphics_pipelines: LoadQueues<PipelineAsset>,
    pub renderpasses: LoadQueues<RenderpassAsset>,
    pub materials: LoadQueues<MaterialAsset>,
    pub material_instances: LoadQueues<MaterialInstanceAsset>,
    pub images: LoadQueues<ImageAsset>,
    pub buffers: LoadQueues<BufferAsset>,
    pub meshes: LoadQueues<MeshAsset>,
}
