use renderer_shell_vulkan::{LogicalSize, Window, VkDevice, VkSwapchain, VkSurface, VkDeviceContext, VkTransferUpload, VkTransferUploadState, VkImage, VkContextBuilder, MsaaLevel, VkCreateContextError, VkContext};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ash::prelude::VkResult;
use renderer_ext::imgui_support::{VkImGuiRenderPassFontAtlas, Sdl2ImguiManager};
use imgui::sys::ImGuiStorage_GetBoolRef;
use sdl2::mouse::MouseState;
use renderer_ext::{PositionComponent, SpriteComponent, MeshComponent, PointLightComponent, SpotLightComponent, DirectionalLightComponent};
use atelier_assets::loader as atelier_loader;
use legion::prelude::*;

use atelier_assets::core::asset_uuid;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;

use renderer_ext::asset_resource::AssetResource;
use renderer_ext::image_utils::{DecodedTexture, enqueue_load_images};
use imgui::{Key, Image};
use renderer_ext::asset_storage::{ResourceLoadHandler, ResourceHandle};
use std::mem::ManuallyDrop;
use std::time::Duration;
use atelier_loader::AssetLoadOp;
use std::error::Error;
use renderer_ext::pipeline::image::ImageAsset;
use renderer_ext::pipeline_description::GraphicsPipeline;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use renderer_ext::features::sprite::{SpriteRenderNodeSet, SpriteRenderNode};
use renderer_base::visibility::{StaticVisibilityNodeSet, DynamicVisibilityNodeSet, DynamicAabbVisibilityNode};
use renderer_ext::time::TimeState;
use glam::f32::Vec3;
use renderer_ext::resource_managers::ResourceManager;
use renderer_base::RenderRegistry;
use sdl2::event::EventType::RenderDeviceReset;
use crate::game_renderer::{GameRenderer, SwapchainLifetimeListener};
use renderer_ext::pipeline::gltf::MeshAsset;
use renderer_ext::features::mesh::{MeshRenderNodeSet, MeshRenderNode};
use renderer_ext::renderpass::debug_renderpass::DebugDraw3DResource;

mod game_renderer;
mod daemon;

fn begin_load_asset<T>(
    asset_uuid: AssetUuid,
    asset_resource: &AssetResource,
) -> atelier_assets::loader::handle::Handle<T> {
    use atelier_assets::loader::Loader;
    let load_handle = asset_resource.loader().add_ref(asset_uuid);
    atelier_assets::loader::handle::Handle::<T>::new(asset_resource.tx().clone(), load_handle)
}

fn main() {
    logging_init();

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    std::thread::spawn(move || {
        daemon::run();
    });

    let mut resources = Resources::default();
    resources.insert(TimeState::new());

    atelier_init(&mut resources);

    let sdl2_systems = sdl2_init();
    imgui_init(&mut resources, &sdl2_systems.window);
    rendering_init(&mut resources, &sdl2_systems.window);

    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems.context
        .event_pump()
        .expect("Could not create sdl event pump");

    let universe = Universe::new();
    let mut world = universe.create_world();

    populate_test_sprite_entities(&mut resources, &mut world);
    populate_test_mesh_entities(&mut resources, &mut world);
    populate_test_lights(&mut resources, &mut world);

    let mut print_time_event = renderer_ext::time::PeriodicEvent::default();

    'running: loop {
        let t0 = std::time::Instant::now();
        //
        // Update time
        //
        {
            resources.get_mut::<TimeState>().unwrap().update();
        }


        //
        // Print FPS
        //
        {
            let time_state = resources.get::<TimeState>().unwrap();
            if print_time_event.try_take_event(
                time_state.current_instant(),
                std::time::Duration::from_secs_f32(1.0),
            ) {
                log::info!("FPS: {}", time_state.updates_per_second());
                //renderer.dump_stats();
            }
        }

        //
        // Notify imgui of frame begin
        //
        {
            let imgui_manager = resources.get::<Sdl2ImguiManager>().unwrap();
            let window = Sdl2Window::new(&sdl2_systems.window);
            imgui_manager.begin_frame(&sdl2_systems.window, &MouseState::new(&event_pump));
        }

        //
        // Update assets
        //
        {
            let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
            asset_resource.update();
        }

        //
        // Update graphics resources
        //
        {
            // let device = resources.get::<VkDeviceContext>().unwrap();
            // let mut game_renderer = resources.get_mut::<Game>().unwrap();
            // game_renderer.update_resources(&*device);
            renderer_ext::update_renderer(&resources);
        }

        //
        // Process input
        //
        if !process_input(&resources, &mut event_pump) {
            break 'running;
        }

        add_light_debug_draw(&resources, &world);

        //
        // imgui debug draw,
        //
        {
            let imgui_manager = resources.get::<Sdl2ImguiManager>().unwrap();
            let time_state = resources.get::<TimeState>().unwrap();
            imgui_manager.with_ui(|ui| {
                ui.main_menu_bar(|| {
                    ui.text(imgui::im_str!(
                        "FPS: {:.1}",
                        time_state.updates_per_second_smoothed()
                    ));
                    ui.separator();
                    ui.text(imgui::im_str!(
                        "Frame: {}",
                        time_state.update_count()
                    ));
                });
            });
        }

        //
        // Close imgui input for this frame and render the results to memory
        //
        {
            let imgui_manager = resources.get::<Sdl2ImguiManager>().unwrap();
            imgui_manager.render(&sdl2_systems.window);
        }

        let t1 = std::time::Instant::now();
        log::info!("[main] simulation took {} ms", (t1 - t0).as_secs_f32() * 1000.0);

        //
        // Redraw
        //
        {
            let window = Sdl2Window::new(&sdl2_systems.window);
            let mut game_renderer = resources.get::<GameRenderer>().unwrap();
            game_renderer.begin_render(&resources, &world, &window);
        }

        //let t2 = std::time::Instant::now();
        //log::info!("main thread took {} ms", (t2 - t0).as_secs_f32() * 1000.0);
    }

    rendering_destroy(&mut resources);
}

