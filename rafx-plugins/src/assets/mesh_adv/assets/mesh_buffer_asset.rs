use super::free_list_suballocator::*;
use crossbeam_channel::{Receiver, Sender};
use fnv::FnvHashMap;
use hydrate_base::LoadHandle;
use rafx::api::{RafxBufferDef, RafxMemoryUsage, RafxQueueType, RafxResourceType};
use rafx::assets::RafxResourceAssetLoader;
use rafx::assets::{
    asset_type_handler, AssetLookup, AssetManager, AssetTypeHandler, DynAssetLookup, LoadQueues,
    LoadRequest, PushBuffer, UploadAssetOp, UploadAssetOpResult,
};
use rafx::framework::upload::UploadQueueContext;
use rafx::framework::ResourceArc;
use rafx::framework::{BufferResource, RenderResources};
use rafx::render_feature_renderer_prelude::AssetResource;
use rafx::RafxResult;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "4b53d85c-98e6-4d77-af8b-0914e67e10dc"]
pub struct MeshAdvBufferAssetData {
    pub resource_type: RafxResourceType,
    pub alignment: u32,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl MeshAdvBufferAssetData {
    pub fn from_vec<T: 'static>(
        resource_type: RafxResourceType,
        alignment: u32,
        data: &Vec<T>,
    ) -> Self {
        let push_buffer = PushBuffer::from_vec(data);
        MeshAdvBufferAssetData {
            resource_type,
            alignment,
            data: push_buffer.into_data(),
        }
    }
}

#[derive(TypeUuid, Clone)]
#[uuid = "009d94d6-6623-4f99-a518-a29fc85c4f2c"]
pub struct MeshAdvBufferAsset {
    allocation: FreeListSuballocatorAllocation,
}

impl MeshAdvBufferAsset {
    pub fn buffer_byte_offset(&self) -> u32 {
        self.allocation.aligned_offset()
    }
}

#[derive(Clone)]
pub struct MeshAdvBindlessBuffers {
    pub vertex: ResourceArc<BufferResource>,
    pub index: ResourceArc<BufferResource>,
}

pub struct MeshAdvBufferAssetTypeHandler {
    asset_lookup: AssetLookup<MeshAdvBufferAsset>,
    load_queues: LoadQueues<MeshAdvBufferAssetData, MeshAdvBufferAsset>,
    buffer_upload_queue: MeshAdvBufferAssetUploadQueue,
    buffers: MeshAdvBindlessBuffers,
    vertex_buffer_suballocator: FreeListSuballocator,
    index_buffer_suballocator: FreeListSuballocator,
    offset_lookup: FnvHashMap<LoadHandle, FreeListSuballocatorAllocation>,
}

const VERTEX_BUFFER_SIZE: u32 = 512 * 1024 * 1024;
const INDEX_BUFFER_SIZE: u32 = 128 * 1024 * 1024;
const BUFFER_ALIGNMENT: u32 = 1024;

impl MeshAdvBufferAssetTypeHandler {
    pub fn create(
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        render_resources: &mut RenderResources,
    ) -> RafxResult<Box<dyn AssetTypeHandler>> {
        let load_queues = LoadQueues::<MeshAdvBufferAssetData, MeshAdvBufferAsset>::default();

        asset_resource.add_storage_with_loader::<MeshAdvBufferAssetData, MeshAdvBufferAsset, _>(
            Box::new(RafxResourceAssetLoader(load_queues.create_loader())),
        );

        let buffer_upload_queue =
            MeshAdvBufferAssetUploadQueue::new(asset_manager.upload_queue_context())?;

        let vertex_data_heap = asset_manager
            .device_context()
            .create_buffer(&RafxBufferDef {
                size: VERTEX_BUFFER_SIZE as u64,
                alignment: BUFFER_ALIGNMENT,
                memory_usage: RafxMemoryUsage::GpuOnly,
                queue_type: RafxQueueType::Transfer,
                resource_type: RafxResourceType::VERTEX_BUFFER,
                ..Default::default()
            })?;
        vertex_data_heap.set_debug_name("MeshAdv Vertex Data Heap");
        let vertex_data_heap = asset_manager
            .resource_manager()
            .resources()
            .insert_buffer(vertex_data_heap);

        let index_data_heap = asset_manager
            .device_context()
            .create_buffer(&RafxBufferDef {
                size: INDEX_BUFFER_SIZE as u64,
                alignment: BUFFER_ALIGNMENT,
                memory_usage: RafxMemoryUsage::GpuOnly,
                queue_type: RafxQueueType::Transfer,
                resource_type: RafxResourceType::INDEX_BUFFER,
                ..Default::default()
            })?;
        index_data_heap.set_debug_name("MeshAdv Index Data Heap");
        let index_data_heap = asset_manager
            .resource_manager()
            .resources()
            .insert_buffer(index_data_heap);

        let heaps = MeshAdvBindlessBuffers {
            index: index_data_heap,
            vertex: vertex_data_heap,
        };

        render_resources.insert(heaps.clone());

        Ok(Box::new(Self {
            asset_lookup: AssetLookup::default(),
            load_queues,
            buffer_upload_queue,
            buffers: heaps,
            vertex_buffer_suballocator: FreeListSuballocator::new(VERTEX_BUFFER_SIZE),
            index_buffer_suballocator: FreeListSuballocator::new(INDEX_BUFFER_SIZE),
            offset_lookup: FnvHashMap::default(),
        }))
    }
}

