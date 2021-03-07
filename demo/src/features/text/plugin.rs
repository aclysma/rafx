use crate::assets::font::FontAsset;
use crate::features::text::{FontAtlasCache, TextRenderFeature};
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::RafxResult;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, MaterialAsset};
use rafx::base::resource_map::ResourceMap;
use rafx::distill::loader::handle::Handle;
use rafx::framework::RenderResources;
use rafx::nodes::{ExtractJob, ExtractResources, RenderRegistryBuilder};
use rafx::renderer::RendererPlugin;

pub struct TextStaticResources {
    pub text_material: Handle<MaterialAsset>,
    pub default_font: Handle<FontAsset>,
}

pub struct TextRendererPlugin;

impl RendererPlugin for TextRendererPlugin {
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

        render_resources.insert(TextStaticResources {
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
        extract_jobs.push(super::create_text_extract_job());
    }
}
