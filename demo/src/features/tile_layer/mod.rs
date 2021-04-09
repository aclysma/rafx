use rafx::render_feature_mod_prelude::*;
use rafx::render_feature_renderer_prelude::*;
rafx::declare_render_feature!(TileLayerRenderFeature, TILE_LAYER_FEATURE_INDEX);

mod extract;
use extract::*;
mod prepare;
use prepare::*;
mod write;
use write::*;
mod public;
pub use public::*;

use distill::loader::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};

struct StaticResources {
    pub tile_layer_material: Handle<MaterialAsset>,
}

pub struct TileLayerRendererPlugin;

impl TileLayerRendererPlugin {
    pub fn legion_init(resources: &mut legion::Resources) {
        resources.insert(TileLayerRenderNodeSet::default());
        resources.insert(TileLayerResource::default());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<TileLayerRenderNodeSet>();
        resources.remove::<TileLayerResource>();
    }
}

impl RendererPlugin for TileLayerRendererPlugin {
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

        render_resources.insert(StaticResources {
            tile_layer_material,
        });

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
        extract_jobs.push(Box::new(ExtractJobImpl::new()));
    }
}
