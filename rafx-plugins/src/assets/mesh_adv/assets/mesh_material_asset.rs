use crate::assets::mesh_adv::material_db::{
    MaterialDBUploadQueue, MaterialDBUploadQueueContext, MeshAdvMaterial, MeshAdvMaterialDef,
};
use crate::assets::mesh_adv::MeshAdvMaterialData;
use crossbeam_channel::{Receiver, Sender};
use fnv::FnvHashMap;
use hydrate_base::handle::Handle;
use hydrate_base::LoadHandle;
use rafx::api::RafxResult;
use rafx::assets::RafxResourceAssetLoader;
use rafx::assets::{
    asset_type_handler, AssetLookup, AssetManager, AssetTypeHandler, DynAssetLookup, ImageAsset,
    LoadQueues, MaterialAsset, UploadAssetOp, UploadAssetOpResult,
};
use rafx::framework::RenderResources;
use rafx::render_feature_renderer_prelude::AssetResource;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::sync::Arc;
use type_uuid::*;

use super::MeshAdvAsset;

#[derive(Serialize, Deserialize, Clone)]
pub struct MeshMaterialAdvAssetDataLod {
    pub mesh: Handle<MeshAdvAsset>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "8a2f44ec-0911-478a-851a-f61bcf085459"]
pub struct MeshMaterialAdvAssetData {
    pub material_asset: Handle<MaterialAsset>,
    pub material_data: MeshAdvMaterialData,
    pub color_texture: Option<Handle<ImageAsset>>,
    pub metallic_roughness_texture: Option<Handle<ImageAsset>>,
    pub normal_texture: Option<Handle<ImageAsset>>,
    pub emissive_texture: Option<Handle<ImageAsset>>,
}

pub struct MeshMaterialAdvAssetInner {
    pub material_asset: MaterialAsset,
    pub material: MeshAdvMaterial,
}

#[derive(TypeUuid, Clone)]
#[uuid = "ff52550c-a599-4a27-820b-f6ee4caebd8a"]
pub struct MeshMaterialAdvAsset {
    pub inner: Arc<MeshMaterialAdvAssetInner>,
}

impl std::fmt::Debug for MeshMaterialAdvAsset {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("MeshMaterialAdvAsset")
            .field("material_data", &self.inner.material.data())
            .finish()
    }
}

impl MeshMaterialAdvAsset {
    pub fn material_data(&self) -> &MeshAdvMaterialData {
        self.inner.material.data()
    }

    pub fn material_asset(&self) -> &MaterialAsset {
        &self.inner.material_asset
    }
}

pub struct MeshAdvMaterialAssetTypeHandler {
    asset_lookup: AssetLookup<MeshMaterialAdvAsset>,
    load_queues: LoadQueues<MeshMaterialAdvAssetData, MeshMaterialAdvAsset>,
    material_upload_context: MaterialDBUploadQueueContext,

    material_upload_result_tx: Sender<MeshAdvMaterialAssetUploadOpResult>,
    material_upload_result_rx: Receiver<MeshAdvMaterialAssetUploadOpResult>,

    pending_asset_data: FnvHashMap<LoadHandle, Handle<MaterialAsset>>,
}

impl MeshAdvMaterialAssetTypeHandler {
    pub fn create(
        _asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        render_resources: &mut RenderResources,
    ) -> RafxResult<Box<dyn AssetTypeHandler>> {
        let load_queues = LoadQueues::<MeshMaterialAdvAssetData, MeshMaterialAdvAsset>::default();

        asset_resource
            .add_storage_with_loader::<MeshMaterialAdvAssetData, MeshMaterialAdvAsset, _>(
                Box::new(RafxResourceAssetLoader(load_queues.create_loader())),
            );

        let material_upload_queue = MaterialDBUploadQueue::new();
        let material_upload_context = material_upload_queue.material_upload_queue_context();

        render_resources.insert(material_upload_queue);

        let (material_upload_result_tx, material_upload_result_rx) = crossbeam_channel::unbounded();

        Ok(Box::new(Self {
            asset_lookup: AssetLookup::new(asset_resource.loader()),
            load_queues,
            material_upload_context,
            material_upload_result_tx,
            material_upload_result_rx,
            pending_asset_data: Default::default(),
        }))
    }
}

