use crate::assets::ldtk::LdtkProjectAsset;
use crate::components::{PositionComponent, SpriteComponent};
use crate::features::sprite::{SpriteRenderNode, SpriteRenderNodeSet};
use crate::features::tile_layer::{TileLayerRenderNodeSet, TileLayerResource};
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase,
};
use crate::time::TimeState;
use crate::RenderOptions;
use glam::f32::Vec3;
use legion::{Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, ImageAsset};
use rafx::distill::loader::handle::Handle;
use rafx::nodes::{RenderPhaseMaskBuilder, RenderViewDepthRange};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{
    DynamicAabbVisibilityNode, DynamicVisibilityNodeSet, StaticVisibilityNodeSet,
};

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

        for i in 0..1000000 {
            let position = Vec3::new(
                ((i / 1000) * 4) as f32 + 700.0,
                ((i % 1000) * 4) as f32 - 300.0,
                100.0,
            );

            //let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
            let alpha = 0.5;

            let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
            let mut dynamic_visibility_node_set =
                resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

            let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
                position,
                alpha,
                scale: 0.125,
                rotation: 0.0,
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

        SpriteScene { ldtk_handle }
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

            update_main_view_2d(&*time_state, &mut *viewports_resource);
        }

        // Wait until we have loaded the tileset. If we have, then set it.
        //TODO: This is not a great way to do this, just testing
        {
            let mut tile_layer_resource = resources.get_mut::<TileLayerResource>().unwrap();
            if tile_layer_resource.project().is_none()
                || tile_layer_resource.project().clone().unwrap() != self.ldtk_handle
            {
                let asset_manager = resources.get::<AssetManager>().unwrap();
                let asset = asset_manager.committed_asset(&self.ldtk_handle);

                if asset.is_some() {
                    let mut tile_layer_render_nodes =
                        resources.get_mut::<TileLayerRenderNodeSet>().unwrap();
                    let mut static_visibility_nodes =
                        resources.get_mut::<StaticVisibilityNodeSet>().unwrap();

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

#[profiling::function]
fn update_main_view_2d(
    time_state: &TimeState,
    viewports_resource: &mut ViewportsResource,
) {
    let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DepthPrepassRenderPhase>()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<UiRenderPhase>()
        .build();

    const CAMERA_XY_DISTANCE: f32 = 400.0;
    const CAMERA_Z: f32 = 1000.0;
    const CAMERA_ROTATE_SPEED: f32 = -0.20;
    const CAMERA_LOOP_OFFSET: f32 = 0.7;
    let loop_time = time_state.total_time().as_secs_f32();

    // Round to a whole number
    let mut eye = glam::Vec3::new(
        (CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET))
            + 1000.0,
        (CAMERA_XY_DISTANCE * f32::sin(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET))
            - 200.0,
        CAMERA_Z,
    );

    let half_width = viewports_resource.main_window_size.width as f32 / 2.0;
    let half_height = viewports_resource.main_window_size.height as f32 / 2.0;

    //
    // This logic ensures pixel-perfect rendering when we have odd-sized width/height dimensions.
    // We also need to round x/y to whole numbers to render pixel-perfect
    //
    if viewports_resource.main_window_size.width % 2 != 0 {
        eye.set_x(eye.x().round() + 0.5);
    } else {
        eye.set_x(eye.x().round());
    }

    if viewports_resource.main_window_size.height % 2 != 0 {
        eye.set_y(eye.y().round() + 0.5);
    } else {
        eye.set_y(eye.y().round());
    }

    let view = glam::Mat4::look_at_rh(
        eye,
        eye.truncate().extend(0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );

    let proj = glam::Mat4::orthographic_rh(
        -half_width,
        half_width,
        -half_height,
        half_height,
        2000.0,
        0.0,
    );

    viewports_resource.main_view_meta = Some(RenderViewMeta {
        eye_position: eye,
        view,
        proj,
        depth_range: RenderViewDepthRange::new_infinite_reverse(0.0),
        render_phase_mask: main_camera_render_phase_mask,
        debug_name: "main".to_string(),
    });
}
