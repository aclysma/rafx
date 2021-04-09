use rafx::render_feature_mod_prelude::*;
use rafx::render_feature_renderer_prelude::*;

use distill::loader::handle::Handle;
use extract::ExtractJobImpl;
use rafx::assets::MaterialAsset;

rafx::declare_render_feature!(SpriteRenderFeature, SPRITE_FEATURE_INDEX);

mod extract;
mod prepare;
mod write;

mod public;
pub use public::*;

struct StaticResources {
    pub sprite_material: Handle<MaterialAsset>,
}

pub struct RendererPluginImpl;

impl RendererPlugin for RendererPluginImpl {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<SpriteRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let sprite_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/sprite.material");

        asset_manager.wait_for_asset_to_load(
            &sprite_material,
            asset_resource,
            "sprite_material",
        )?;

        render_resources.insert(StaticResources { sprite_material });

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
    resources.insert(SpriteRenderNodeSet::default());
}

pub fn legion_destroy(resources: &mut Resources) {
    resources.remove::<SpriteRenderNodeSet>();
}
