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
use rafx::renderer::{AssetSource, Renderer};
use rafx::renderer::{RendererConfigResource, ViewportsResource};
use rafx::visibility::VisibilityRegion;

pub mod assets;
mod components;
pub mod daemon_args;
mod features;
mod init;
mod phases;
mod render_graph_generator;
mod scenes;
mod time;

mod demo_plugin;
use crate::assets::font::FontAsset;
use crate::features::text::TextResource;
use crate::features::tile_layer::TileLayerResource;
pub use demo_plugin::DemoRendererPlugin;
use rafx::framework::RenderResources;

#[cfg(all(
    feature = "profile-with-tracy-memory",
    not(feature = "profile-with-stats-alloc")
))]
#[global_allocator]
static GLOBAL: profiling::tracy_client::ProfiledAllocator<std::alloc::System> =
    profiling::tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

#[cfg(all(
    feature = "profile-with-stats-alloc",
    not(feature = "profile-with-tracy-memory")
))]
#[global_allocator]
pub static STATS_ALLOC: &stats_alloc::StatsAlloc<std::alloc::System> =
    &stats_alloc::INSTRUMENTED_SYSTEM;

struct StatsAllocMemoryRegion<'a> {
    region_name: &'a str,
    #[cfg(all(
        feature = "profile-with-stats-alloc",
        not(feature = "profile-with-tracy-memory")
    ))]
    region: stats_alloc::Region<'a, std::alloc::System>,
}

impl<'a> StatsAllocMemoryRegion<'a> {
    pub fn new(region_name: &'a str) -> Self {
        StatsAllocMemoryRegion {
            region_name,
            #[cfg(all(
                feature = "profile-with-stats-alloc",
                not(feature = "profile-with-tracy-memory")
            ))]
            region: stats_alloc::Region::new(STATS_ALLOC),
        }
    }
}

#[cfg(all(
    feature = "profile-with-stats-alloc",
    not(feature = "profile-with-tracy-memory")
))]
impl Drop for StatsAllocMemoryRegion<'_> {
    fn drop(&mut self) {
        log::info!(
            "({}) | {:?}",
            self.region_name,
            self.region.change_and_reset()
        );
    }
}

// Should be kept in sync with the constants in tonemapper.glsl
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum TonemapperType {
    None,
    StephenHillACES,
    SimplifiedLumaACES,
    Hejl2015,
    Hable,
    FilmicALU,
    LogDerivative,
    VisualizeRGBMax,
    VisualizeLuma,
    MAX,
}
impl TonemapperType {
    pub fn display_name(&self) -> &'static str {
        match self {
            TonemapperType::None => "None",
            TonemapperType::StephenHillACES => "Stephen Hill ACES",
            TonemapperType::SimplifiedLumaACES => "SimplifiedLumaACES",
            TonemapperType::Hejl2015 => "Hejl 2015",
            TonemapperType::Hable => "Hable",
            TonemapperType::FilmicALU => "Filmic ALU (Hable)",
            TonemapperType::LogDerivative => "LogDerivative",
            TonemapperType::VisualizeRGBMax => "Visualize RGB Max",
            TonemapperType::VisualizeLuma => "Visualize RGB Luma",
            TonemapperType::MAX => "MAX_TONEMAPPER_VALUE",
        }
    }
}
impl From<i32> for TonemapperType {
    fn from(v: i32) -> Self {
        assert!(v <= Self::MAX as i32);
        unsafe { std::mem::transmute(v) }
    }
}

impl std::fmt::Display for TonemapperType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
#[derive(Clone)]
pub struct RenderOptions {
    pub enable_msaa: bool,
    pub enable_hdr: bool,
    pub enable_bloom: bool,
    pub show_debug3d: bool,
    pub show_text: bool,
    pub show_skybox: bool,
    pub show_feature_toggles: bool,
    pub show_shadows: bool,
    pub blur_pass_count: usize,
    pub tonemapper_type: TonemapperType,
    pub freeze_visibility: bool,
}

