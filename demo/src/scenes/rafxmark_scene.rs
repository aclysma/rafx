// NOTE(dvd): Inspired by Bevy `bevymark` example (MIT licensed) https://github.com/bevyengine/bevy/blob/81b53d15d4e038261182b8d7c8f65f9a3641fd2d/examples/tools/bevymark.rs

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
use legion::systems::CommandBuffer;
use legion::{IntoQuery, Read, Resources, Schedule, SystemBuilder, World, Write};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::ImageAsset;
use rafx::distill::loader::handle::Handle;
use rafx::nodes::{RenderPhaseMaskBuilder, RenderViewDepthRange};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet};
use sdl2::event::Event;
use sdl2::mouse::MouseButton;

const SPRITES_PER_SECOND: u32 = 1000;
const GRAVITY: f32 = -9.8 * 100.0;
const MAX_VELOCITY: f32 = 750.;
const SPRITE_SCALE: f32 = 1.0;
const HALF_SPRITE_SIZE: f32 = 64. * SPRITE_SCALE * 0.5;

const TOP: f32 = 250.;
const LEFT: f32 = -450.;
const BOTTOM: f32 = -250.;
const RIGHT: f32 = 450.;

#[derive(Copy, Clone)]
struct BodyComponent {
    velocity: Vec3,
}

#[derive(Clone)]
struct TextComponent {
    text: String,
    font: Handle<FontAsset>,
}

#[derive(Clone, Copy)]
struct InputComponent {
    is_left_mouse_button_down: bool,
}

#[derive(Clone)]
struct SpriteSpawnerComponent {
    sprite_count: usize,
    sprite_image: Handle<ImageAsset>,
}

pub(super) struct RafxmarkScene {
    schedule: Schedule,
}

impl RafxmarkScene {
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

        let update_camera_system = SystemBuilder::new("update_camera")
            .write_resource::<ViewportsResource>()
            .build(move |_, _, viewports_resource, _| {
                profiling::scope!("update_camera_system");
                update_main_view_2d(viewports_resource);
            });

        let sprite_spawner_system = SystemBuilder::new("sprite_spawner")
            .read_resource::<TimeState>()
            .write_resource::<SpriteRenderNodeSet>()
            .write_resource::<DynamicVisibilityNodeSet>()
            .with_query(<(
                Read<InputComponent>,
                Write<SpriteSpawnerComponent>,
                Write<TextComponent>,
            )>::query())
            .build(
                move |commands,
                      world,
                      (time, sprite_render_nodes, dynamic_visibility_node_set),
                      queries| {
                    profiling::scope!("sprite_spawner_system");
                    for (input, sprite_spawner, sprite_count_text) in queries.iter_mut(world) {
                        if input.is_left_mouse_button_down {
                            add_sprites(
                                sprite_spawner,
                                commands,
                                time,
                                sprite_render_nodes,
                                dynamic_visibility_node_set,
                            );
                        }

                        sprite_count_text.text =
                            format!("Sprite Count: {}", sprite_spawner.sprite_count);
                    }
                },
            );

        let gravity_system = SystemBuilder::new("gravity")
            .read_resource::<TimeState>()
            .with_query(<Write<BodyComponent>>::query())
            .build(move |_, world, time, queries| {
                profiling::scope!("gravity_system");
                for body in queries.iter_mut(world) {
                    body.velocity.y = body.velocity.y + GRAVITY * time.previous_update_dt();
                }
            });

        let velocity_system = SystemBuilder::new("velocity")
            .read_resource::<TimeState>()
            .with_query(<(Write<PositionComponent>, Write<BodyComponent>)>::query())
            .build(move |_, world, time, queries| {
                profiling::scope!("velocity_system");
                for (pos, body) in queries.iter_mut(world) {
                    pos.position += body.velocity * time.previous_update_dt();
                }
            });

        let collision_system = SystemBuilder::new("collision")
            .with_query(<(Write<PositionComponent>, Write<BodyComponent>)>::query())
            .build(move |_, world, (), queries| {
                profiling::scope!("collision_system");
                for (pos, body) in queries.iter_mut(world) {
                    if pos.position.x < LEFT {
                        pos.position.x = LEFT;
                        body.velocity.x = -body.velocity.x;
                    } else if pos.position.x > RIGHT {
                        pos.position.x = RIGHT;
                        body.velocity.x = -body.velocity.x;
                    }

                    if pos.position.y > TOP {
                        pos.position.y = TOP;
                        body.velocity.y = -body.velocity.y;
                    } else if pos.position.y < BOTTOM {
                        pos.position.y = BOTTOM;
                        body.velocity.y = -body.velocity.y;
                    }
                }
            });

