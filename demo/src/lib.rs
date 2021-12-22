// There's a decent amount of code that's just for example and isn't called
#![allow(dead_code)]

mod main_native;

pub use main_native::*;

use legion::*;
use structopt::StructOpt;

use rafx::api::{RafxExtents2D, RafxResult, RafxSwapchainHelper};
use rafx::assets::AssetManager;

pub use crate::daemon_args::AssetDaemonArgs;
use crate::scenes::SceneManager;
use crate::time::{PeriodicEvent, TimeState};
use rafx::assets::distill_impl::AssetResource;
use rafx::render_features::ExtractResources;
use rafx::renderer::{AssetSource, Renderer};
use rafx::renderer::{RendererConfigResource, ViewportsResource};
use rafx::visibility::VisibilityRegion;

pub mod daemon_args;
mod init;
mod input;
mod scenes;
mod time;

mod demo_renderer_thread_pool;

use crate::input::InputResource;
use rafx::distill::loader::handle::Handle;
use rafx_plugins::assets::font::FontAsset;
#[cfg(feature = "egui")]
use rafx_plugins::features::egui::{EguiContextResource, WinitEguiManager};
use rafx_plugins::features::skybox::SkyboxResource;
use rafx_plugins::features::text::TextResource;
use rafx_plugins::features::tile_layer::TileLayerResource;
use winit::event_loop::ControlFlow;

#[cfg(feature = "basic-pipeline")]
use rafx_plugins::features::mesh_basic::{
    MeshBasicRenderObjectSet as MeshRenderObjectSet, MeshBasicRenderOptions as MeshRenderOptions,
};
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::pipelines::basic::BasicPipelineTonemapperType as PipelineTonemapperType;
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::pipelines::basic::{
    BasicPipelineRenderOptions as PipelineRenderOptions,
    BasicPipelineTonemapDebugData as PipelineTonemapDebugData,
};

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::features::mesh_adv::{
    MeshBasicRenderObjectSet as MeshRenderObjectSet, MeshBasicRenderOptions as MeshRenderOptions,
};
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::BasicPipelineTonemapperType as PipelineTonemapperType;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::{
    BasicPipelineRenderOptions as PipelineRenderOptions,
    BasicPipelineTonemapDebugData as PipelineTonemapDebugData,
};

#[cfg(all(feature = "profile-with-tracy-memory", not(feature = "stats_alloc")))]
#[global_allocator]
static GLOBAL: profiling::tracy_client::ProfiledAllocator<std::alloc::System> =
    profiling::tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

#[cfg(all(feature = "stats_alloc", not(feature = "profile-with-tracy-memory")))]
#[global_allocator]
pub static STATS_ALLOC: &stats_alloc::StatsAlloc<std::alloc::System> =
    &stats_alloc::INSTRUMENTED_SYSTEM;

struct StatsAllocMemoryRegion<'a> {
    region_name: &'a str,
    #[cfg(all(feature = "stats_alloc", not(feature = "profile-with-tracy-memory")))]
    region: stats_alloc::Region<'a, std::alloc::System>,
}

impl<'a> StatsAllocMemoryRegion<'a> {
    pub fn new(region_name: &'a str) -> Self {
        StatsAllocMemoryRegion {
            region_name,
            #[cfg(all(feature = "stats_alloc", not(feature = "profile-with-tracy-memory")))]
            region: stats_alloc::Region::new(STATS_ALLOC),
        }
    }
}

#[cfg(all(feature = "stats_alloc", not(feature = "profile-with-tracy-memory")))]
impl Drop for StatsAllocMemoryRegion<'_> {
    fn drop(&mut self) {
        log::info!(
            "({}) | {:?}",
            self.region_name,
            self.region.change_and_reset()
        );
    }
}

#[derive(Clone)]
pub struct RenderOptions {
    pub enable_msaa: bool,
    pub enable_hdr: bool,
    pub enable_bloom: bool,
    pub enable_textures: bool,
    pub enable_lighting: bool,
    pub show_surfaces: bool,
    pub show_wireframes: bool,
    pub show_debug3d: bool,
    pub show_text: bool,
    pub show_skybox: bool,
    pub show_feature_toggles: bool,
    pub show_shadows: bool,
    pub blur_pass_count: usize,
    pub tonemapper_type: PipelineTonemapperType,
    pub enable_visibility_update: bool,
}