fn logging_init() {
    let mut log_level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    {
        log_level = log::LevelFilter::Debug;
    }

    // Setup logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_module("renderer_ext::resource_managers::descriptor_sets", log::LevelFilter::Info)
        .filter_module("renderer_base", log::LevelFilter::Info)
        .filter_level(log_level)
        // .format(|buf, record| { //TODO: Get a frame count in here
        //     writeln!(buf,
        //              "{} [{}] - {}",
        //              chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
        //              record.level(),
        //              record.args()
        //     )
        // })
        .init();
}

fn atelier_init(
    resources: &mut Resources,
) {
    resources.insert(AssetResource::default());
}

struct Sdl2Systems {
    pub context: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
}

fn sdl2_init() -> Sdl2Systems {
    // Setup SDL
    let context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Default window size
    let logical_size = LogicalSize {
        width: 900,
        height: 600,
    };

    // Create the window
    let window = video_subsystem
        .window(
            "Renderer Prototype",
            logical_size.width,
            logical_size.height,
        )
        .position_centered()
        .allow_highdpi()
        .resizable()
        .vulkan()
        .build()
        .expect("Failed to create window");

    Sdl2Systems {
        context,
        video_subsystem,
        window
    }
}

// Should occur *before* the renderer starts
fn imgui_init(
    resources: &mut Resources,
    sdl2_window: &sdl2::video::Window,
) {
    // Load imgui, we do it a little early because it wants to have the actual SDL2 window and
    // doesn't work with the thin window wrapper
    let imgui_manager = renderer_ext::imgui_support::init_imgui_manager(sdl2_window);
    resources.insert(imgui_manager);
}

fn rendering_init(
    resources: &mut Resources,
    sdl2_window: &sdl2::video::Window,
) {
    // Thin window wrapper to decouple the renderer from a specific windowing crate
    let window_wrapper = Sdl2Window::new(&sdl2_window);

    resources.insert(SpriteRenderNodeSet::default());
    resources.insert(MeshRenderNodeSet::default());
    resources.insert(StaticVisibilityNodeSet::default());
    resources.insert(DynamicVisibilityNodeSet::default());

    let mut context = VkContextBuilder::new()
        .use_vulkan_debug_layer(false)
        .msaa_level_priority(vec![MsaaLevel::Sample4])
        //.msaa_level_priority(vec![MsaaLevel::Sample1])
        .prefer_mailbox_present_mode();

    //#[cfg(debug_assertions)]
    {
        //context = context.use_vulkan_debug_layer(true);
    }

    let vk_context = context.build(&window_wrapper).unwrap();
    let device_context = vk_context.device_context().clone();
    resources.insert(vk_context);
    resources.insert(device_context);

    renderer_ext::init_renderer(resources);

    let mut game_renderer = GameRenderer::new(&window_wrapper, &resources).unwrap();
    resources.insert(game_renderer);

    let window_surface = SwapchainLifetimeListener::create_surface(resources, &window_wrapper).unwrap();
    resources.insert(window_surface);
}

