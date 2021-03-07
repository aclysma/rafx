use crate::assets::gltf::MeshAssetType;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::renderer::RendererPlugin;

pub struct GltfAssetTypeRendererPlugin;

impl RendererPlugin for GltfAssetTypeRendererPlugin {
    //
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon
            .with_importer("gltf", super::GltfImporter)
            .with_importer("glb", super::GltfImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) {
        asset_manager.register_asset_type::<MeshAssetType>(asset_resource);
    }
}
