use atelier_assets::loader::AssetLoadOp;
use type_uuid::TypeUuid;
use crate::ResourceLoadHandler;
use std::marker::PhantomData;
use crossbeam_channel::{Sender, Receiver};
use renderer_assets::assets::shader::ShaderAsset;
use renderer_assets::assets::pipeline::{
    PipelineAsset, MaterialAsset, MaterialInstanceAsset, RenderpassAsset,
};
use renderer_assets::assets::image::ImageAsset;
use atelier_assets::loader::LoadHandle;
use renderer_assets::assets::buffer::BufferAsset;
use crate::resource_load_handler::ResourceLoadResult;
use crate::resource_managers::asset_lookup::{LoadedShaderModule, LoadedGraphicsPipeline, LoadedRenderpass, LoadedMaterial, LoadedMaterialInstance, LoadedImage, LoadedBuffer};

//
// Message handling for asset load/commit/free events
//
pub struct LoadRequest<AssetT, LoadedT> {
    pub load_handle: LoadHandle,
    pub load_op: AssetLoadOp,
    pub result_tx: Sender<LoadedT>,
    pub asset: AssetT,
}

pub struct CommitRequest<T> {
    pub load_handle: LoadHandle,
    phantom_data: PhantomData<T>,
}

pub struct FreeRequest<T> {
    pub load_handle: LoadHandle,
    phantom_data: PhantomData<T>,
}

pub struct LoadQueuesTx<AssetT, LoadedT> {
    load_request_tx: Sender<LoadRequest<AssetT, LoadedT>>,
    commit_request_tx: Sender<CommitRequest<AssetT>>,
    free_request_tx: Sender<FreeRequest<AssetT>>,
}

impl<AssetT, LoadedT> Clone for LoadQueuesTx<AssetT, LoadedT> {
    fn clone(&self) -> Self {
        LoadQueuesTx {
            load_request_tx: self.load_request_tx.clone(),
            commit_request_tx: self.commit_request_tx.clone(),
            free_request_tx: self.free_request_tx.clone(),
        }
    }
}

pub struct LoadQueuesRx<AssetT, LoadedT> {
    load_request_rx: Receiver<LoadRequest<AssetT, LoadedT>>,
    commit_request_rx: Receiver<CommitRequest<AssetT>>,
    free_request_rx: Receiver<FreeRequest<AssetT>>,
}

pub struct LoadQueues<AssetT, LoadedT> {
    tx: LoadQueuesTx<AssetT, LoadedT>,
    rx: LoadQueuesRx<AssetT, LoadedT>,
}

impl<AssetT, LoadedT> LoadQueues<AssetT, LoadedT> {
    pub fn take_load_requests(&mut self) -> Vec<LoadRequest<AssetT, LoadedT>> {
        self.rx.load_request_rx.try_iter().collect()
    }

    pub fn take_commit_requests(&mut self) -> Vec<CommitRequest<AssetT>> {
        self.rx.commit_request_rx.try_iter().collect()
    }

    pub fn take_free_requests(&mut self) -> Vec<FreeRequest<AssetT>> {
        self.rx.free_request_rx.try_iter().collect()
    }
}

impl<AssetT, LoadedT> LoadQueues<AssetT, LoadedT>
where
    AssetT: for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
    LoadedT: TypeUuid + 'static + Send
{
    pub fn create_load_handler(&self) -> GenericLoadHandler<AssetT, LoadedT> {
        GenericLoadHandler {
            load_queues: self.tx.clone(),
        }
    }
}

impl<AssetT, LoadedT> Default for LoadQueues<AssetT, LoadedT> {
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
pub struct GenericLoadHandler<AssetT, LoadedT>
where
    AssetT: for<'a> serde::Deserialize<'a> + 'static + Send,
    LoadedT: TypeUuid + 'static + Send
{
    load_queues: LoadQueuesTx<AssetT, LoadedT>,
}

impl<AssetT, LoadedT> ResourceLoadHandler<AssetT, LoadedT> for GenericLoadHandler<AssetT, LoadedT>
where
    AssetT: for<'a> serde::Deserialize<'a> + 'static + Send,
    LoadedT: TypeUuid + 'static + Send,
{
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        asset: AssetT,
    ) -> ResourceLoadResult<LoadedT> {
        log::trace!(
            "ResourceLoadHandler update_asset {} {:?}",
            core::any::type_name::<AssetT>(),
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
            core::any::type_name::<AssetT>(),
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
            core::any::type_name::<AssetT>(),
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
    pub shader_modules: LoadQueues<ShaderAsset, LoadedShaderModule>,
    pub graphics_pipelines: LoadQueues<PipelineAsset, LoadedGraphicsPipeline>,
    pub renderpasses: LoadQueues<RenderpassAsset, LoadedRenderpass>,
    pub materials: LoadQueues<MaterialAsset, LoadedMaterial>,
    pub material_instances: LoadQueues<MaterialInstanceAsset, LoadedMaterialInstance>,
    pub images: LoadQueues<ImageAsset, LoadedImage>,
    pub buffers: LoadQueues<BufferAsset, LoadedBuffer>,
}