fn rendering_destroy(
    resources: &mut Resources
) {
    // Destroy these first
    {
        SwapchainLifetimeListener::tear_down(resources);

        resources.remove::<VkSurface>();
        resources.remove::<GameRenderer>();
        resources.remove::<VkDeviceContext>();
        resources.remove::<SpriteRenderNodeSet>();
        resources.remove::<MeshRenderNodeSet>();
        resources.remove::<StaticVisibilityNodeSet>();
        resources.remove::<DynamicVisibilityNodeSet>();

        renderer_ext::destroy_renderer(resources);
    }

    // Drop this one last
    resources.remove::<VkContext>();
}

fn populate_test_sprite_entities(resources: &mut Resources, world: &mut World) {
    let sprite_image = {
        let mut asset_resource = resources.get::<AssetResource>().unwrap();
        begin_load_asset::<ImageAsset>(
            asset_uuid!("7c42f3bc-e96b-49f6-961b-5bfc799dee50"),
            &asset_resource,
        )
    };

    let sprites = ["sprite1", "sprite2", "sprite3"];
    for i in 0..1000 {
        let position = Vec3::new(((i / 10) * 25) as f32, ((i % 10) * 25) as f32, 0.0);
        //let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
        let alpha = 1.0;
        let _sprite = sprites[i % sprites.len()];

        let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
        let mut dynamic_visibility_node_set = resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

        // User calls functions to register render objects
        // - This is a retained API because render object existence can trigger loading streaming assets and
        //   keep them resident in memory
        // - Some render objects might not correspond to legion entities, and some people might not be using
        //   legion at all
        // - the `_with_handle` variant allows us to get the handle of the value that's going to be allocated
        //   This resolves a circular dependency where the component needs the render node handle and the
        //   render node needs the entity.
        // - ALTERNATIVE: Could create an empty entity, create the components, and then add all of them
        sprite_render_nodes.register_sprite_with_handle(|sprite_handle| {
            let aabb_info = DynamicAabbVisibilityNode {
                handle: sprite_handle.into(),
                // aabb bounds
            };

            // User calls functions to register visibility objects
            // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
            let visibility_handle =
                dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

            let position_component = PositionComponent { position };
            let sprite_component = SpriteComponent {
                sprite_handle,
                visibility_handle,
                alpha,
                image: sprite_image.clone(),
            };

            let entity = world.insert(
                (),
                (0..1).map(|_| (position_component, sprite_component.clone())),
            )[0];

            world.get_component::<PositionComponent>(entity).unwrap();

            SpriteRenderNode {
                entity, // sprite asset
            }
        });
    }
}

fn populate_test_mesh_entities(resources: &mut Resources, world: &mut World) {
    let mesh = {
        let mut asset_resource = resources.get::<AssetResource>().unwrap();
        begin_load_asset::<MeshAsset>(
            asset_uuid!("ffc9b240-0a17-4ff4-bb7d-72d13cc6e261"),
            &asset_resource,
        )
    };

    for i in 0..100 {
        let position = Vec3::new(((i / 10) * 3) as f32, ((i % 10) * 3) as f32, 0.0);

        let mut mesh_render_nodes = resources.get_mut::<MeshRenderNodeSet>().unwrap();
        let mut dynamic_visibility_node_set = resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

        // User calls functions to register render objects
        // - This is a retained API because render object existence can trigger loading streaming assets and
        //   keep them resident in memory
        // - Some render objects might not correspond to legion entities, and some people might not be using
        //   legion at all
        // - the `_with_handle` variant allows us to get the handle of the value that's going to be allocated
        //   This resolves a circular dependency where the component needs the render node handle and the
        //   render node needs the entity.
        // - ALTERNATIVE: Could create an empty entity, create the components, and then add all of them
        mesh_render_nodes.register_mesh_with_handle(|mesh_handle| {
            let aabb_info = DynamicAabbVisibilityNode {
                handle: mesh_handle.into(),
                // aabb bounds
            };

            // User calls functions to register visibility objects
            // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
            let visibility_handle =
                dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

            let position_component = PositionComponent { position };
            let mesh_component = MeshComponent {
                mesh_handle,
                visibility_handle,
                mesh: mesh.clone()
            };

            let entity = world.insert(
                (),
                (0..1).map(|_| (position_component, mesh_component.clone())),
            )[0];

            world.get_component::<PositionComponent>(entity).unwrap();

            MeshRenderNode {
                entity, // sprite asset
            }
        });
    }
}

