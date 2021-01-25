// There's a decent amount of code that's just for example and isn't called
#![allow(dead_code)]

use crate::imgui_support::Sdl2ImguiManager;
use legion::*;
use rafx::api::RafxResult;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseState;

use crate::asset_resource::AssetResource;
use crate::daemon::AssetDaemonArgs;
use crate::game_asset_manager::GameAssetManager;
use crate::game_renderer::GameRenderer;
use crate::scenes::SceneManager;
use crate::time::TimeState;
use rafx::assets::AssetManager;
use structopt::StructOpt;

mod asset_loader;
mod asset_resource;
mod asset_storage;
mod assets;
mod components;
pub mod daemon;
mod features;
mod game_asset_lookup;
mod game_asset_manager;
mod game_renderer;
mod imgui_support;
mod init;
mod phases;
mod render_contexts;
mod scenes;
mod time;

#[derive(Clone)]
pub struct RenderOptions {
    pub enable_msaa: bool,
    pub enable_hdr: bool,
    pub enable_bloom: bool,
    pub blur_pass_count: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            enable_msaa: true,
            enable_hdr: true,
            enable_bloom: true,
            blur_pass_count: 5,
        }
    }
}

impl RenderOptions {
    pub fn window(
        &mut self,
        ui: &imgui::Ui<'_>,
    ) -> bool {
        let mut open = true;
        //TODO: tweak this and use imgui-inspect
        imgui::Window::new(imgui::im_str!("Render Options"))
            //.position([10.0, 25.0], imgui::Condition::FirstUseEver)
            //.size([600.0, 250.0], imgui::Condition::FirstUseEver)
            .opened(&mut open)
            .build(ui, || self.ui(ui));
        open
    }

    pub fn ui(
        &mut self,
        ui: &imgui::Ui<'_>,
    ) {
        ui.checkbox(imgui::im_str!("enable_msaa"), &mut self.enable_msaa);
        ui.checkbox(imgui::im_str!("enable_hdr"), &mut self.enable_hdr);
        ui.checkbox(imgui::im_str!("enable_bloom"), &mut self.enable_bloom);
        let mut blur_pass_count = self.blur_pass_count as i32;
        ui.drag_int(imgui::im_str!("blur_pass_count"), &mut blur_pass_count)
            .min(0)
            .max(10)
            .build();
        self.blur_pass_count = blur_pass_count as usize;
    }
}

#[derive(Default)]
pub struct DebugUiState {
    show_render_options: bool,

    #[cfg(feature = "profile-with-puffin")]
    show_profiler: bool,
}

#[derive(StructOpt)]
pub struct DemoArgs {
    /// Path to the packfile
    #[structopt(name = "packfile", long, parse(from_os_str))]
    pub packfile: Option<std::path::PathBuf>,

    #[structopt(name = "external-daemon", long)]
    pub external_daemon: bool,

    #[structopt(flatten)]
    pub daemon_args: AssetDaemonArgs,
}