        let update_render_node_system = SystemBuilder::new("update_render_node")
            .write_resource::<SpriteRenderNodeSet>()
            .with_query(<(Write<PositionComponent>, Read<SpriteComponent>)>::query())
            .build(move |_, world, sprite_render_node_set, queries| {
                profiling::scope!("update_render_node_system");
                for (pos, sprite) in queries.iter_mut(world) {
                    sprite_render_node_set
                        .get_mut(&sprite.render_node)
                        .unwrap()
                        .position = pos.position;
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
                        glam::Vec4::new(1.0, 1.0, 1.0, 1.0),
                    );
                }
            });

        let schedule = Schedule::builder()
            .add_system(update_camera_system)
            .add_system(sprite_spawner_system)
            .add_system(gravity_system)
            .add_system(velocity_system)
            .add_system(collision_system)
            .add_system(update_render_node_system)
            .add_system(print_text_system)
            .build();

        world.push((
            TextComponent {
                text: "".to_string(),
                font,
            },
            InputComponent {
                is_left_mouse_button_down: false,
            },
            SpriteSpawnerComponent {
                sprite_count: 0,
                sprite_image,
            },
        ));

        RafxmarkScene { schedule }
    }
}

impl super::TestScene for RafxmarkScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        self.schedule.execute(world, resources);
    }

    fn process_input(
        &mut self,
        world: &mut World,
        _resources: &Resources,
        event: Event,
    ) {
        let mut query = <Write<InputComponent>>::query();
        let input = query.iter_mut(world).last().unwrap();
        match event {
            Event::MouseButtonDown { mouse_btn, .. } => {
                if mouse_btn == MouseButton::Left {
                    input.is_left_mouse_button_down = true;
                }
            }
            Event::MouseButtonUp { mouse_btn, .. } => {
                if mouse_btn == MouseButton::Left {
                    input.is_left_mouse_button_down = false;
                }
            }
            _ => {}
        }
    }
}

fn add_sprites(
    sprite_spawner: &mut SpriteSpawnerComponent,
    commands: &mut CommandBuffer,
    time: &TimeState,
    sprite_render_nodes: &mut SpriteRenderNodeSet,
    dynamic_visibility_node_set: &mut DynamicVisibilityNodeSet,
) {
    let spawn_count = (SPRITES_PER_SECOND as f32 * time.previous_update_dt()) as usize;

    let mut rng = rand::thread_rng();
    let tint = super::random_color(&mut rng);

    let sprite_x = LEFT + HALF_SPRITE_SIZE;
    let sprite_y = TOP - HALF_SPRITE_SIZE;

    for count in 0..spawn_count {
        let sprite_z = (sprite_spawner.sprite_count + count) as f32 * 0.00001;

        let velocity = Vec3::new(
            rand::random::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
            0.,
            0.,
        );

        let position = Vec3::new(sprite_x, sprite_y, 100.0 + sprite_z as f32);

        let alpha = 0.8;

        let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
            position,
            scale: Vec2::splat(SPRITE_SCALE),
            rotation: Quat::from_rotation_z(0.0),
            tint,
            alpha,
            image: sprite_spawner.sprite_image.clone(),
        });

        let aabb_info = DynamicAabbVisibilityNode {
            handle: render_node.as_raw_generic_handle(),
        };

        let visibility_node = dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

        let position_component = PositionComponent { position };
        let sprite_component = SpriteComponent {
            render_node,
            visibility_node,
            alpha,
            image: sprite_spawner.sprite_image.clone(),
        };
        let body_component = BodyComponent { velocity };

        commands.extend(
            (0..1).map(move |_| (position_component, sprite_component.clone(), body_component)),
        );
    }

    sprite_spawner.sprite_count += spawn_count;
}

#[profiling::function]
fn update_main_view_2d(viewports_resource: &mut ViewportsResource) {
    let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DepthPrepassRenderPhase>()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<UiRenderPhase>()
        .build();

    const CAMERA_Z: f32 = 1000.0;

    // Round to a whole number
    let mut eye = Vec3::new(0., 0., CAMERA_Z);

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

    let view = glam::Mat4::look_at_rh(eye, Vec3::new(0., 0., 0.), Vec3::new(0.0, 1.0, 0.0));

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