impl RenderOptions {
    fn default_2d() -> Self {
        RenderOptions {
            enable_msaa: false,
            enable_hdr: false,
            enable_bloom: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: true,
            show_shadows: true,
            show_feature_toggles: false,
            blur_pass_count: 0,
            tonemapper_type: TonemapperType::None,
            freeze_visibility: false,
        }
    }

    fn default_3d() -> Self {
        RenderOptions {
            enable_msaa: true,
            enable_hdr: true,
            enable_bloom: true,
            show_debug3d: true,
            show_text: true,
            show_skybox: true,
            show_shadows: true,
            show_feature_toggles: true,
            blur_pass_count: 5,
            tonemapper_type: TonemapperType::LogDerivative,
            freeze_visibility: false,
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

        if self.show_feature_toggles {
            ui.checkbox(
                imgui::im_str!("show_debug3d_feature"),
                &mut self.show_debug3d,
            );
            ui.checkbox(imgui::im_str!("show_text_feature"), &mut self.show_text);
            ui.checkbox(imgui::im_str!("show_skybox_feature"), &mut self.show_skybox);
            ui.checkbox(imgui::im_str!("show_shadows"), &mut self.show_shadows);
        }

        let mut blur_pass_count = self.blur_pass_count as i32;

        imgui::Drag::new(imgui::im_str!("blur_pass_count"))
            .range(0..=10)
            .build(ui, &mut blur_pass_count);

        self.blur_pass_count = blur_pass_count as usize;

        // iterate over the valid tonemapper values and convert them into their names
        let tonemapper_names: Vec<imgui::ImString> = (0..(TonemapperType::MAX as i32))
            .map(|t| imgui::ImString::new(TonemapperType::from(t).display_name()))
            .collect();

        let mut current_tonemapper_type = self.tonemapper_type as i32;

        if let Some(combo) = imgui::ComboBox::new(imgui::im_str!("tonemapper_type"))
            .preview_value(&tonemapper_names[current_tonemapper_type as usize])
            .begin(ui)
        {
            ui.list_box(
                imgui::im_str!(""),
                &mut current_tonemapper_type,
                &tonemapper_names.iter().collect::<Vec<_>>(),
                tonemapper_names.len() as i32,
            );
            combo.end(ui);
            self.tonemapper_type = current_tonemapper_type.into();
        }

        ui.checkbox(
            imgui::im_str!("freeze visibility"),
            &mut self.freeze_visibility,
        );
    }
}

#[derive(Default)]
pub struct DebugUiState {
    show_render_options: bool,
    show_asset_list: bool,

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

    let mut scene_manager = SceneManager::default();

    let mut resources = Resources::default();
    resources.insert(TimeState::new());
    resources.insert(RenderOptions::default_2d());
    resources.insert(DebugUiState::default());

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

    let font = {
        let asset_resource = resources.get::<AssetResource>().unwrap();
        asset_resource.load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf")
    };

    'running: loop {
        profiling::scope!("Main Loop");

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
        // Process input
        //
        if !process_input(&mut scene_manager, &mut world, &resources, &mut event_pump) {
            break 'running;
        }

        {
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            let (width, height) = sdl2_systems.window.vulkan_drawable_size();
            viewports_resource.main_window_size = RafxExtents2D { width, height };
        }

