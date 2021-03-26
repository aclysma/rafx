use crate::components::{PositionComponent, SpriteComponent};
use crate::features::sprite::{SpriteRenderNode, SpriteRenderNodeSet};
use crate::time::TimeState;
use glam::f32::Vec3;
use legion::{Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{ImageAsset, AssetManager};
use rafx::renderer::ViewportsResource;
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use crate::assets::ldtk::LdtkProjectAsset;
use rafx::distill::loader::handle::Handle;
use crate::features::tile_layer::{TileLayerResource, TileLayerRenderNodeSet};
use crate::RenderOptions;

pub(super) struct SpriteScene {
    ldtk_handle: Handle<LdtkProjectAsset>,
}

impl SpriteScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_2d();


        let sprite_image = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            //asset_resource.load_asset_path::<ImageAsset, _>("textures/texture2.jpg")
            asset_resource.load_asset::<ImageAsset>("cad0eeb3-68e1-48a5-81b6-ba4a7e848f38".into())
        };

        let ldtk_handle = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<LdtkProjectAsset, _>("ldtk/example.ldtk")
            //asset_resource.load_asset::<LdtkProjectAsset>("e01f536b-0a05-4d14-81cd-f010d4a45e81".into())
        };

        // for i in 0..105 {
        //     let position = Vec3::new(
        //         ((i / 7) * 50) as f32 - 350.0,
        //         ((i % 7) * 50) as f32 - 200.0,
        //         i as f32 * 1.0,
        //     );
        //
        //     //let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
        //     let alpha = 1.0;
        //
        //     let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
        //     let mut dynamic_visibility_node_set =
        //         resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();
        //
        //     let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
        //         position,
        //         alpha,
        //         image: sprite_image.clone(),
        //     });
        //
        //     let aabb_info = DynamicAabbVisibilityNode {
        //         handle: render_node.as_raw_generic_handle(),
        //         // aabb bounds
        //     };
        //
        //     // User calls functions to register visibility objects
        //     // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
        //     let visibility_node = dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);
        //
        //     let position_component = PositionComponent { position };
        //     let sprite_component = SpriteComponent {
        //         render_node,
        //         visibility_node,
        //         alpha,
        //         image: sprite_image.clone(),
        //     };
        //
        //     world.extend((0..1).map(|_| (position_component, sprite_component.clone())));
        // }

        SpriteScene {
            ldtk_handle
        }
    }
}

impl super::TestScene for SpriteScene {
    fn update(
        &mut self,
        _world: &mut World,
        resources: &Resources,
    ) {
        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();

            super::update_main_view_2d(&*time_state, &mut *viewports_resource);
        }

        // Wait until we have loaded the tileset. If we have, then set it.
        //TODO: This is not a great way to do this, just testing
        {
            let mut tile_layer_resource = resources.get_mut::<TileLayerResource>().unwrap();
            if tile_layer_resource.project().is_none() || tile_layer_resource.project().clone().unwrap() != self.ldtk_handle {
                let asset_manager = resources.get::<AssetManager>().unwrap();
                let asset = asset_manager.committed_asset(&self.ldtk_handle);

                if asset.is_some() {
                    let mut tile_layer_render_nodes = resources.get_mut::<TileLayerRenderNodeSet>().unwrap();
                    let mut static_visibility_nodes = resources.get_mut::<StaticVisibilityNodeSet>().unwrap();

                    tile_layer_resource.set_project(
                        &self.ldtk_handle,
                        &*asset_manager,
                        &mut *tile_layer_render_nodes,
                        &mut *static_visibility_nodes,
                    );
                }
            }
        }
    }

    fn cleanup(
        &mut self,
        _world: &mut World,
        resources: &Resources,
    ) {
        let mut tile_layer_resource = resources.get_mut::<TileLayerResource>().unwrap();
        tile_layer_resource.clear_project();
    }
}
