use crate::assets::ldtk::LdtkProjectAsset;
use crate::components::{SpriteComponent, TransformComponent, VisibilityComponent};
use crate::features::imgui::ImGuiRenderFeature;
use crate::features::sprite::{SpriteRenderFeature, SpriteRenderNode, SpriteRenderNodeSet};
use crate::features::text::TextRenderFeature;
use crate::features::tile_layer::{
    TileLayerRenderFeature, TileLayerRenderNodeSet, TileLayerResource,
};
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
use rafx::nodes::{RenderFeatureMaskBuilder, RenderPhaseMaskBuilder, RenderViewDepthRange};
use rafx::rafx_visibility::{DepthRange, OrthographicParameters, Projection};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{CullModel, EntityId, ViewFrustumArc, VisibilityRegion};

pub(super) struct SpriteScene {
    ldtk_handle: Handle<LdtkProjectAsset>,
    main_view_frustum: ViewFrustumArc,
}

impl SpriteScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_2d();

        let visibility_region = resources.get::<VisibilityRegion>().unwrap();

        let sprite_image = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<ImageAsset, _>("textures/texture2.jpg")
            //asset_resource.load_asset::<ImageAsset>("cad0eeb3-68e1-48a5-81b6-ba4a7e848f38".into())
        };

        let ldtk_handle = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<LdtkProjectAsset, _>("ldtk/example.ldtk")
            //asset_resource.load_asset::<LdtkProjectAsset>("e01f536b-0a05-4d14-81cd-f010d4a45e81".into())
        };

        for i in 0..100 {
            let position = Vec3::new(
                ((i / 5) * 100) as f32 + 900.0,
                ((i % 5) * 50) as f32 - 100.0,
                100.0 + (i % 10) as f32,
            );

            let alpha = ((i / 5) as f32 * 0.1).min(1.0);

            let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();

            let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
                tint: glam::Vec3::new(1.0, 1.0, 1.0),
                alpha,
                image: sprite_image.clone(),
            });

            let transform_component = TransformComponent {
                translation: position,
                scale: glam::Vec3::splat(0.125),
                rotation: glam::Quat::from_rotation_z(0.0),
            };

            let sprite_component = SpriteComponent {
                render_node: render_node.clone(),
            };

            let entity = world.push((transform_component.clone(), sprite_component));
            let mut entry = world.entry(entity).unwrap();
            entry.add_component(VisibilityComponent {
                handle: {
                    let handle = visibility_region.register_dynamic_object(
                        EntityId::from(entity),
                        CullModel::quad(800., 450.),
                    );
                    handle.set_transform(
                        transform_component.translation,
                        transform_component.rotation,
                        transform_component.scale,
                    );
                    handle.add_feature(render_node.as_raw_generic_handle());
                    handle
                },
            });
        }

        let main_view_frustum = visibility_region.register_view_frustum();

        SpriteScene {
            ldtk_handle,
            main_view_frustum,
        }
    }
}

impl super::TestScene for SpriteScene {
    fn update(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
    ) {
        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();

            update_main_view_2d(
                &*time_state,
                &mut self.main_view_frustum,
                &mut *viewports_resource,
            );
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
                    let visibility_region = resources.get::<VisibilityRegion>().unwrap();

                    tile_layer_resource.set_project(
                        &self.ldtk_handle,
                        &*asset_manager,
                        &mut *tile_layer_render_nodes,
                        &visibility_region,
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
    main_view_frustum: &mut ViewFrustumArc,
    viewports_resource: &mut ViewportsResource,
) {
    let main_camera_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DepthPrepassRenderPhase>()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<UiRenderPhase>()
        .build();

    let main_camera_feature_mask = RenderFeatureMaskBuilder::default()
        .add_render_feature::<ImGuiRenderFeature>()
        .add_render_feature::<SpriteRenderFeature>()
        .add_render_feature::<TextRenderFeature>()
        .add_render_feature::<TileLayerRenderFeature>()
        .build();

    const CAMERA_XY_DISTANCE: f32 = 400.0;
    const CAMERA_Z: f32 = 1000.0;
    const CAMERA_ROTATE_SPEED: f32 = -0.15;
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
        eye.x = eye.x.round() + 0.5;
    } else {
        eye.x = eye.x.round();
    }

    if viewports_resource.main_window_size.height % 2 != 0 {
        eye.y = eye.y.round() + 0.5;
    } else {
        eye.y = eye.y.round();
    }

    let look_at = eye.truncate().extend(0.0);
    let up = glam::Vec3::new(0.0, 1.0, 0.0);

    let view = glam::Mat4::look_at_rh(eye, look_at, up);

    let near = 0.01;
    let far = 2000.0;

    let projection = Projection::Orthographic(OrthographicParameters::new(
        -half_width,
        half_width,
        -half_height,
        half_height,
        near,
        far,
        DepthRange::InfiniteReverse,
    ));

    main_view_frustum
        .set_projection(&projection)
        .set_transform(eye, look_at, up);

    viewports_resource.main_view_meta = Some(RenderViewMeta {
        view_frustum: main_view_frustum.clone(),
        eye_position: eye,
        view,
        proj: projection.as_rh_mat4(),
        depth_range: RenderViewDepthRange::from_projection(&projection),
        render_phase_mask: main_camera_phase_mask,
        render_feature_mask: main_camera_feature_mask,
        debug_name: "main".to_string(),
    });
}
