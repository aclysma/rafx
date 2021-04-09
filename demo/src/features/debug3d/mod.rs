use rafx::render_feature_mod_prelude::*;
use rafx::render_feature_renderer_prelude::*;

use distill::loader::handle::Handle;
use extract::ExtractJobImpl;
use rafx::assets::MaterialAsset;

rafx::declare_render_feature!(Debug3DRenderFeature, DEBUG_3D_FEATURE_INDEX);

mod extract;
mod prepare;
mod write;

mod public;
pub use public::debug3d_resource::DebugDraw3DResource;

struct StaticResources {
    pub debug3d_material: Handle<MaterialAsset>,
}

pub struct RendererPluginImpl;

impl RendererPlugin for RendererPluginImpl {
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

        render_resources.insert(StaticResources { debug3d_material });

        Ok(())
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(Box::new(ExtractJobImpl::new()));
    }
}

// Legion-specific

use legion::Resources;

pub fn legion_init(resources: &mut Resources) {
    resources.insert(DebugDraw3DResource::new());
}

pub fn legion_destroy(resources: &mut Resources) {
    resources.remove::<DebugDraw3DResource>();
}
