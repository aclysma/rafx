use crate::features::imgui::ImGuiRenderFeature;
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::RafxResult;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, MaterialAsset};
use rafx::base::resource_map::ResourceMap;
use rafx::distill::loader::handle::Handle;
use rafx::framework::{ImageViewResource, RenderResources, ResourceArc};
use rafx::nodes::{ExtractJob, ExtractResources, RenderRegistryBuilder};
use rafx::renderer::RendererPlugin;

pub struct ImguiStaticResources {
    pub imgui_material: Handle<MaterialAsset>,
    pub imgui_font_atlas_image_view: ResourceArc<ImageViewResource>,
}

#[derive(Default)]
pub struct ImguiRendererPlugin;

impl RendererPlugin for ImguiRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<ImGuiRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let imgui_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/imgui.material");

        asset_manager.wait_for_asset_to_load(&imgui_material, asset_resource, "imgui material")?;

        use crate::features::imgui::Sdl2ImguiManager;

        let imgui_font_atlas_data = extract_resources
            .fetch::<Sdl2ImguiManager>()
            .build_font_atlas();

        let dyn_resource_allocator = asset_manager.create_dyn_resource_allocator_set();
        let imgui_font_atlas_image_view = super::imgui_font_atlas::create_font_atlas_image_view(
            imgui_font_atlas_data,
            asset_manager.device_context(),
            upload,
            &dyn_resource_allocator,
        )?;

        render_resources.insert(ImguiStaticResources {
            imgui_material,
            imgui_font_atlas_image_view,
        });

        Ok(())
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(super::create_imgui_extract_job());
    }
}
