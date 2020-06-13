use renderer_shell_vulkan::{
    LogicalSize, Window, VkDevice, VkSwapchain, VkSurface, VkDeviceContext,
    VkTransferUpload, VkTransferUploadState, VkImage,
};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ash::prelude::VkResult;
use renderer_ext::imgui_support::{VkImGuiRenderPassFontAtlas};
use imgui::sys::ImGuiStorage_GetBoolRef;
use sdl2::mouse::MouseState;
use renderer_ext::{GameRendererWithContext, PositionComponent, SpriteComponent};
use atelier_assets::loader as atelier_loader;
use legion::prelude::*;

use atelier_assets::core::asset_uuid;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;

mod daemon;
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

fn main() {
    let mut log_level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    {
        log_level = log::LevelFilter::Debug;
    }

    // Setup logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        //.filter_module("renderer_shell_vulkan::buffer", log::LevelFilter::Debug)
        //.filter_module("renderer_ext::game_renderer", log::LevelFilter::Debug)
        .filter_module("renderer_ext::resource_managers::descriptor_sets", log::LevelFilter::Info)
        //.filter_module("renderer_ext::pipeline", log::LevelFilter::Trace)
        //.filter_level(log::LevelFilter::Error)
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

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    std::thread::spawn(move || {
        daemon::run();
    });

    // Something to track time
    let mut time_state = TimeState::new();
    time_state.update();

    // Setup SDL
    let sdl_context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Default window size
    let logical_size = LogicalSize {
        width: 900,
        height: 600,
    };

    // Create the window
    let sdl_window = video_subsystem
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

    log::info!("window created");

    // Load imgui, we do it a little early because it wants to have the actual SDL2 window and
    // doesn't work with the thin window wrapper
    let imgui_manager = renderer_ext::imgui_support::init_imgui_manager(&sdl_window);

    // Thin window wrapper to decouple the renderer from a specific windowing crate
    let window = Sdl2Window::new(&sdl_window);

    // Assets will be stored here, we init it ahead of the renderer as it will register its own
    // asset types
    let mut asset_resource = AssetResource::default();

    let universe = Universe::new();
    let mut world = universe.create_world();
    let mut resources = Resources::default();
    resources.insert(SpriteRenderNodeSet::default());
    resources.insert(StaticVisibilityNodeSet::default());
    resources.insert(DynamicVisibilityNodeSet::default());
    resources.insert(AssetResource::default());
    resources.insert(time_state);

    // Create the renderer, this will init the vulkan instance, device, and set up a swap chain
    let renderer = GameRendererWithContext::new(
        &window,
        imgui_manager.build_font_atlas(),
        &resources
    );

    // Check if there were error setting up vulkan
    if let Err(e) = renderer {
        log::error!("Error during renderer construction: {:?}", e);
        return;
    }

    log::info!("renderer created");

    let mut renderer = renderer.unwrap();

    log::info!("Starting window event loop");
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Could not create sdl event pump");










    {
        let sprites = ["sprite1", "sprite2", "sprite3"];
        for i in 0..100 {
            let position = Vec3::new(((i / 10) * 100) as f32, ((i % 10) * 100) as f32, 0.0);
            let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
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
                };

                let entity = world.insert(
                    (),
                    (0..1).map(|_| (position_component, sprite_component.clone())),
                )[0];

                log::debug!("create entity {:?}", entity);
                world.get_component::<PositionComponent>(entity).unwrap();

                SpriteRenderNode {
                    entity, // sprite asset
                }
            });
        }
    }














    let mut print_time_event = renderer_ext::time::PeriodicEvent::default();

    'running: loop {
        for event in event_pump.poll_iter() {
            imgui_manager.handle_event(&event);
            if !imgui_manager.ignore_event(&event) {
                //log::trace!("{:?}", event);
                match event {
                    //
                    // Halt if the user requests to close the window
                    //
                    Event::Quit { .. } => break 'running,

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
                            break 'running;
                        }

                        if keycode == Keycode::D {
                            renderer.dump_stats();
                        }
                    }

                    _ => {}
                }
            }
        }

        let window = Sdl2Window::new(&sdl_window);
        imgui_manager.begin_frame(&sdl_window, &MouseState::new(&event_pump));

        asset_resource.update();
        renderer.update_resources();

        imgui_manager.with_ui(|ui| {
            //let mut opened = true;
            //ui.show_demo_window(&mut opened);
            ui.main_menu_bar(|| {
                let time_state = resources.get::<TimeState>().unwrap();
                ui.text(imgui::im_str!(
                    "FPS: {:.1}",
                    time_state.updates_per_second_smoothed()
                ));
            });
        });

        imgui_manager.render(&sdl_window);

        //
        // Redraw
        //
        renderer.draw(&window, &world, &resources).unwrap();
        resources.get_mut::<TimeState>().unwrap().update();

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
    }
}
