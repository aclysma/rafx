use super::asset_upload_queue::{ImageAssetUploadOpResult, ImageAssetUploadQueue};
use crate::hydrate_impl::{AssetResource, RafxResourceAssetLoader};
use crate::{
    AssetLookup, AssetManager, AssetTypeHandler, DynAssetLookup, ImageAsset, ImageAssetData,
    LoadQueues,
};
use rafx_api::{RafxResult, RafxTexture};
use std::any::TypeId;

pub struct ImageAssetTypeHandler {
    asset_lookup: AssetLookup<ImageAsset>,
    load_queues: LoadQueues<ImageAssetData, ImageAsset>,
    image_upload_queue: ImageAssetUploadQueue,
}

impl ImageAssetTypeHandler {
    pub fn create(
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) -> RafxResult<Box<dyn AssetTypeHandler>> {
        let load_queues = LoadQueues::<ImageAssetData, ImageAsset>::default();

        asset_resource.add_storage_with_loader::<ImageAssetData, ImageAsset, _>(Box::new(
            RafxResourceAssetLoader(load_queues.create_loader()),
        ));

        let image_upload_queue = ImageAssetUploadQueue::new(
            asset_manager.upload_queue_context(),
            asset_manager.device_context(),
        )?;

        Ok(Box::new(Self {
            asset_lookup: AssetLookup::default(),
            load_queues,
            image_upload_queue,
        }))
    }
}

impl AssetTypeHandler for ImageAssetTypeHandler {
    fn process_load_requests(
        &mut self,
        asset_manager: &mut AssetManager,
    ) -> RafxResult<()> {
        for request in self.load_queues.take_load_requests() {
            //TODO: Route the request directly to the upload queue
            log::trace!("Uploading image {:?}", request.load_handle);
            self.image_upload_queue.upload_image(request)?;
        }

        let results: Vec<_> = self
            .image_upload_queue
            .image_upload_result_rx
            .try_iter()
            .collect();
        for result in results {
            match result {
                ImageAssetUploadOpResult::UploadComplete(load_op, result_tx, texture) => {
                    log::trace!("Uploading image {:?} complete", load_op.load_handle());

                    if asset_manager
                        .device_context()
                        .device_info()
                        .debug_names_enabled
                    {
                        texture.set_debug_name(&format!("Image Asset {:?}", load_op.load_handle()));
                    }

                    let loaded_asset = finish_load_image(asset_manager, texture);
                    crate::assets::asset_type_handler::handle_load_result(
                        load_op,
                        loaded_asset,
                        &mut self.asset_lookup,
                        result_tx,
                    );
                }
                ImageAssetUploadOpResult::UploadError(load_handle) => {
                    log::trace!("Uploading image {:?} failed", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
                ImageAssetUploadOpResult::UploadDrop(load_handle) => {
                    log::trace!("Uploading image {:?} cancelled", load_handle);
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
        TypeId::of::<ImageAsset>()
    }
}

#[profiling::function]
fn finish_load_image(
    asset_manager: &mut AssetManager,
    texture: RafxTexture,
) -> RafxResult<ImageAsset> {
    let image = asset_manager.resources().insert_image(texture);

    let image_view = asset_manager
        .resources()
        .get_or_create_image_view(&image, None)?;

    Ok(ImageAsset { image, image_view })
}