fn populate_test_lights(
    resources: &mut Resources,
    world: &mut World,
) {
    add_point_light(
        resources,
        world,
        glam::Vec3::new(-3.0, -3.0, 3.0),
        PointLightComponent {
            color: [1.0, 1.0, 1.0, 1.0].into(),
            intensity: 130.0,
            range: 25.0
        }
    );

    add_point_light(
        resources,
        world,
        glam::Vec3::new(-3.0, 3.0, 3.0),
        PointLightComponent {
            color: [1.0, 1.0, 1.0, 1.0].into(),
            intensity: 130.0,
            range: 25.0
        }
    );

    let light_from = glam::Vec3::new(-3.0, -3.0, 0.0);
    let light_to = glam::Vec3::new(0.0, 0.0, 0.0);
    let light_direction = (light_to - light_from).normalize();
    add_spot_light(
        resources,
        world,
        light_from,
        SpotLightComponent {
            direction: light_direction,
            spotlight_half_angle: 10.0 * (std::f32::consts::PI / 180.0),
            range: 8.0,
            color: [1.0, 1.0, 1.0, 1.0].into(),
            intensity: 1000.0,
        }
    );

    let light_from = glam::Vec3::new(5.0, 5.0, 5.0);
    let light_to = glam::Vec3::new(0.0, 0.0, 0.0);
    let light_direction = (light_to - light_from).normalize();
    add_directional_light(
        resources,
        world,
        DirectionalLightComponent {
            direction: light_direction,
            intensity: 5.0,
            color: [1.0, 1.0, 1.0, 1.0].into()
        }
    );
}

fn add_directional_light(
    resources: &mut Resources,
    world: &mut World,
    light_component: DirectionalLightComponent
) {
    world.insert(
        (),
        vec![(light_component,)],
    );
}

fn add_spot_light(
    resources: &mut Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: SpotLightComponent
) {
    let position_component = PositionComponent {
        position
    };

    world.insert(
        (),
        vec![(position_component, light_component)],
    );
}

fn add_point_light(
    resources: &mut Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: PointLightComponent
) {
    let position_component = PositionComponent {
        position
    };

    world.insert(
        (),
        vec![(position_component, light_component)],
    );
}

fn add_light_debug_draw(
    resources: &Resources,
    world: &World,
) {
    let mut debug_draw = resources.get_mut::<DebugDraw3DResource>().unwrap();

    let query = <(Read<DirectionalLightComponent>)>::query();
    for light in query.iter(world) {
        let light_from = glam::Vec3::new(0.0, 0.0, 0.0);
        let light_to = light.direction;

        debug_draw.add_line(light_from, light_to, light.color);
    }

    let query = <(Read<PositionComponent>, Read<PointLightComponent>)>::query();
    for (position, light) in query.iter(world) {
        debug_draw.add_sphere(
            position.position,
            0.25,
            light.color,
            12
        );
    }

    let query = <(Read<PositionComponent>, Read<SpotLightComponent>)>::query();
    for (position, light) in query.iter(world) {
        let light_from = position.position;
        let light_to = position.position + light.direction;
        let light_direction = (light_to - light_from).normalize();

        debug_draw.add_cone(
            light_from,
            light_from + (light.range * light_direction),
            light.range * light.spotlight_half_angle.tan(),
            light.color,
            8
        );
    }
}

fn process_input(resources: &Resources, event_pump: &mut sdl2::EventPump) -> bool {
    let imgui_manager = resources.get::<Sdl2ImguiManager>().unwrap();
    for event in event_pump.poll_iter() {
        imgui_manager.handle_event(&event);
        if !imgui_manager.ignore_event(&event) {
            //log::trace!("{:?}", event);
            match event {
                //
                // Halt if the user requests to close the window
                //
                Event::Quit { .. } => return false,

                //
                // Close if the escape key is hit
                //
                Event::KeyDown {
                    keycode: Some(keycode),
                    keymod: modifiers,
                    ..
                } => {
                    //log::trace!("Key Down {:?} {:?}", keycode, modifiers);
                    if keycode == Keycode::Escape {
                        return false;
                    }

                    if keycode == Keycode::D {
                        let stats = resources.get::<VkDeviceContext>().unwrap().allocator().calculate_stats().unwrap();
                        println!("{:#?}", stats);
                    }

                    if keycode == Keycode::M {
                        let metrics = resources.get::<ResourceManager>().unwrap().metrics();
                        println!("{:#?}", metrics);
                    }
                }

                _ => {}
            }
        }
    }

    true
}
