use crate::features::tile_layer::{TileLayerRenderFeature, TileLayerRenderNodeSet, TileLayerResource};
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::RafxResult;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, MaterialAsset, AssetManagerRenderResource};
use rafx::base::resource_map::ResourceMap;
use rafx::distill::loader::handle::Handle;
use rafx::framework::RenderResources;
use rafx::nodes::{ExtractJob, ExtractResources, RenderRegistryBuilder, RenderFeature};
use rafx::renderer::RendererPlugin;
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};

pub struct TileLayerStaticResources {
    pub tile_layer_material: Handle<MaterialAsset>,
}

pub struct TileLayerRendererPlugin;

impl RendererPlugin for TileLayerRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<TileLayerRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let tile_layer_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/tile_layer.material");

        asset_manager.wait_for_asset_to_load(
            &tile_layer_material,
            asset_resource,
            "tile_layer_material",
        )?;

        render_resources.insert(TileLayerStaticResources { tile_layer_material });

        render_resources.insert(TileLayerRenderNodeSet::default());
        render_resources.try_insert_default::<StaticVisibilityNodeSet>();
        render_resources.try_insert_default::<DynamicVisibilityNodeSet>();

        Ok(())
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(super::create_tile_layer_extract_job());
    }
}