impl RenderOptions {
    fn default_2d() -> Self {
        RenderOptions {
            enable_msaa: false,
            enable_hdr: false,
            enable_bloom: false,
            enable_textures: true,
            enable_lighting: true,
            show_surfaces: true,
            show_wireframes: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: false,
            show_shadows: true,
            show_feature_toggles: false,
            blur_pass_count: 0,
            tonemapper_type: PipelineTonemapperType::None,
            enable_visibility_update: true,
        }
    }

    fn default_3d() -> Self {
        RenderOptions {
            enable_msaa: true,
            enable_hdr: true,
            enable_bloom: true,
            enable_textures: true,
            enable_lighting: true,
            show_surfaces: true,
            show_wireframes: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: true,
            show_shadows: true,
            show_feature_toggles: true,
            blur_pass_count: 5,
            tonemapper_type: PipelineTonemapperType::Bergstrom,
            enable_visibility_update: true,
        }
    }
}

impl RenderOptions {
    #[cfg(feature = "egui")]
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
    ) {
        ui.checkbox(&mut self.enable_msaa, "enable_msaa");
        ui.checkbox(&mut self.enable_hdr, "enable_hdr");

        if self.enable_hdr {
            ui.indent("HDR options", |ui| {
                let tonemapper_names: Vec<_> = (0..(PipelineTonemapperType::MAX as i32))
                    .map(|t| PipelineTonemapperType::from(t).display_name())
                    .collect();

                egui::ComboBox::from_label("tonemapper_type")
                    .selected_text(tonemapper_names[self.tonemapper_type as usize])
                    .show_ui(ui, |ui| {
                        for (i, name) in tonemapper_names.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.tonemapper_type,
                                PipelineTonemapperType::from(i as i32),
                                name,
                            );
                        }
                    });

                ui.checkbox(&mut self.enable_bloom, "enable_bloom");
                if self.enable_bloom {
                    ui.indent("", |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.blur_pass_count, 0..=10)
                                .clamp_to_range(true)
                                .text("blur_pass_count"),
                        );
                    });
                }
            });
        }

        if self.show_feature_toggles {
            ui.checkbox(&mut self.show_wireframes, "show_wireframes");
            ui.checkbox(&mut self.show_surfaces, "show_surfaces");

            if self.show_surfaces {
                ui.indent("", |ui| {
                    ui.checkbox(&mut self.enable_textures, "enable_textures");
                    ui.checkbox(&mut self.enable_lighting, "enable_lighting");

                    if self.enable_lighting {
                        ui.indent("", |ui| {
                            ui.checkbox(&mut self.show_shadows, "show_shadows");
                        });
                    }

                    ui.checkbox(&mut self.show_skybox, "show_skybox_feature");
                });
            }

            ui.checkbox(&mut self.show_debug3d, "show_debug3d_feature");
            ui.checkbox(&mut self.show_text, "show_text_feature");
        }

        ui.checkbox(
            &mut self.enable_visibility_update,
            "enable_visibility_update",
        );
    }
}

#[derive(Default)]
pub struct DebugUiState {
    show_render_options: bool,
    show_asset_list: bool,
    show_tonemap_debug: bool,
    show_shadow_map_debug: bool,

    #[cfg(feature = "profile-with-puffin")]
    show_profiler: bool,
}

#[derive(StructOpt)]
pub struct DemoArgs {
    /// Path to the packfile
    #[structopt(name = "packfile", long, parse(from_os_str))]
    pub packfile: Option<std::path::PathBuf>,

    #[structopt(skip)]
    pub packbuffer: Option<&'static [u8]>,

    #[structopt(name = "external-daemon", long)]
    pub external_daemon: bool,

    #[structopt(flatten)]
    pub daemon_args: AssetDaemonArgs,
}

impl DemoArgs {
    fn asset_source(&self) -> Option<AssetSource> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(packfile) = &self.packfile {
            return Some(AssetSource::Packfile(packfile.to_path_buf()));
        }

        {
            return Some(AssetSource::Daemon {
                external_daemon: self.external_daemon,
                daemon_args: self.daemon_args.clone().into(),
            });
        }
    }
}

