use crate::assets::upload::BufferUploadOpResult;
use crate::distill_impl::{AssetResource, ResourceAssetLoader};
use crate::{
    AssetLookup, AssetManager, AssetTypeHandler, AssetTypeHandlerFactory, DynAssetLookup,
    LoadQueues,
};
use rafx_api::RafxBuffer;
use rafx_framework::ResourceArc;
use rafx_framework::{BufferResource, RafxResult};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "2d6653ce-5f77-40a2-b050-f2d148699d78"]
pub struct BufferAssetData {
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "fc3b1eb8-c986-449e-a165-6a8f4582e6c5"]
pub struct BufferAsset {
    pub buffer: ResourceArc<BufferResource>,
}

pub struct BufferAssetTypeHandler {
    asset_lookup: AssetLookup<BufferAsset>,
    load_queues: LoadQueues<BufferAssetData, BufferAsset>,
}

impl AssetTypeHandlerFactory for BufferAssetTypeHandler {
    fn create(asset_resource: &mut AssetResource) -> Box<dyn AssetTypeHandler> {
        let load_queues = LoadQueues::<BufferAssetData, BufferAsset>::default();

        asset_resource.add_storage_with_loader::<BufferAssetData, BufferAsset, _>(Box::new(
            ResourceAssetLoader(load_queues.create_loader()),
        ));

        Box::new(Self {
            asset_lookup: AssetLookup::new(asset_resource.loader()),
            load_queues,
        })
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
            asset_manager.upload_manager().upload_buffer(request)?;
        }

        let results: Vec<_> = asset_manager
            .upload_manager()
            .buffer_upload_result_rx
            .try_iter()
            .collect();
        for result in results {
            match result {
                BufferUploadOpResult::UploadComplete(load_op, result_tx, buffer) => {
                    log::trace!("Uploading buffer {:?} complete", load_op.load_handle());
                    let loaded_asset = finish_load_buffer(asset_manager, buffer);
                    crate::assets::asset_type_handler::handle_load_result(
                        load_op,
                        loaded_asset,
                        &mut self.asset_lookup,
                        result_tx,
                    );
                }
                BufferUploadOpResult::UploadError(load_handle) => {
                    log::trace!("Uploading buffer {:?} failed", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
                BufferUploadOpResult::UploadDrop(load_handle) => {
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
