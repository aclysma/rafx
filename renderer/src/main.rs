use renderer_shell_vulkan::{
    LogicalSize, Window, VkDevice, VkSwapchain, VkSurface, VkDeviceContext, VkTransferUpload,
    VkTransferUploadState, VkImage, VkContextBuilder, MsaaLevel, VkCreateContextError, VkContext,
};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ash::prelude::VkResult;
use renderer_features::imgui_support::{ImGuiFontAtlas, Sdl2ImguiManager};
use imgui::sys::ImGuiStorage_GetBoolRef;
use sdl2::mouse::MouseState;
use renderer_features::{
    PositionComponent, SpriteComponent, PointLightComponent, SpotLightComponent,
    DirectionalLightComponent,
};
use atelier_assets::loader as atelier_loader;
use legion::prelude::*;

use atelier_assets::core::asset_uuid;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;

use renderer_assets::asset_resource::AssetResource;
use renderer_assets::image_utils::{DecodedTexture, enqueue_load_images};
use imgui::{Key, Image};
use renderer_assets::asset_storage::{ResourceLoadHandler};
use std::mem::ManuallyDrop;
use std::time::Duration;
use atelier_loader::AssetLoadOp;
use std::error::Error;
use renderer_assets::assets::image::ImageAsset;
use renderer_assets::vk_description::GraphicsPipeline;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use renderer_features::features::sprite::{SpriteRenderNodeSet, SpriteRenderNode};
use renderer_visibility::{
    StaticVisibilityNodeSet, DynamicVisibilityNodeSet, DynamicAabbVisibilityNode,
};
use renderer_base::time::TimeState;
use glam::f32::Vec3;
use renderer_resources::resource_managers::ResourceManager;
use renderer_nodes::RenderRegistry;
use sdl2::event::EventType::RenderDeviceReset;
use crate::game_renderer::{GameRenderer, SwapchainLifetimeListener};
use renderer::assets::gltf::MeshAsset;
use renderer::features::mesh::{MeshRenderNodeSet, MeshRenderNode};
use renderer_features::renderpass::debug_renderpass::DebugDraw3DResource;
use renderer::resource_manager::GameResourceManager;

mod game_renderer;
mod daemon;
mod init;
mod test_scene;

fn main() {
    init::logging_init();

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    std::thread::spawn(move || {
        daemon::run();
    });

    let mut resources = Resources::default();
    resources.insert(TimeState::new());

    init::atelier_init(&mut resources);

    let sdl2_systems = init::sdl2_init();
    init::imgui_init(&mut resources, &sdl2_systems.window);
    init::rendering_init(&mut resources, &sdl2_systems.window);

    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems
        .context
        .event_pump()
        .expect("Could not create sdl event pump");

    let universe = Universe::new();
    let mut world = universe.create_world();

    test_scene::populate_test_sprite_entities(&mut resources, &mut world);
    test_scene::populate_test_mesh_entities(&mut resources, &mut world);
    test_scene::populate_test_lights(&mut resources, &mut world);

    let mut print_time_event = renderer_base::time::PeriodicEvent::default();

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
            renderer_resources::update_renderer_assets(&resources);

            let resource_manager = resources.get::<ResourceManager>().unwrap();
            let mut game_resource_manager = resources.get_mut::<GameResourceManager>().unwrap();

            game_resource_manager.update_resources(&*resource_manager);
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
                    ui.text(imgui::im_str!("Frame: {}", time_state.update_count()));
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
        log::info!(
            "[main] simulation took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

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

    init::rendering_destroy(&mut resources);
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
        debug_draw.add_sphere(position.position, 0.25, light.color, 12);
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
            8,
        );
    }
}

fn process_input(
    resources: &Resources,
    event_pump: &mut sdl2::EventPump,
) -> bool {
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
                        let stats = resources
                            .get::<VkDeviceContext>()
                            .unwrap()
                            .allocator()
                            .calculate_stats()
                            .unwrap();
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