struct DemoApp {
    scene_manager: SceneManager,
    resources: Resources,
    world: World,
    print_time_event: PeriodicEvent,
    font: Handle<FontAsset>,
}

impl DemoApp {
    fn init(
        args: &DemoArgs,
        window: &winit::window::Window,
    ) -> RafxResult<Self> {
        #[cfg(feature = "profile-with-tracy")]
        profiling::tracy_client::set_thread_name("Main Thread");
        #[cfg(feature = "profile-with-optick")]
        profiling::optick::register_thread("Main Thread");

        let scene_manager = SceneManager::default();

        let mut resources = Resources::default();
        resources.insert(TimeState::new());
        resources.insert(InputResource::new());
        resources.insert(RenderOptions::default_2d());
        resources.insert(MeshRenderOptions::default());
        resources.insert(PipelineRenderOptions::default());
        resources.insert(PipelineTonemapDebugData::default());
        resources.insert(DebugUiState::default());

        let asset_source = args.asset_source().unwrap();

        let physical_size = window.inner_size();
        init::rendering_init(
            &mut resources,
            asset_source,
            window,
            physical_size.width,
            physical_size.height,
        )?;

        let world = World::default();
        let print_time_event = crate::time::PeriodicEvent::default();

        let font = {
            let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
            let font = asset_resource.load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf");
            resources
                .get_mut::<AssetManager>()
                .unwrap()
                .wait_for_asset_to_load(&font, &mut *asset_resource, "demo font")?;
            font
        };

        Ok(DemoApp {
            scene_manager,
            resources,
            world,
            print_time_event,
            font,
        })
    }

