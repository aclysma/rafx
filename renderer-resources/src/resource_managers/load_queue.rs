use atelier_assets::loader::AssetLoadOp;
use type_uuid::TypeUuid;
use crate::ResourceLoadHandler;
use std::marker::PhantomData;
use crossbeam_channel::{Sender, Receiver};
use renderer_assets::assets::shader::ShaderAssetData;
use renderer_assets::assets::pipeline::{
    PipelineAssetData, MaterialAssetData, MaterialInstanceAssetData, RenderpassAssetData,
};
use renderer_assets::assets::image::ImageAssetData;
use atelier_assets::loader::LoadHandle;
use renderer_assets::assets::buffer::BufferAssetData;
use crate::resource_load_handler::ResourceLoadResult;
use crate::resource_managers::asset_lookup::{ShaderAsset, PipelineAsset, RenderpassAsset, MaterialAsset, MaterialInstanceAsset, ImageAsset, BufferAsset};

//
// Message handling for asset load/commit/free events
//
pub struct LoadRequest<AssetDataT, LoadedT> {
    pub load_handle: LoadHandle,
    pub load_op: AssetLoadOp,
    pub result_tx: Sender<LoadedT>,
    pub asset: AssetDataT,
}

pub struct CommitRequest<T> {
    pub load_handle: LoadHandle,
    phantom_data: PhantomData<T>,
}

pub struct FreeRequest<T> {
    pub load_handle: LoadHandle,
    phantom_data: PhantomData<T>,
}

pub struct LoadQueuesTx<AssetDataT, LoadedT> {
    load_request_tx: Sender<LoadRequest<AssetDataT, LoadedT>>,
    commit_request_tx: Sender<CommitRequest<AssetDataT>>,
    free_request_tx: Sender<FreeRequest<AssetDataT>>,
}

impl<AssetDataT, LoadedT> Clone for LoadQueuesTx<AssetDataT, LoadedT> {
    fn clone(&self) -> Self {
        LoadQueuesTx {
            load_request_tx: self.load_request_tx.clone(),
            commit_request_tx: self.commit_request_tx.clone(),
            free_request_tx: self.free_request_tx.clone(),
        }
    }
}

pub struct LoadQueuesRx<AssetDataT, LoadedT> {
    load_request_rx: Receiver<LoadRequest<AssetDataT, LoadedT>>,
    commit_request_rx: Receiver<CommitRequest<AssetDataT>>,
    free_request_rx: Receiver<FreeRequest<AssetDataT>>,
}

pub struct LoadQueues<AssetDataT, LoadedT> {
    tx: LoadQueuesTx<AssetDataT, LoadedT>,
    rx: LoadQueuesRx<AssetDataT, LoadedT>,
}

impl<AssetDataT, LoadedT> LoadQueues<AssetDataT, LoadedT> {
    pub fn take_load_requests(&mut self) -> Vec<LoadRequest<AssetDataT, LoadedT>> {
        self.rx.load_request_rx.try_iter().collect()
    }

    pub fn take_commit_requests(&mut self) -> Vec<CommitRequest<AssetDataT>> {
        self.rx.commit_request_rx.try_iter().collect()
    }

    pub fn take_free_requests(&mut self) -> Vec<FreeRequest<AssetDataT>> {
        self.rx.free_request_rx.try_iter().collect()
    }
}

impl<AssetDataT, LoadedT> LoadQueues<AssetDataT, LoadedT>
where
    AssetDataT: for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
    LoadedT: TypeUuid + 'static + Send
{
    pub fn create_load_handler(&self) -> GenericLoadHandler<AssetDataT, LoadedT> {
        GenericLoadHandler {
            load_queues: self.tx.clone(),
        }
    }
}

impl<AssetDataT, LoadedT> Default for LoadQueues<AssetDataT, LoadedT> {
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
pub struct GenericLoadHandler<AssetDataT, LoadedT>
where
    AssetDataT: for<'a> serde::Deserialize<'a> + 'static + Send,
    LoadedT: TypeUuid + 'static + Send
{
    load_queues: LoadQueuesTx<AssetDataT, LoadedT>,
}

impl<AssetDataT, LoadedT> ResourceLoadHandler<AssetDataT, LoadedT> for GenericLoadHandler<AssetDataT, LoadedT>
where
    AssetDataT: for<'a> serde::Deserialize<'a> + 'static + Send,
    LoadedT: TypeUuid + 'static + Send,
{
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        asset: AssetDataT,
    ) -> ResourceLoadResult<LoadedT> {
        log::trace!(
            "ResourceLoadHandler update_asset {} {:?}",
            core::any::type_name::<AssetDataT>(),
            load_handle
        );

        let (result_tx, result_rx) = crossbeam_channel::bounded(1);

        let request = LoadRequest {
            load_handle,
            load_op,
            result_tx,
            asset,
        };

        self.load_queues.load_request_tx.send(request).unwrap();
        ResourceLoadResult::new(result_rx)
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
    ) {
        log::trace!(
            "ResourceLoadHandler commit_asset_version {} {:?}",
            core::any::type_name::<AssetDataT>(),
            load_handle
        );
        let request = CommitRequest {
            load_handle,
            phantom_data: Default::default(),
        };

        self.load_queues.commit_request_tx.send(request).unwrap();
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
    ) {
        log::trace!(
            "ResourceLoadHandler free {} {:?}",
            core::any::type_name::<AssetDataT>(),
            load_handle
        );
        let request = FreeRequest {
            load_handle,
            phantom_data: Default::default(),
        };

        self.load_queues.free_request_tx.send(request).unwrap();
    }
}

#[derive(Default)]
pub struct LoadQueueSet {
    pub shader_modules: LoadQueues<ShaderAssetData, ShaderAsset>,
    pub graphics_pipelines: LoadQueues<PipelineAssetData, PipelineAsset>,
    pub renderpasses: LoadQueues<RenderpassAssetData, RenderpassAsset>,
    pub materials: LoadQueues<MaterialAssetData, MaterialAsset>,
    pub material_instances: LoadQueues<MaterialInstanceAssetData, MaterialInstanceAsset>,
    pub images: LoadQueues<ImageAssetData, ImageAsset>,
    pub buffers: LoadQueues<BufferAssetData, BufferAsset>,
}
