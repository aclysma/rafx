use crate::assets::font::FontAsset;
use crate::components::{PositionComponent, SpriteComponent};
use crate::features::sprite::{SpriteRenderNode, SpriteRenderNodeSet};
use crate::features::text::TextResource;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase,
};
use crate::time::TimeState;
use crate::RenderOptions;
use glam::{Quat, Vec2, Vec3};
use legion;
use legion::{SystemBuilder, IntoQuery, Read, Resources, Schedule, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::ImageAsset;
use rafx::distill::loader::handle::Handle;
use rafx::nodes::{RenderPhaseMaskBuilder, RenderViewDepthRange};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet};
use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

pub(super) struct ManySpritesScene {
    sprite_count: usize,
    font: Handle<FontAsset>,
    schedule: Schedule,
    position: Transform,
    up: Vec3,
}

impl ManySpritesScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_2d();

        let sprite_image = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<ImageAsset, _>("textures/texture-tiny-rust.jpeg")
        };

        let font = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf")
        };

        let mut rng = rand::thread_rng();

        let tile_size = Vec2::splat(64.0);
        let map_size = Vec2::splat(320.0);

        let half_x = (map_size.x() / 2.0) as i32;
        let half_y = (map_size.y() / 2.0) as i32;

        let update_render_node_system = SystemBuilder::new("update_render_node")
            .read_resource::<TimeState>()
            .write_resource::<SpriteRenderNodeSet>()
            .with_query(<Read<SpriteComponent>>::query())
            .build(move |_, world, (time, sprite_render_node_set), queries| {
                profiling::scope!("update_render_node_system");
                for sprite in queries.iter_mut(world) {
                    let render_node : &mut SpriteRenderNode = sprite_render_node_set
                        .get_mut(&sprite.render_node)
                        .unwrap();
                    render_node.rotation *= Quat::from_rotation_z(time.previous_update_dt() * rand::random::<f32>());
                }
            });

        let schedule = Schedule::builder()
            .add_system(update_render_node_system)
            .build();

        let mut sprite_count = 0 as usize;

        for y in -half_y..half_y {
            for x in -half_x..half_x {
                let position = Vec2::new(x as f32, y as f32);
                let translation = (position * tile_size).extend(rng.gen::<f32>());
                let scale = Vec2::new(rng.gen::<f32>() * 2.0, rng.gen::<f32>() * 2.0);

                let tint = super::random_color(&mut rng);
                let alpha = f32::max(0.2, rng.gen::<f32>());

                let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
                let mut dynamic_visibility_node_set =
                    resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

                let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
                    position: translation,
                    scale,
                    rotation: Quat::from_rotation_ypr(
                        rng.gen::<f32>(),
                        rng.gen::<f32>(),
                        rng.gen::<f32>(),
                    ),
                    tint,
                    alpha,
                    image: sprite_image.clone(),
                });

                let aabb_info = DynamicAabbVisibilityNode {
                    handle: render_node.as_raw_generic_handle(),
                };

                let visibility_node = dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

                let position_component = PositionComponent {
                    position: translation,
                };
                let sprite_component = SpriteComponent {
                    render_node,
                    visibility_node,
                    alpha,
                    image: sprite_image.clone(),
                };

                world.extend((0..1).map(|_| (position_component, sprite_component.clone())));

                sprite_count += 1;
            }
        }

        const CAMERA_Z: f32 = 1000.0;

        let position = Transform {
            translation: Vec3::new(0., 0., CAMERA_Z),
            rotation: Quat::from_rotation_x(0.),
            scale: Vec3::new(1., 1., 1.),
        };

        let up = Vec3::new(0., 1., 0.);

        ManySpritesScene {
            sprite_count,
            schedule,
            font,
            position,
            up,
        }
    }

    #[profiling::function]
    fn update_main_view_2d(
        &mut self,
        time: &TimeState,
        viewports_resource: &mut ViewportsResource,
    ) {
        let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
            .add_render_phase::<DepthPrepassRenderPhase>()
            .add_render_phase::<OpaqueRenderPhase>()
            .add_render_phase::<TransparentRenderPhase>()
            .add_render_phase::<UiRenderPhase>()
            .build();

        // Round to a whole number

        self.position
            .rotate(Quat::from_rotation_z(time.previous_update_dt() * 0.75));
        self.position = self.position.mul_transform(Transform {
            translation: Vec3::new(1., 0., 0.) * CAMERA_SPEED * time.previous_update_dt(),
            rotation: Quat::from_rotation_x(0.),
            scale: Vec3::new(1., 1., 1.),
        });

        let mut eye = self.position.translation;

        let mut transform = Transform {
            translation: eye,
            rotation: Quat::from_rotation_x(0.),
            scale: Vec3::new(1., 1., 1.),
        };

        transform.rotate(Quat::from_rotation_z(time.previous_update_dt() / 2.0));

        self.up = transform.mul_vec3(self.up);

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

        let view = glam::Mat4::look_at_rh(eye, Vec3::new(0., 0., 0.), self.up);

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
}

#[derive(Clone, Copy)]
struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn rotate(
        &mut self,
        rotation: Quat,
    ) {
        self.rotation *= rotation;
    }

    pub fn mul_transform(
        &self,
        transform: Transform,
    ) -> Self {
        let translation = self.mul_vec3(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    pub fn mul_vec3(
        &self,
        mut value: Vec3,
    ) -> Vec3 {
        value = self.rotation * value;
        value = self.scale * value;
        value += self.translation;
        value
    }
}

impl super::TestScene for ManySpritesScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            self.update_main_view_2d(&time_state, &mut *viewports_resource);
        }

        self.schedule.execute(world, resources);

        {
            let mut text_resource = resources.get_mut::<TextResource>().unwrap();
            text_resource.add_text(
                format!("Sprite Count: {}", self.sprite_count),
                glam::Vec3::new(25.0, 25.0, 0.0),
                &self.font,
                40.0,
                glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
            );
        }
    }
}
