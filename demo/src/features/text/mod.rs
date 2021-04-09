rafx::declare_render_feature_mod!();
rafx::declare_render_feature_renderer_plugin!();

rafx::declare_render_feature!(TextRenderFeature, TEXT_FEATURE_INDEX);

mod extract;
mod prepare;
mod write;

mod internal;
mod public;

use crate::assets::font::FontAsset;
use distill::loader::handle::Handle;
use internal::FontAtlasCache;
use rafx::assets::MaterialAsset;

pub use public::AppendText;
pub use public::TextResource;

struct StaticResources {
    pub text_material: Handle<MaterialAsset>,
    pub default_font: Handle<FontAsset>,
}

pub struct RendererPluginImpl;

impl RendererPlugin for RendererPluginImpl {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<TextRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let text_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/text.material");
        let default_font =
            asset_resource.load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf");

        asset_manager.wait_for_asset_to_load(&text_material, asset_resource, "text material")?;

        asset_manager.wait_for_asset_to_load(&default_font, asset_resource, "default font")?;

        render_resources.insert(StaticResources {
            text_material,
            default_font,
        });

        render_resources.insert(FontAtlasCache::default());

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
    resources.insert(TextResource::new());
}

pub fn legion_destroy(resources: &mut Resources) {
    resources.remove::<TextResource>();
}