impl AssetTypeHandler for MeshAdvBufferAssetTypeHandler {
    fn process_load_requests(
        &mut self,
        _asset_manager: &mut AssetManager,
    ) -> RafxResult<()> {
        for request in self.load_queues.take_load_requests() {
            let (dst_buffer, allocation) =
                if request.asset.resource_type == RafxResourceType::VERTEX_BUFFER {
                    let allocation = self
                        .vertex_buffer_suballocator
                        .allocate(request.asset.data.len() as u32, request.asset.alignment)
                        .expect("can't allocate vertex buffer space");
                    (self.buffers.vertex.clone(), allocation)
                } else if request.asset.resource_type == RafxResourceType::INDEX_BUFFER {
                    let allocation = self
                        .index_buffer_suballocator
                        .allocate(request.asset.data.len() as u32, request.asset.alignment)
                        .expect("can't allocate index buffer space");
                    (self.buffers.index.clone(), allocation)
                } else {
                    unimplemented!();
                };

            log::trace!("Uploading MeshAdvBuffer {:?}", request.load_handle);
            let load_handle = request.load_handle.clone();
            self.buffer_upload_queue.upload_buffer(
                request,
                dst_buffer,
                allocation.aligned_offset() as u64,
            )?;
            self.offset_lookup.insert(load_handle, allocation);
        }

        let results: Vec<_> = self
            .buffer_upload_queue
            .buffer_upload_result_rx
            .try_iter()
            .collect();
        for result in results {
            match result {
                MeshAdvBufferAssetUploadOpResult::UploadComplete(load_op, result_tx, _) => {
                    log::trace!(
                        "Uploading MeshAdvBuffer {:?} complete",
                        load_op.load_handle()
                    );
                    let allocation = self.offset_lookup.remove(&load_op.load_handle()).unwrap();
                    let loaded_asset = Ok(MeshAdvBufferAsset { allocation });
                    asset_type_handler::handle_load_result(
                        load_op,
                        loaded_asset,
                        &mut self.asset_lookup,
                        result_tx,
                    );
                }
                MeshAdvBufferAssetUploadOpResult::UploadError(load_handle) => {
                    log::trace!("Uploading MeshAdvBuffer {:?} failed", load_handle);
                    self.offset_lookup.remove(&load_handle).unwrap();
                    // Don't need to do anything - the upload should have triggered an error on the load_op
                }
                MeshAdvBufferAssetUploadOpResult::UploadDrop(load_handle) => {
                    log::trace!("Uploading MeshAdvBuffer {:?} cancelled", load_handle);
                    self.offset_lookup.remove(&load_handle).unwrap();
                    // Don't need to do anything - the upload should have triggered an error on the load_op
                }
            }
        }

        asset_type_handler::handle_commit_requests(&mut self.load_queues, &mut self.asset_lookup);
        asset_type_handler::handle_free_requests(&mut self.load_queues, &mut self.asset_lookup);
        Ok(())
    }

    fn on_frame_complete(&mut self) -> RafxResult<()> {
        self.vertex_buffer_suballocator.on_frame_complete();
        self.index_buffer_suballocator.on_frame_complete();
        Ok(())
    }

    fn asset_lookup(&self) -> &dyn DynAssetLookup {
        &self.asset_lookup
    }

    fn asset_type_id(&self) -> TypeId {
        TypeId::of::<MeshAdvBufferAsset>()
    }
}

pub type MeshAdvBufferAssetUploadOpResult = UploadAssetOpResult<(), MeshAdvBufferAsset>;

pub struct MeshAdvBufferAssetUploadQueue {
    pub upload_queue_context: UploadQueueContext,

    pub buffer_upload_result_tx: Sender<MeshAdvBufferAssetUploadOpResult>,
    pub buffer_upload_result_rx: Receiver<MeshAdvBufferAssetUploadOpResult>,
}

impl MeshAdvBufferAssetUploadQueue {
    pub fn new(upload_queue_context: UploadQueueContext) -> RafxResult<Self> {
        let (buffer_upload_result_tx, buffer_upload_result_rx) = crossbeam_channel::unbounded();

        Ok(MeshAdvBufferAssetUploadQueue {
            upload_queue_context,
            buffer_upload_result_tx,
            buffer_upload_result_rx,
        })
    }

    pub fn upload_buffer(
        &self,
        request: LoadRequest<MeshAdvBufferAssetData, MeshAdvBufferAsset>,
        dst_buffer: ResourceArc<BufferResource>,
        dst_byte_offset: u64,
    ) -> RafxResult<()> {
        let op = Box::new(UploadAssetOp::new(
            request.load_op,
            request.load_handle,
            request.result_tx,
            self.buffer_upload_result_tx.clone(),
        ));
        assert!(!request.asset.data.is_empty());
        self.upload_queue_context.upload_to_existing_buffer(
            op,
            request.asset.resource_type,
            request.asset.data,
            dst_buffer,
            dst_byte_offset,
        )
    }
}

pub type MeshAdvBufferAssetType = MeshAdvBufferAssetTypeHandler;