    fn update(
        &mut self,
        window: &winit::window::Window,
    ) -> RafxResult<winit::event_loop::ControlFlow> {
        profiling::scope!("Main Loop");

        let t0 = rafx::base::Instant::now();

        //
        // Update time
        //
        {
            self.resources.get_mut::<TimeState>().unwrap().update();
        }

        //
        // Print FPS
        //
        {
            let time_state = self.resources.get::<TimeState>().unwrap();
            if self.print_time_event.try_take_event(
                time_state.current_instant(),
                std::time::Duration::from_secs_f32(1.0),
            ) {
                log::info!("FPS: {}", time_state.updates_per_second());
                //renderer.dump_stats();
            }
        }

        {
            let mut viewports_resource = self.resources.get_mut::<ViewportsResource>().unwrap();
            let physical_size = window.inner_size();
            viewports_resource.main_window_size = RafxExtents2D {
                width: physical_size.width,
                height: physical_size.height,
            };
        }

        {
            if self.scene_manager.has_next_scene() {
                self.scene_manager
                    .try_cleanup_current_scene(&mut self.world, &self.resources);

                {
                    // NOTE(dvd): Legion leaks memory because the entity IDs aren't reset when the
                    // world is cleared and the entity location map will grow without bounds.
                    self.world = World::default();

                    // NOTE(dvd): The Renderer maintains some per-frame temporary data to avoid
                    // allocating each frame. We can clear this between scene transitions.
                    let mut renderer = self.resources.get_mut::<Renderer>().unwrap();
                    renderer.clear_temporary_work();
                }

                *self.resources.get_mut::<MeshRenderOptions>().unwrap() = Default::default();
                *self.resources.get_mut::<RenderOptions>().unwrap() = RenderOptions::default_3d();

                self.scene_manager
                    .try_create_next_scene(&mut self.world, &self.resources);
            }
        }

        //
        // Update assets
        //
        {
            profiling::scope!("update asset resource");
            let mut asset_resource = self.resources.get_mut::<AssetResource>().unwrap();
            asset_resource.update();
        }

        //
        // Update graphics resources
        //
        {
            profiling::scope!("update asset loaders");
            let mut asset_manager = self.resources.get_mut::<AssetManager>().unwrap();

            asset_manager.update_asset_loaders().unwrap();
        }

        //
        // Notify egui of frame begin
        //
        #[cfg(feature = "egui")]
        {
            let egui_manager = self.resources.get::<WinitEguiManager>().unwrap();
            egui_manager.begin_frame(window)?;
        }

        {
            let mut text_resource = self.resources.get_mut::<TextResource>().unwrap();

            text_resource.add_text(
                "Use Left/Right arrow keys to switch demos".to_string(),
                glam::Vec3::new(100.0, 400.0, 0.0),
                &self.font,
                20.0,
                glam::Vec4::new(1.0, 1.0, 1.0, 1.0),
            );
        }

        {
            self.scene_manager
                .update_scene(&mut self.world, &mut self.resources);
        }

        #[cfg(feature = "egui")]
        {
            let ctx = self
                .resources
                .get::<EguiContextResource>()
                .unwrap()
                .context();
            let time_state = self.resources.get::<TimeState>().unwrap();
            let mut debug_ui_state = self.resources.get_mut::<DebugUiState>().unwrap();
            let mut render_options = self.resources.get_mut::<RenderOptions>().unwrap();
            let tonemap_debug_data = self.resources.get::<PipelineTonemapDebugData>().unwrap();
            let asset_resource = self.resources.get::<AssetResource>().unwrap();

            egui::TopBottomPanel::top("top_panel").show(&ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "Windows", |ui| {
                        ui.checkbox(&mut debug_ui_state.show_render_options, "Render Options");

                        ui.checkbox(&mut debug_ui_state.show_asset_list, "Asset List");
                        ui.checkbox(&mut debug_ui_state.show_tonemap_debug, "Tonemap Debug");
                        ui.checkbox(
                            &mut debug_ui_state.show_shadow_map_debug,
                            "Shadow Map Debug",
                        );

                        #[cfg(feature = "profile-with-puffin")]
                        if ui
                            .checkbox(&mut debug_ui_state.show_profiler, "Profiler")
                            .changed()
                        {
                            log::info!(
                                "Setting puffin profiler enabled: {:?}",
                                debug_ui_state.show_profiler
                            );
                            profiling::puffin::set_scopes_on(debug_ui_state.show_profiler);
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        ui.label(format!("Frame: {}", time_state.update_count()));
                        ui.separator();
                        ui.label(format!(
                            "FPS: {:.1}",
                            time_state.updates_per_second_smoothed()
                        ));
                    });
                })
            });

            if debug_ui_state.show_tonemap_debug {
                egui::Window::new("Tonemap Debug")
                    .open(&mut debug_ui_state.show_tonemap_debug)
                    .show(&ctx, |ui| {
                        let data = tonemap_debug_data.inner.lock().unwrap();

                        ui.add(egui::Label::new(format!(
                            "histogram_sample_count: {}",
                            data.histogram_sample_count
                        )));
                        ui.add(egui::Label::new(format!(
                            "histogram_max_value: {}",
                            data.histogram_max_value
                        )));

                        use egui::plot::{Line, Plot, VLine, Value, Values};
                        let line_values: Vec<_> = data
                            .histogram
                            .iter()
                            //.skip(1) // don't include index 0
                            .enumerate()
                            .map(|(i, value)| Value::new(i as f64, *value as f64))
                            .collect();
                        let line =
                            Line::new(Values::from_values_iter(line_values.into_iter())).fill(0.0);
                        let average_line = VLine::new(data.result_average_bin);
                        let low_line = VLine::new(data.result_low_bin);
                        let high_line = VLine::new(data.result_high_bin);
                        Some(
                            ui.add(
                                Plot::new("my_plot")
                                    .line(line)
                                    .vline(average_line)
                                    .vline(low_line)
                                    .vline(high_line)
                                    .include_y(0.0)
                                    .include_y(1.0)
                                    .show_axes([false, false]),
                            ),
                        )
                    });
            }

            tonemap_debug_data
                .inner
                .lock()
                .unwrap()
                .enable_debug_data_collection = debug_ui_state.show_tonemap_debug;

            if debug_ui_state.show_render_options {
                egui::Window::new("Render Options")
                    .open(&mut debug_ui_state.show_render_options)
                    .show(&ctx, |ui| {
                        render_options.ui(ui);
                    });
            }

            if debug_ui_state.show_shadow_map_debug {
                egui::Window::new("Shadow map Debug")
                    .open(&mut debug_ui_state.show_shadow_map_debug)
                    .show(&ctx, |ui| {
                        //TODO: Build a UI for this
                        ui.add(egui::Label::new("test"));
                    });
            }

            if debug_ui_state.show_asset_list {
                egui::Window::new("Asset List")
                    .open(&mut debug_ui_state.show_asset_list)
                    .show(&ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let loader = asset_resource.loader();
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
                                    ui.label(format!(
                                        "{}:{} .. {}",
                                        info.file_name.unwrap_or_else(|| "???".to_string()),
                                        info.asset_name.unwrap_or_else(|| format!("{}", id)),
                                        info.refs
                                    ));
                                } else {
                                    ui.label("NO INFO");
                                }
                            }
                        });
                    });
            }

            #[cfg(feature = "profile-with-puffin")]
            if debug_ui_state.show_profiler {
                profiling::scope!("puffin profiler");
                puffin_egui::profiler_window(&ctx);
            }

            let mut render_config_resource =
                self.resources.get_mut::<RendererConfigResource>().unwrap();
            render_config_resource
                .visibility_config
                .enable_visibility_update = render_options.enable_visibility_update;
        }

        {
            let render_options = self.resources.get::<RenderOptions>().unwrap();

            let mut basic_pipeline_render_options =
                self.resources.get_mut::<PipelineRenderOptions>().unwrap();
            basic_pipeline_render_options.enable_msaa = render_options.enable_msaa;
            basic_pipeline_render_options.enable_hdr = render_options.enable_hdr;
            basic_pipeline_render_options.enable_bloom = render_options.enable_bloom;
            basic_pipeline_render_options.enable_textures = render_options.enable_textures;
            basic_pipeline_render_options.show_surfaces = render_options.show_surfaces;
            basic_pipeline_render_options.show_wireframes = render_options.show_wireframes;
            basic_pipeline_render_options.show_debug3d = render_options.show_debug3d;
            basic_pipeline_render_options.show_text = render_options.show_text;
            basic_pipeline_render_options.show_skybox = render_options.show_skybox;
            basic_pipeline_render_options.show_feature_toggles =
                render_options.show_feature_toggles;
            basic_pipeline_render_options.blur_pass_count = render_options.blur_pass_count;
            basic_pipeline_render_options.tonemapper_type = render_options.tonemapper_type;
            basic_pipeline_render_options.enable_visibility_update =
                render_options.enable_visibility_update;

            let mut mesh_render_options = self.resources.get_mut::<MeshRenderOptions>().unwrap();
            mesh_render_options.show_surfaces = render_options.show_surfaces;
            mesh_render_options.show_shadows = render_options.show_shadows;
            mesh_render_options.enable_lighting = render_options.enable_lighting;
        }

        //
        // Close egui input for this frame
        //
        #[cfg(feature = "egui")]
        {
            let egui_manager = self.resources.get::<WinitEguiManager>().unwrap();
            egui_manager.end_frame();
        }

        let t1 = rafx::base::Instant::now();
        log::trace!(
            "[main] Simulation took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        //
        // Redraw
        //
        {
            let dt = self
                .resources
                .get::<TimeState>()
                .unwrap()
                .previous_update_time();

            profiling::scope!("Start Next Frame Render");
            let renderer = self.resources.get::<Renderer>().unwrap();

            let mut extract_resources = ExtractResources::default();

            macro_rules! add_to_extract_resources {
                ($ty: ident) => {
                    #[allow(non_snake_case)]
                    let mut $ty = self.resources.get_mut::<$ty>().unwrap();
                    extract_resources.insert(&mut *$ty);
                };
                ($ty: path, $name: ident) => {
                    let mut $name = self.resources.get_mut::<$ty>().unwrap();
                    extract_resources.insert(&mut *$name);
                };
            }

            add_to_extract_resources!(VisibilityRegion);
            add_to_extract_resources!(RafxSwapchainHelper);
            add_to_extract_resources!(ViewportsResource);
            add_to_extract_resources!(AssetManager);
            add_to_extract_resources!(TimeState);
            add_to_extract_resources!(RenderOptions);
            add_to_extract_resources!(PipelineRenderOptions);
            add_to_extract_resources!(PipelineTonemapDebugData);
            add_to_extract_resources!(MeshRenderOptions);
            add_to_extract_resources!(RendererConfigResource);
            add_to_extract_resources!(TileLayerResource);
            add_to_extract_resources!(SkyboxResource);
            add_to_extract_resources!(
                rafx_plugins::features::sprite::SpriteRenderObjectSet,
                sprite_render_object_set
            );
            add_to_extract_resources!(MeshRenderObjectSet, mesh_render_object_set);
            add_to_extract_resources!(
                rafx_plugins::features::tile_layer::TileLayerRenderObjectSet,
                tile_layer_render_object_set
            );
            add_to_extract_resources!(
                rafx_plugins::features::debug3d::Debug3DResource,
                debug_draw_3d_resource
            );
            add_to_extract_resources!(
                rafx_plugins::features::debug_pip::DebugPipResource,
                debug_pip_resource
            );
            add_to_extract_resources!(rafx_plugins::features::text::TextResource, text_resource);

            #[cfg(feature = "egui")]
            add_to_extract_resources!(
                rafx_plugins::features::egui::WinitEguiManager,
                winit_egui_manager
            );

            extract_resources.insert(&mut self.world);

            renderer
                .start_rendering_next_frame(&mut extract_resources, dt)
                .unwrap();
        }

        let t2 = rafx::base::Instant::now();
        log::trace!(
            "[main] start rendering took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        profiling::finish_frame!();

        {
            let mut input_resource = self.resources.get_mut::<InputResource>().unwrap();
            input_resource.end_frame();
        }

        Ok(ControlFlow::Poll)
    }

    fn process_input(
        &mut self,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    ) -> bool {
        Self::do_process_input(
            &mut self.scene_manager,
            &mut self.world,
            &self.resources,
            event,
            window,
        )
    }

    fn do_process_input(
        scene_manager: &mut SceneManager,
        world: &mut World,
        resources: &Resources,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    ) -> bool {
        use winit::event::*;

        #[cfg(feature = "egui")]
        let egui_manager = resources
            .get::<rafx_plugins::features::egui::WinitEguiManager>()
            .unwrap();

        #[cfg(feature = "egui")]
        let ignore_event = {
            egui_manager.handle_event(event);
            egui_manager.ignore_event(event)
        };

        #[cfg(not(feature = "egui"))]
        let ignore_event = false;

        if !ignore_event {
            //log::trace!("{:?}", event);
            let mut was_handled = false;
            match event {
                //
                // Halt if the user requests to close the window
                //
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => return false,

                //
                // Close if the escape key is hit
                //
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(virtual_keycode),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    //log::trace!("Key Down {:?} {:?}", keycode, modifiers);
                    if *virtual_keycode == VirtualKeyCode::Escape {
                        return false;
                    }

                    if *virtual_keycode == VirtualKeyCode::Left {
                        scene_manager.queue_load_previous_scene();
                        was_handled = true;
                    }

                    if *virtual_keycode == VirtualKeyCode::Right {
                        scene_manager.queue_load_next_scene();
                        was_handled = true;
                    }

                    if *virtual_keycode == VirtualKeyCode::G {
                        window
                            .set_cursor_grab(true)
                            .expect("Failed to grab mouse cursor");
                    }

                    if *virtual_keycode == VirtualKeyCode::M {
                        let metrics = resources.get::<AssetManager>().unwrap().metrics();
                        println!("{:#?}", metrics);
                        was_handled = true;
                    }
                }
                _ => {}
            }

            if !was_handled {
                scene_manager.process_input(world, resources, event);

                {
                    let mut input_resource = resources.get_mut::<InputResource>().unwrap();
                    input::handle_winit_event(event, &mut *input_resource);
                }
            }
        }

        true
    }
}

impl Drop for DemoApp {
    fn drop(&mut self) {
        init::rendering_destroy(&mut self.resources).unwrap()
    }
}

pub fn update_loop(
    args: &DemoArgs,
    window: winit::window::Window,
    event_loop: winit::event_loop::EventLoop<()>,
) -> RafxResult<()> {
    log::debug!("calling init");
    let mut app = DemoApp::init(args, &window).unwrap();

    log::debug!("start update loop");
    event_loop.run(move |event, _, control_flow| {
        use winit::event::Event;
        match event {
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                *control_flow = app.update(&window).unwrap();
            }
            event @ _ => {
                if !app.process_input(&event, &window) {
                    *control_flow = ControlFlow::Exit;
                }
            }
        }
    });
}
