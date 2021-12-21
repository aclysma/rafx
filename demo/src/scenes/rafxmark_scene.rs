// NOTE(dvd): Inspired by Bevy `bevymark` example (MIT licensed) https://github.com/bevyengine/bevy/blob/81b53d15d4e038261182b8d7c8f65f9a3641fd2d/examples/tools/bevymark.rs

use crate::time::TimeState;
use crate::RenderOptions;
use glam::Vec3;
use legion;
use legion::systems::CommandBuffer;
use legion::{IntoQuery, Read, Resources, Schedule, SystemBuilder, World, Write};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::ImageAsset;
use rafx::distill::loader::handle::Handle;
use rafx::rafx_visibility::{DepthRange, OrthographicParameters, Projection};
use rafx::render_features::RenderViewDepthRange;
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{CullModel, ObjectId, ViewFrustumArc, VisibilityRegion};
use rafx_plugins::assets::font::FontAsset;
use rafx_plugins::components::SpriteComponent;
use rafx_plugins::components::{TransformComponent, VisibilityComponent};
use rafx_plugins::features::sprite::{SpriteRenderObject, SpriteRenderObjectSet};
use rafx_plugins::features::text::TextResource;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};

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
    main_view_frustum: ViewFrustumArc,
}

impl RafxmarkScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_2d();

        let visibility_region = resources.get::<VisibilityRegion>().unwrap();
        let main_view_frustum = visibility_region.register_view_frustum();
        let mut main_view_frustum_copy = main_view_frustum.clone();

        let sprite_image = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<ImageAsset, _>("textures/texture-tiny-rust.jpeg")
        };

        let font = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf")
        };

        let update_camera_system = SystemBuilder::new("update_camera")
            .read_resource::<RenderOptions>()
            .write_resource::<ViewportsResource>()
            .build(move |_, _, (render_options, viewports_resource), _| {
                profiling::scope!("update_camera_system");
                update_main_view_2d(
                    &*render_options,
                    &mut main_view_frustum_copy,
                    viewports_resource,
                );
            });

        let sprite_spawner_system = SystemBuilder::new("sprite_spawner")
            .read_resource::<TimeState>()
            .read_resource::<VisibilityRegion>()
            .write_resource::<SpriteRenderObjectSet>()
            .with_query(<(
                Read<InputComponent>,
                Write<SpriteSpawnerComponent>,
                Write<TextComponent>,
            )>::query())
            .build(
                move |commands, world, (time, visibility_region, sprite_render_nodes), queries| {
                    profiling::scope!("sprite_spawner_system");
                    for (input, sprite_spawner, sprite_count_text) in queries.iter_mut(world) {
                        if input.is_left_mouse_button_down {
                            add_sprites(
                                sprite_spawner,
                                commands,
                                time,
                                sprite_render_nodes,
                                visibility_region,
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
            .with_query(<(Write<TransformComponent>, Write<BodyComponent>)>::query())
            .build(move |_, world, time, queries| {
                profiling::scope!("velocity_system");
                for (transform, body) in queries.iter_mut(world) {
                    transform.translation += body.velocity * time.previous_update_dt();
                }
            });

        let collision_system = SystemBuilder::new("collision")
            .with_query(<(
                Write<TransformComponent>,
                Write<BodyComponent>,
                Read<VisibilityComponent>,
            )>::query())
            .build(move |_, world, (), queries| {
                profiling::scope!("collision_system");
                for (transform, body, visibility) in queries.iter_mut(world) {
                    if transform.translation.x < LEFT {
                        transform.translation.x = LEFT;
                        body.velocity.x = -body.velocity.x;
                    } else if transform.translation.x > RIGHT {
                        transform.translation.x = RIGHT;
                        body.velocity.x = -body.velocity.x;
                    }

                    if transform.translation.y > TOP {
                        transform.translation.y = TOP;
                        body.velocity.y = -body.velocity.y;
                    } else if transform.translation.y < BOTTOM {
                        transform.translation.y = BOTTOM;
                        body.velocity.y = -body.velocity.y;
                    }

                    visibility.visibility_object_handle.set_transform(
                        transform.translation,
                        transform.rotation,
                        transform.scale,
                    );
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

        RafxmarkScene {
            schedule,
            main_view_frustum,
        }
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
        event: &Event<()>,
    ) {
        let mut query = <Write<InputComponent>>::query();
        let input = query.iter_mut(world).last().unwrap();
        match event {
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                if *button == MouseButton::Left {
                    input.is_left_mouse_button_down = state.eq(&ElementState::Pressed)
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
    sprite_render_objects: &mut SpriteRenderObjectSet,
    visibility_region: &VisibilityRegion,
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

        let sprite_render_object =
            sprite_render_objects.register_render_object(SpriteRenderObject {
                tint,
                alpha,
                image: sprite_spawner.sprite_image.clone(),
            });

        let transform_component = TransformComponent {
            translation: position,
            scale: glam::Vec3::splat(SPRITE_SCALE),
            rotation: glam::Quat::from_rotation_z(0.0),
        };

        let sprite_component = SpriteComponent {
            render_object_handle: sprite_render_object.clone(),
        };

        let body_component = BodyComponent { velocity };

        let entity = commands.push((
            transform_component.clone(),
            sprite_component,
            body_component,
        ));

        commands.add_component(
            entity,
            VisibilityComponent {
                visibility_object_handle: {
                    let handle = visibility_region
                        .register_dynamic_object(ObjectId::from(entity), CullModel::quad(64., 64.));
                    handle.set_transform(
                        transform_component.translation,
                        transform_component.rotation,
                        transform_component.scale,
                    );
                    handle.add_render_object(&sprite_render_object);
                    handle
                },
            },
        );
    }

    sprite_spawner.sprite_count += spawn_count;
}

#[profiling::function]
fn update_main_view_2d(
    render_options: &RenderOptions,
    main_view_frustum: &mut ViewFrustumArc,
    viewports_resource: &mut ViewportsResource,
) {
    let (phase_mask_builder, feature_mask_builder, feature_flag_mask_builder) =
        super::util::default_main_view_masks(render_options);

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

    let look_at = Vec3::ZERO;
    let up = Vec3::Y;

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
        render_phase_mask: phase_mask_builder.build(),
        render_feature_mask: feature_mask_builder.build(),
        render_feature_flag_mask: feature_flag_mask_builder.build(),
        debug_name: "main".to_string(),
    });
}