impl AssetTypeHandler for MeshAdvMaterialAssetTypeHandler {
    fn process_load_requests(
        &mut self,
        asset_manager: &mut AssetManager,
    ) -> RafxResult<()> {
        for request in self.load_queues.take_load_requests() {
            log::info!("Uploading MeshAdvMaterial {:?}", request.load_handle);
            let load_handle = request.load_handle.clone();

            let color_texture = request
                .asset
                .color_texture
                .map(|x| asset_manager.latest_asset(&x).map(|y| y.image_view.clone()))
                .flatten();
            let metallic_roughness_texture = request
                .asset
                .metallic_roughness_texture
                .map(|x| asset_manager.latest_asset(&x).map(|y| y.image_view.clone()))
                .flatten();
            let normal_texture = request
                .asset
                .normal_texture
                .map(|x| asset_manager.latest_asset(&x).map(|y| y.image_view.clone()))
                .flatten();
            let emissive_texture = request
                .asset
                .emissive_texture
                .map(|x| asset_manager.latest_asset(&x).map(|y| y.image_view.clone()))
                .flatten();

            let material_def = MeshAdvMaterialDef {
                data: request.asset.material_data,
                color_texture,
                metallic_roughness_texture,
                normal_texture,
                emissive_texture,
            };

            let op = UploadAssetOp::new(
                request.load_op,
                request.load_handle,
                request.result_tx,
                self.material_upload_result_tx.clone(),
            );

            self.material_upload_context
                .add_material(op, material_def)?;
            self.pending_asset_data
                .insert(load_handle, request.asset.material_asset);
        }

        let results: Vec<_> = self.material_upload_result_rx.try_iter().collect();
        for result in results {
            match result {
                MeshAdvMaterialAssetUploadOpResult::UploadComplete(
                    load_op,
                    result_tx,
                    material,
                ) => {
                    log::info!(
                        "Uploading MeshAdvMaterial {:?} complete",
                        load_op.load_handle()
                    );
                    let material_asset = self
                        .pending_asset_data
                        .remove(&load_op.load_handle())
                        .unwrap();

                    let material_asset =
                        asset_manager.latest_asset(&material_asset).unwrap().clone();

                    let loaded_asset = Ok(MeshMaterialAdvAsset {
                        //finish_load_buffer(asset_manager, allocation);
                        inner: Arc::new(MeshMaterialAdvAssetInner {
                            material_asset,
                            material,
                        }),
                    });
                    asset_type_handler::handle_load_result(
                        load_op,
                        loaded_asset,
                        &mut self.asset_lookup,
                        result_tx,
                    );
                }
                MeshAdvMaterialAssetUploadOpResult::UploadError(load_handle) => {
                    log::info!("Uploading MeshAdvMaterial {:?} failed", load_handle);
                    self.pending_asset_data.remove(&load_handle).unwrap();
                    // Don't need to do anything - the upload should have triggered an error on the load_op
                }
                MeshAdvMaterialAssetUploadOpResult::UploadDrop(load_handle) => {
                    log::info!("Uploading MeshAdvMaterial {:?} cancelled", load_handle);
                    self.pending_asset_data.remove(&load_handle).unwrap();
                    // Don't need to do anything - the upload should have triggered an error on the load_op
                }
            }
        }

        asset_type_handler::handle_commit_requests(&mut self.load_queues, &mut self.asset_lookup);
        asset_type_handler::handle_free_requests(&mut self.load_queues, &mut self.asset_lookup);
        Ok(())
    }

    fn asset_lookup(&self) -> &dyn DynAssetLookup {
        &self.asset_lookup
    }

    fn asset_type_id(&self) -> TypeId {
        TypeId::of::<MeshMaterialAdvAsset>()
    }
}

pub type MeshAdvMaterialAssetUploadOpResult =
    UploadAssetOpResult<MeshAdvMaterial, MeshMaterialAdvAsset>;

pub type MeshAdvMaterialAssetType = MeshAdvMaterialAssetTypeHandler;
