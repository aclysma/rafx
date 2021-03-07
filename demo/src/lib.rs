// There's a decent amount of code that's just for example and isn't called
#![allow(dead_code)]

use legion::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use structopt::StructOpt;

use rafx::api::{RafxExtents2D, RafxResult, RafxSwapchainHelper};
use rafx::assets::AssetManager;

use crate::daemon_args::AssetDaemonArgs;
use crate::scenes::SceneManager;
use crate::time::TimeState;
use rafx::assets::distill_impl::AssetResource;
use rafx::nodes::ExtractResources;
use rafx::renderer::ViewportsResource;
use rafx::renderer::{AssetSource, Renderer};

mod assets;
mod components;
pub mod daemon_args;
mod features;
mod init;
mod phases;
mod render_graph_generator;
mod scenes;
mod time;

mod demo_plugin;
pub use demo_plugin::DemoRendererPlugin;

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
    #[cfg(feature = "use-imgui")]
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

    #[cfg(feature = "use-imgui")]
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

    let asset_source = if let Some(packfile) = &args.packfile {
        AssetSource::Packfile(packfile.to_path_buf())
    } else {
        AssetSource::Daemon {
            external_daemon: args.external_daemon,
            daemon_args: args.daemon_args.clone().into(),
        }
    };

    let sdl2_systems = init::sdl2_init();
    init::rendering_init(&mut resources, &sdl2_systems.window, asset_source)?;

    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems
        .context
        .event_pump()
        .expect("Could not create sdl event pump");

    let mut world = World::default();
    let mut print_time_event = crate::time::PeriodicEvent::default();

    #[cfg(feature = "profile-with-puffin")]
    let mut profiler_ui = puffin_imgui::ProfilerUi::default();

    'running: loop {
        profiling::scope!("Main Loop");

        {
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            let (width, height) = sdl2_systems.window.vulkan_drawable_size();
            viewports_resource.main_window_size = RafxExtents2D { width, height }
        }

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
        #[cfg(feature = "use-imgui")]
        {
            use crate::features::imgui::Sdl2ImguiManager;
            use sdl2::mouse::MouseState;
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

            asset_manager.update_asset_loaders().unwrap();
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
        #[cfg(feature = "use-imgui")]
        {
            use crate::features::imgui::Sdl2ImguiManager;
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
        #[cfg(feature = "use-imgui")]
        {
            use crate::features::imgui::Sdl2ImguiManager;
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
            let game_renderer = resources.get::<Renderer>().unwrap();

            let mut extract_resources = ExtractResources::default();

            macro_rules! add_to_extract_resources {
                ($ty: ident) => {
                    #[allow(non_snake_case)]
                    let mut $ty = resources.get_mut::<$ty>().unwrap();
                    extract_resources.insert(&mut *$ty);
                };
                ($ty: path, $name: ident) => {
                    let mut $name = resources.get_mut::<$ty>().unwrap();
                    extract_resources.insert(&mut *$name);
                };
            }

            add_to_extract_resources!(RafxSwapchainHelper);
            add_to_extract_resources!(ViewportsResource);
            add_to_extract_resources!(AssetManager);
            add_to_extract_resources!(TimeState);
            add_to_extract_resources!(RenderOptions);
            add_to_extract_resources!(
                rafx::visibility::DynamicVisibilityNodeSet,
                dynamic_visibility_node_set
            );
            add_to_extract_resources!(
                rafx::visibility::StaticVisibilityNodeSet,
                static_visibility_node_set
            );
            add_to_extract_resources!(
                crate::features::sprite::SpriteRenderNodeSet,
                sprite_render_node_set
            );
            add_to_extract_resources!(
                crate::features::mesh::MeshRenderNodeSet,
                mesh_render_node_set
            );
            add_to_extract_resources!(
                crate::features::debug3d::DebugDraw3DResource,
                debug_draw_3d_resource
            );
            add_to_extract_resources!(crate::features::text::TextResource, text_resource);
            add_to_extract_resources!(crate::features::imgui::Sdl2ImguiManager, sdl2_imgui_manager);

            extract_resources.insert(&mut world);

            game_renderer
                .start_rendering_next_frame(&mut extract_resources)
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
    #[cfg(feature = "use-imgui")]
    let imgui_manager = resources
        .get::<crate::features::imgui::Sdl2ImguiManager>()
        .unwrap();
    let mut scene_manager = resources.get_mut::<SceneManager>().unwrap();
    for event in event_pump.poll_iter() {
        #[cfg(feature = "use-imgui")]
        let ignore_event = {
            imgui_manager.handle_event(&event);
            imgui_manager.ignore_event(&event)
        };

        #[cfg(not(feature = "use-imgui"))]
        let ignore_event = false;

        if !ignore_event {
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
