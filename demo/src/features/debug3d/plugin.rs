use rafx::render_feature_renderer_prelude::*;

use super::{Debug3DExtractJob, Debug3DRenderFeature, DebugDraw3DResource};
use distill::loader::handle::Handle;
use rafx::assets::MaterialAsset;

pub struct Debug3DStaticResources {
    pub debug3d_material: Handle<MaterialAsset>,
}

pub struct Debug3DRendererPlugin;

impl Debug3DRendererPlugin {
    pub fn legion_init(resources: &mut legion::Resources) {
        resources.insert(DebugDraw3DResource::new());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<DebugDraw3DResource>();
    }
}

impl RendererPlugin for Debug3DRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<Debug3DRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let debug3d_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/debug.material");

        asset_manager
            .wait_for_asset_to_load(&debug3d_material, asset_resource, "debug.material")
            .unwrap();

        render_resources.insert(Debug3DStaticResources { debug3d_material });

        Ok(())
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(Box::new(Debug3DExtractJob::new()));
    }
}
