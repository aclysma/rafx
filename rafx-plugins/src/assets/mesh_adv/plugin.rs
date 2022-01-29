use crate::assets::mesh_adv::material_db::{MaterialDB, MaterialDBUploadQueue};
use crate::assets::mesh_adv::{
    MeshAdvAssetType, MeshAdvBufferAssetType, MeshAdvMaterialAssetType, ModelAdvAssetType,
    PrefabAdvAssetType,
};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::framework::RenderResources;
use rafx::render_feature_renderer_prelude::RafxTransferUpload;
use rafx::render_features::ExtractResources;
use rafx::renderer::{RendererAssetPlugin, RendererLoadContext};
use rafx::RafxResult;

pub struct MeshAdvAssetTypeRendererPlugin;

impl RendererAssetPlugin for MeshAdvAssetTypeRendererPlugin {
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon
            .with_importer(&["gltf"], super::MeshAdvGltfImporter)
            .with_importer(&["glb"], super::MeshAdvGltfImporter)
            .with_importer(&["blender_material"], super::MeshAdvBlenderMaterialImporter)
            .with_importer(&["blender_model"], super::MeshAdvBlenderModelImporter)
            .with_importer(&["blender_mesh"], super::MeshAdvBlenderImporter)
            .with_importer(&["blender_prefab"], super::MeshAdvBlenderPrefabImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        render_resources: &mut RenderResources,
    ) -> RafxResult<()> {
        let asset_type =
            MeshAdvMaterialAssetType::create(asset_manager, asset_resource, render_resources)?;
        asset_manager.register_asset_type(asset_type)?;
        let asset_type =
            MeshAdvBufferAssetType::create(asset_manager, asset_resource, render_resources)?;
        asset_manager.register_asset_type(asset_type)?;
        let asset_type = MeshAdvAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)?;
        let asset_type = ModelAdvAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)?;
        let asset_type = PrefabAdvAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)?;
        Ok(())
    }

    fn on_frame_complete(
        &self,
        _asset_manager: &mut AssetManager,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn initialize_static_resources(
        &self,
        _renderer_load_context: &RendererLoadContext,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut RenderResources,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        render_resources.insert(super::material_db::MaterialDB::new());
        Ok(())
    }

    fn process_asset_loading(
        &self,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        render_resources: &RenderResources,
    ) -> RafxResult<()> {
        let mut material_db = render_resources.fetch_mut::<MaterialDB>();
        let material_db_upload_queue = render_resources.fetch_mut::<MaterialDBUploadQueue>();
        material_db_upload_queue.update(&mut *material_db);
        material_db.update();
        Ok(())
    }

    fn prepare_renderer_destroy(
        &self,
        render_resources: &RenderResources,
    ) -> RafxResult<()> {
        let mut material_db = render_resources.fetch_mut::<MaterialDB>();
        material_db.destroy();
        Ok(())
    }
}
