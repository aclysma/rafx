use crate::assets::buffer::asset_upload_queue::{
    BufferAssetUploadOpResult, BufferAssetUploadQueue,
};
use crate::distill_impl::{AssetResource, ResourceAssetLoader};
use crate::{
    AssetLookup, AssetManager, AssetTypeHandler, BufferAsset, BufferAssetData, DynAssetLookup,
    LoadQueues,
};
use rafx_api::RafxBuffer;
use rafx_framework::RafxResult;
use std::any::TypeId;

pub struct BufferAssetTypeHandler {
    asset_lookup: AssetLookup<BufferAsset>,
    load_queues: LoadQueues<BufferAssetData, BufferAsset>,
    buffer_upload_queue: BufferAssetUploadQueue,
}

impl BufferAssetTypeHandler {
    pub fn create(
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) -> RafxResult<Box<dyn AssetTypeHandler>> {
        let load_queues = LoadQueues::<BufferAssetData, BufferAsset>::default();

        asset_resource.add_storage_with_loader::<BufferAssetData, BufferAsset, _>(Box::new(
            ResourceAssetLoader(load_queues.create_loader()),
        ));

        let buffer_upload_queue =
            BufferAssetUploadQueue::new(asset_manager.upload_queue_context())?;

        Ok(Box::new(Self {
            asset_lookup: AssetLookup::new(asset_resource.loader()),
            load_queues,
            buffer_upload_queue,
        }))
    }
}

impl AssetTypeHandler for BufferAssetTypeHandler {
    fn process_load_requests(
        &mut self,
        asset_manager: &mut AssetManager,
    ) -> RafxResult<()> {
        for request in self.load_queues.take_load_requests() {
            //TODO: Route the request directly to the upload queue
            log::trace!("Uploading buffer {:?}", request.load_handle);
            self.buffer_upload_queue.upload_buffer(request)?;
        }

        let results: Vec<_> = self
            .buffer_upload_queue
            .buffer_upload_result_rx
            .try_iter()
            .collect();
        for result in results {
            match result {
                BufferAssetUploadOpResult::UploadComplete(load_op, result_tx, buffer) => {
                    log::trace!("Uploading buffer {:?} complete", load_op.load_handle());

                    if asset_manager
                        .device_context()
                        .device_info()
                        .debug_names_enabled
                    {
                        buffer.set_debug_name(&format!("Buffer Asset {:?}", load_op.load_handle()));
                    }

                    let loaded_asset = finish_load_buffer(asset_manager, buffer);
                    crate::assets::asset_type_handler::handle_load_result(
                        load_op,
                        loaded_asset,
                        &mut self.asset_lookup,
                        result_tx,
                    );
                }
                BufferAssetUploadOpResult::UploadError(load_handle) => {
                    log::trace!("Uploading buffer {:?} failed", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
                BufferAssetUploadOpResult::UploadDrop(load_handle) => {
                    log::trace!("Uploading buffer {:?} cancelled", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
            }
        }

        crate::assets::asset_type_handler::handle_commit_requests(
            &mut self.load_queues,
            &mut self.asset_lookup,
        );
        crate::assets::asset_type_handler::handle_free_requests(
            &mut self.load_queues,
            &mut self.asset_lookup,
        );
        Ok(())
    }

    fn asset_lookup(&self) -> &dyn DynAssetLookup {
        &self.asset_lookup
    }

    fn asset_type_id(&self) -> TypeId {
        TypeId::of::<BufferAsset>()
    }
}

#[profiling::function]
fn finish_load_buffer(
    asset_manager: &mut AssetManager,
    buffer: RafxBuffer,
) -> RafxResult<BufferAsset> {
    let buffer = asset_manager.resources().insert_buffer(buffer);

    Ok(BufferAsset { buffer })
}
