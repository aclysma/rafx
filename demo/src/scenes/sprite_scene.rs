use crate::asset_resource::AssetResource;
use crate::components::{PositionComponent, SpriteComponent};
use crate::features::sprite::{SpriteRenderNode, SpriteRenderNodeSet};
use glam::f32::Vec3;
use legion::{Resources, World};
use rafx::assets::ImageAsset;
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet};

pub(super) struct SpriteScene {}

impl SpriteScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let sprite_image = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            //asset_resource.load_asset_path::<ImageAsset, _>("textures/texture2.jpg")
            asset_resource.load_asset::<ImageAsset>("cad0eeb3-68e1-48a5-81b6-ba4a7e848f38".into())
        };

        for i in 0..105 {
            let position = Vec3::new(
                ((i / 7) * 50) as f32 - 350.0,
                ((i % 7) * 50) as f32 - 200.0,
                i as f32 * 1.0,
            );

            //let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
            let alpha = 1.0;

            let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
            let mut dynamic_visibility_node_set =
                resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

            let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
                position,
                alpha,
                image: sprite_image.clone(),
            });

            let aabb_info = DynamicAabbVisibilityNode {
                handle: render_node.as_raw_generic_handle(),
                // aabb bounds
            };

            // User calls functions to register visibility objects
            // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
            let visibility_node = dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

            let position_component = PositionComponent { position };
            let sprite_component = SpriteComponent {
                render_node,
                visibility_node,
                alpha,
                image: sprite_image.clone(),
            };

            world.extend((0..1).map(|_| (position_component, sprite_component.clone())));
        }

        SpriteScene {}
    }
}

impl super::TestScene for SpriteScene {
    fn update(
        &mut self,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }
}
