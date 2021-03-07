use crate::features::sprite::{SpriteRenderFeature, SpriteRenderNodeSet};
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::RafxResult;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, MaterialAsset};
use rafx::base::resource_map::ResourceMap;
use rafx::distill::loader::handle::Handle;
use rafx::framework::RenderResources;
use rafx::nodes::{ExtractJob, ExtractResources, RenderNodeReservations, RenderRegistryBuilder};
use rafx::renderer::RendererPlugin;

pub struct SpriteStaticResources {
    pub sprite_material: Handle<MaterialAsset>,
}

pub struct SpriteRendererPlugin;

impl RendererPlugin for SpriteRendererPlugin {
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

        render_resources.insert(SpriteStaticResources { sprite_material });

        Ok(())
    }

    fn add_render_node_reservations(
        &self,
        render_node_reservations: &mut RenderNodeReservations,
        extract_resources: &ExtractResources,
    ) {
        let mut sprite_render_nodes = extract_resources.fetch_mut::<SpriteRenderNodeSet>();
        sprite_render_nodes.update();
        render_node_reservations.add_reservation(&*sprite_render_nodes);
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(super::create_sprite_extract_job());
    }
}
