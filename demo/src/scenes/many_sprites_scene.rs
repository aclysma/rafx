// NOTE(dvd): Inspired by Bevy `many_sprites` example (MIT licensed) https://github.com/bevyengine/bevy/blob/621cba4864fd5d2c0962151b126769eff45797fd/examples/2d/many_sprites.rs

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
use legion::{IntoQuery, Read, Resources, Schedule, SystemBuilder, World, Write};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::ImageAsset;
use rafx::distill::loader::handle::Handle;
use rafx::nodes::{RenderPhaseMaskBuilder, RenderViewDepthRange};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet};
use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

#[derive(Clone, Copy)]
struct CameraComponent {
    position: TransformComponent,
    up: Vec3,
}

#[derive(Clone)]
struct TextComponent {
    text: String,
    font: Handle<FontAsset>,
}

#[derive(Clone, Copy)]
struct TransformComponent {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl TransformComponent {
    pub fn rotate(
        &mut self,
        rotation: Quat,
    ) {
        self.rotation *= rotation;
    }

    pub fn mul_transform(
        &self,
        transform: TransformComponent,
    ) -> Self {
        let translation = self.mul_vec3(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        TransformComponent {
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

pub(super) struct ManySpritesScene {
    schedule: Schedule,
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

        let update_camera_system = SystemBuilder::new("update_camera")
            .read_resource::<TimeState>()
            .write_resource::<ViewportsResource>()
            .with_query(<Write<CameraComponent>>::query())
            .build(move |_, world, (time_state, viewports_resource), queries| {
                profiling::scope!("update_camera_system");
                for camera in queries.iter_mut(world) {
                    update_main_view_2d(camera, time_state, viewports_resource);
                }
            });

        let update_render_node_system = SystemBuilder::new("update_render_node")
            .read_resource::<TimeState>()
            .write_resource::<SpriteRenderNodeSet>()
            .with_query(<Read<SpriteComponent>>::query())
            .build(move |_, world, (time, sprite_render_node_set), queries| {
                profiling::scope!("update_render_node_system");
                for sprite in queries.iter_mut(world) {
                    let render_node: &mut SpriteRenderNode =
                        sprite_render_node_set.get_mut(&sprite.render_node).unwrap();
                    render_node.rotation *=
                        Quat::from_rotation_z(time.previous_update_dt() * rand::random::<f32>());
                }
            });

        let print_text_system = SystemBuilder::new("print_text")
            .write_resource::<TextResource>()
            .with_query(<Read<TextComponent>>::query())
            .build(move |_, world, text_resource, queries| {
                profiling::scope!("print_text_system");
                for text in queries.iter_mut(world) {
                    text_resource.add_text(
                        text.text.clone(),
                        glam::Vec3::new(25.0, 25.0, 0.0),
                        &text.font,
                        40.0,
                        glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
                    );
                }
            });

        let schedule = Schedule::builder()
            .add_system(update_camera_system)
            .add_system(update_render_node_system)
            .add_system(print_text_system)
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

        world.push((CameraComponent {
            position: TransformComponent {
                translation: Vec3::new(0., 0., CAMERA_Z),
                rotation: Quat::from_rotation_x(0.),
                scale: Vec3::new(1., 1., 1.),
            },
            up: Vec3::new(0., 1., 0.),
        },));

        world.push((TextComponent {
            text: format!("Sprite Count: {}", sprite_count),
            font,
        },));

        ManySpritesScene { schedule }
    }
}

impl super::TestScene for ManySpritesScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        self.schedule.execute(world, resources);
    }
}

#[profiling::function]
fn update_main_view_2d(
    camera: &mut CameraComponent,
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

    camera
        .position
        .rotate(Quat::from_rotation_z(time.previous_update_dt() * 0.75));
    camera.position = camera.position.mul_transform(TransformComponent {
        translation: Vec3::new(1., 0., 0.) * CAMERA_SPEED * time.previous_update_dt(),
        rotation: Quat::from_rotation_x(0.),
        scale: Vec3::new(1., 1., 1.),
    });

    let mut eye = camera.position.translation;

    let mut transform = TransformComponent {
        translation: eye,
        rotation: Quat::from_rotation_x(0.),
        scale: Vec3::new(1., 1., 1.),
    };

    transform.rotate(Quat::from_rotation_z(time.previous_update_dt() / 2.0));

    camera.up = transform.mul_vec3(camera.up);

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

    let view = glam::Mat4::look_at_rh(eye, Vec3::new(0., 0., 0.), camera.up);

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
