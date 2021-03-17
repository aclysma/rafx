use crate::components::{PositionComponent, SpriteComponent};
use crate::time::TimeState;
use glam::f32::Vec3;
use legion::{Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::ImageAsset;
use rafx::renderer::ViewportsResource;

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

            let alpha = 1.0;

            let position_component = PositionComponent { position };
            let sprite_component = SpriteComponent {
                render_node: None,
                visibility_node: None,
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
        resources: &Resources,
    ) {
        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();

            super::update_main_view(&*time_state, &mut *viewports_resource);
        }
    }
}