        {
            scene_manager.try_create_next_scene(&mut world, &resources);
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
        // Notify imgui of frame begin
        //
        #[cfg(feature = "use-imgui")]
        {
            use crate::features::imgui::Sdl2ImguiManager;
            use sdl2::mouse::MouseState;
            let imgui_manager = resources.get::<Sdl2ImguiManager>().unwrap();
            imgui_manager.begin_frame(&sdl2_systems.window, &MouseState::new(&event_pump));
        }

        {
            let mut text_resource = resources.get_mut::<TextResource>().unwrap();

            text_resource.add_text(
                "Use Left/Right arrow keys to switch demos".to_string(),
                glam::Vec3::new(100.0, 400.0, 0.0),
                &font,
                20.0,
                glam::Vec4::new(1.0, 1.0, 1.0, 1.0),
            );
        }

        {
            scene_manager.update_scene(&mut world, &mut resources);
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
            let asset_manager = resources.get::<AssetResource>().unwrap();
            imgui_manager.with_ui(|ui| {
                profiling::scope!("main menu bar");
                ui.main_menu_bar(|| {
                    ui.menu(imgui::im_str!("Windows"), true, || {
                        ui.checkbox(
                            imgui::im_str!("Render Options"),
                            &mut debug_ui_state.show_render_options,
                        );

                        ui.checkbox(
                            imgui::im_str!("Asset List"),
                            &mut debug_ui_state.show_asset_list,
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

                if debug_ui_state.show_asset_list {
                    imgui::Window::new(imgui::im_str!("Asset List"))
                        .opened(&mut debug_ui_state.show_asset_list)
                        .build(ui, || {
                            let loader = asset_manager.loader();
                            let mut asset_info = loader
                                .get_active_loads()
                                .into_iter()
                                .map(|item| loader.get_load_info(item))
                                .collect::<Vec<_>>();
                            asset_info.sort_by(|x, y| {
                                x.as_ref()
                                    .map(|x| &x.path)
                                    .cmp(&y.as_ref().map(|y| &y.path))
                            });
                            for info in asset_info {
                                if let Some(info) = info {
                                    let id = info.asset_id;
                                    ui.text(format!(
                                        "{}:{} .. {}",
                                        info.file_name.unwrap_or_else(|| "???".to_string()),
                                        info.asset_name.unwrap_or_else(|| format!("{}", id)),
                                        info.refs
                                    ));
                                } else {
                                    ui.text("NO INFO");
                                }
                            }
                        });
                }

                #[cfg(feature = "profile-with-puffin")]
                if debug_ui_state.show_profiler {
                    profiling::scope!("puffin profiler");
                    profiler_ui.window(ui);
                }
            });

            let mut render_config_resource = resources.get_mut::<RendererConfigResource>().unwrap();
            render_config_resource
                .visibility_config
                .enable_visibility_update = !render_options.freeze_visibility;
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

            add_to_extract_resources!(VisibilityRegion);
            add_to_extract_resources!(RafxSwapchainHelper);
            add_to_extract_resources!(ViewportsResource);
            add_to_extract_resources!(AssetManager);
            add_to_extract_resources!(TimeState);
            add_to_extract_resources!(RenderOptions);
            add_to_extract_resources!(RendererConfigResource);
            add_to_extract_resources!(TileLayerResource);
            add_to_extract_resources!(
                crate::features::sprite::SpriteRenderNodeSet,
                sprite_render_node_set
            );
            add_to_extract_resources!(
                crate::features::mesh::MeshRenderNodeSet,
                mesh_render_node_set
            );
            add_to_extract_resources!(
                crate::features::tile_layer::TileLayerRenderNodeSet,
                tile_layer_render_node_set
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
    scene_manager: &mut SceneManager,
    world: &mut World,
    resources: &Resources,
    event_pump: &mut sdl2::EventPump,
) -> bool {
    #[cfg(feature = "use-imgui")]
    let imgui_manager = resources
        .get::<crate::features::imgui::Sdl2ImguiManager>()
        .unwrap();
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
            let mut was_handled = false;
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
                        was_handled = true;
                    }

                    if keycode == Keycode::Left {
                        scene_manager.queue_load_previous_scene();
                        was_handled = true;
                    }

                    if keycode == Keycode::Right {
                        scene_manager.queue_load_next_scene();
                        was_handled = true;
                    }

                    if keycode == Keycode::M {
                        let metrics = resources.get::<AssetManager>().unwrap().metrics();
                        println!("{:#?}", metrics);
                        was_handled = true;
                    }
                }
                _ => {}
            }

            if !was_handled {
                scene_manager.process_input(world, resources, event);
            }
        }
    }

    true
}
