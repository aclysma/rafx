use rafx::render_feature_renderer_prelude::*;

use super::{SkyboxExtractJob, SkyboxRenderFeature};
use distill::loader::handle::Handle;
use rafx::assets::{ImageAsset, MaterialAsset};

pub struct SkyboxStaticResources {
    pub skybox_material: Handle<MaterialAsset>,
    pub skybox_texture: Handle<ImageAsset>,
}

pub struct SkyboxRendererPlugin;

impl SkyboxRendererPlugin {
    pub fn legion_init(_resources: &mut legion::Resources) {}

    pub fn legion_destroy(_resources: &mut legion::Resources) {}
}

impl RendererPlugin for SkyboxRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<SkyboxRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let skybox_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/skybox.material");

        let skybox_texture =
            asset_resource.load_asset_path::<ImageAsset, _>("textures/skybox.basis");

        asset_manager.wait_for_asset_to_load(
            &skybox_material,
            asset_resource,
            "skybox material",
        )?;

        asset_manager.wait_for_asset_to_load(&skybox_texture, asset_resource, "skybox texture")?;

        render_resources.insert(SkyboxStaticResources {
            skybox_material,
            skybox_texture,
        });

        Ok(())
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(Box::new(SkyboxExtractJob::new()));
    }
}
