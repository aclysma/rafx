// There's a decent amount of code that's just for example and isn't called
#![allow(dead_code)]

use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::imgui_support::Sdl2ImguiManager;
use legion::*;
use renderer::vulkan::VkDeviceContext;
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseState;

use crate::asset_resource::AssetResource;
use crate::features::debug3d::DebugDraw3DResource;
use crate::game_renderer::GameRenderer;
use crate::resource_manager::GameAssetManager;
use crate::time::TimeState;
use renderer::assets::AssetManager;

mod asset_loader;
mod asset_resource;
mod asset_storage;
mod assets;
mod components;
mod daemon;
mod features;
mod game_asset_lookup;
mod game_renderer;
mod imgui_support;
mod init;
mod phases;
mod render_contexts;
mod resource_manager;
mod test_scene;
mod time;

fn main() {
    init::logging_init();

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    std::thread::spawn(move || {
        daemon::run();
    });

    let mut resources = Resources::default();
    resources.insert(TimeState::new());

    // Connect to the daemon we just launched
    init::atelier_init(&mut resources, "127.0.0.1:9999".to_string());

    let sdl2_systems = init::sdl2_init();
    init::imgui_init(&mut resources, &sdl2_systems.window);
    init::rendering_init(&mut resources, &sdl2_systems.window);

    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems
        .context
        .event_pump()
        .expect("Could not create sdl event pump");

    let mut world = World::default();

    //test_scene::populate_test_sprite_entities(&mut resources, &mut world);
    test_scene::populate_test_mesh_entities(&mut resources, &mut world);
    test_scene::populate_test_lights(&mut resources, &mut world);

    let mut print_time_event = crate::time::PeriodicEvent::default();

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
            let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
            let mut game_resource_manager = resources.get_mut::<GameAssetManager>().unwrap();

            asset_manager.update_asset_loaders().unwrap();
            game_resource_manager
                .update_asset_loaders(&*asset_manager)
                .unwrap();
        }

        //
        // Process input
        //
        if !process_input(&resources, &mut event_pump) {
            break 'running;
        }

        add_light_debug_draw(&resources, &world);

        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut query = <Write<DirectionalLightComponent>>::query();
            for mut light in query.iter_mut(&mut world) {
                const LIGHT_XY_DISTANCE: f32 = 50.0;
                const LIGHT_Z: f32 = 50.0;
                const LIGHT_ROTATE_SPEED: f32 = 0.0;
                const LIGHT_LOOP_OFFSET: f32 = 2.0;
                let loop_time = time_state.total_time().as_secs_f32();
                let light_from = glam::Vec3::new(
                    LIGHT_XY_DISTANCE
                        * f32::cos(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_XY_DISTANCE
                        * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_Z,
                    //LIGHT_Z// * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET).abs(),
                    //0.2
                    //2.0
                );
                let light_to = glam::Vec3::default();

                light.direction = (light_to - light_from).normalize();
            }
        }

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
        log::trace!(
            "[main] simulation took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        //
        // Redraw
        //
        {
            let window = Sdl2Window::new(&sdl2_systems.window);
            let game_renderer = resources.get::<GameRenderer>().unwrap();
            game_renderer
                .start_rendering_next_frame(&resources, &world, &window)
                .unwrap();
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

    let mut query = <Read<DirectionalLightComponent>>::query();
    for light in query.iter(world) {
        let light_from = light.direction * -10.0;
        let light_to = glam::Vec3::zero();

        debug_draw.add_line(light_from, light_to, light.color);
    }

    let mut query = <(Read<PositionComponent>, Read<PointLightComponent>)>::query();
    for (position, light) in query.iter(world) {
        debug_draw.add_sphere(position.position, 0.25, light.color, 12);
    }

    let mut query = <(Read<PositionComponent>, Read<SpotLightComponent>)>::query();
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
                    keymod: _modifiers,
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
                        let metrics = resources.get::<AssetManager>().unwrap().metrics();
                        println!("{:#?}", metrics);
                    }
                }

                _ => {}
            }
        }
    }

    true
}
