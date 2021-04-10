use rafx::render_feature_renderer_prelude::*;

use super::{
    create_font_atlas_image_view, init_sdl2_imgui_manager, ImGuiExtractJob, RenderFeatureType,
    Sdl2ImguiManager,
};
use distill::loader::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::framework::{ImageViewResource, ResourceArc};

pub struct ImGuiStaticResources {
    pub imgui_material: Handle<MaterialAsset>,
    pub imgui_font_atlas_image_view: ResourceArc<ImageViewResource>,
}

#[derive(Default)]
pub struct ImGuiRendererPlugin;

impl ImGuiRendererPlugin {
    pub fn legion_init(
        resources: &mut legion::Resources,
        window: &sdl2::video::Window,
    ) {
        let imgui_manager = init_sdl2_imgui_manager(window);
        resources.insert(imgui_manager);
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<Sdl2ImguiManager>();
    }
}

impl RendererPlugin for ImGuiRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<RenderFeatureType>()
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

        let imgui_font_atlas_data = extract_resources
            .fetch::<Sdl2ImguiManager>()
            .build_font_atlas();

        let dyn_resource_allocator = asset_manager.create_dyn_resource_allocator_set();
        let imgui_font_atlas_image_view = create_font_atlas_image_view(
            imgui_font_atlas_data,
            asset_manager.device_context(),
            upload,
            &dyn_resource_allocator,
        )?;

        render_resources.insert(ImGuiStaticResources {
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
        extract_jobs.push(Box::new(ImGuiExtractJob::new()));
    }
}