pub fn run(args: &DemoArgs) -> RafxResult<()> {
    #[cfg(feature = "profile-with-tracy")]
    profiling::tracy_client::set_thread_name("Main Thread");
    #[cfg(feature = "profile-with-optick")]
    profiling::optick::register_thread("Main Thread");

    let mut resources = Resources::default();
    resources.insert(TimeState::new());
    resources.insert(RenderOptions::default());
    resources.insert(DebugUiState::default());
    resources.insert(SceneManager::default());

    if let Some(packfile) = &args.packfile {
        log::info!("Reading from packfile {:?}", packfile);

        // Initialize the packfile loader with the packfile path
        init::atelier_init_packfile(&mut resources, &packfile);
    } else {
        if !args.external_daemon {
            log::info!("Hosting local daemon at {:?}", args.daemon_args.address);

            // Spawn the daemon in a background thread. This could be a different process, but
            // for simplicity we'll launch it here.
            let daemon_args = args.daemon_args.clone().into();
            std::thread::spawn(move || {
                daemon::run(daemon_args);
            });
        } else {
            log::info!("Connecting to daemon at {:?}", args.daemon_args.address);
        }

        // Connect to the daemon we just launched
        init::atelier_init_daemon(&mut resources, args.daemon_args.address.to_string());
    }

    let sdl2_systems = init::sdl2_init();
    init::imgui_init(&mut resources, &sdl2_systems.window);
    init::rendering_init(&mut resources, &sdl2_systems.window)?;

    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems
        .context
        .event_pump()
        .expect("Could not create sdl event pump");

    let mut world = World::default();
    let mut print_time_event = crate::time::PeriodicEvent::default();

    #[cfg(feature = "profile-with-puffin")]
    let mut profiler_ui = puffin_imgui::ProfilerUi::default();

    #[cfg(feature = "profile-with-tracy")]
    {
        use tracing_subscriber::layer::SubscriberExt;
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
        )
        .unwrap();
    }

    'running: loop {
        profiling::scope!("Main Loop");

        {
            resources
                .get_mut::<SceneManager>()
                .unwrap()
                .try_create_next_scene(&mut world, &resources);
        }

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
            profiling::scope!("update asset resource");
            let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
            asset_resource.update();
        }

        //
        // Update graphics resources
        //
        {
            profiling::scope!("update asset loaders");
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

        {
            resources
                .get_mut::<SceneManager>()
                .unwrap()
                .update_scene(&mut world, &resources);
        }

        //
        // imgui debug draw,
        //
        {
            profiling::scope!("imgui");
            let imgui_manager = resources.get::<Sdl2ImguiManager>().unwrap();
            let time_state = resources.get::<TimeState>().unwrap();
            let mut debug_ui_state = resources.get_mut::<DebugUiState>().unwrap();
            let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
            imgui_manager.with_ui(|ui| {
                profiling::scope!("main menu bar");
                ui.main_menu_bar(|| {
                    ui.menu(imgui::im_str!("Windows"), true, || {
                        ui.checkbox(
                            imgui::im_str!("Render Options"),
                            &mut debug_ui_state.show_render_options,
                        );

                        #[cfg(feature = "profile-with-puffin")]
                        if ui.checkbox(
                            imgui::im_str!("Profiler"),
                            &mut debug_ui_state.show_profiler,
                        ) {
                            log::info!(
                                "Setting puffin profiler enabled: {:?}",
                                debug_ui_state.show_profiler
                            );
                            profiling::puffin::set_scopes_on(debug_ui_state.show_profiler);
                        }
                    });
                    ui.text(imgui::im_str!(
                        "FPS: {:.1}",
                        time_state.updates_per_second_smoothed()
                    ));
                    ui.separator();
                    ui.text(imgui::im_str!("Frame: {}", time_state.update_count()));
                });

                if debug_ui_state.show_render_options {
                    imgui::Window::new(imgui::im_str!("Render Options")).build(ui, || {
                        render_options.window(ui);
                    });
                }

                #[cfg(feature = "profile-with-puffin")]
                if debug_ui_state.show_profiler {
                    profiling::scope!("puffin profiler");
                    profiler_ui.window(ui);
                }
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
            "[main] Simulation took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        //
        // Redraw
        //
        {
            profiling::scope!("Start Next Frame Render");
            let game_renderer = resources.get::<GameRenderer>().unwrap();

            let (window_width, window_height) = sdl2_systems.window.vulkan_drawable_size();

            game_renderer
                .start_rendering_next_frame(&resources, &world, window_width, window_height)
                .unwrap();
        }

        let t2 = std::time::Instant::now();
        log::trace!(
            "[main] start rendering took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        profiling::finish_frame!();
    }

    init::rendering_destroy(&mut resources)
}

fn process_input(
    resources: &Resources,
    event_pump: &mut sdl2::EventPump,
) -> bool {
    let imgui_manager = resources.get::<Sdl2ImguiManager>().unwrap();
    let mut scene_manager = resources.get_mut::<SceneManager>().unwrap();
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

                    #[cfg(feature = "rafx-vulkan")]
                    if keycode == Keycode::D {
                        let stats = resources
                            .get::<rafx::api::RafxDeviceContext>()
                            .unwrap()
                            .vk_device_context()
                            .unwrap()
                            .allocator()
                            .calculate_stats()
                            .unwrap();
                        println!("{:#?}", stats);
                    }

                    if keycode == Keycode::Left {
                        scene_manager.queue_load_previous_scene();
                    }

                    if keycode == Keycode::Right {
                        scene_manager.queue_load_next_scene();
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
