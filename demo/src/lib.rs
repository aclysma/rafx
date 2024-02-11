// There's a decent amount of code that's just for example and isn't called
#![allow(dead_code)]

mod main_native;

pub use main_native::*;
use std::path::PathBuf;

use legion::*;
use structopt::StructOpt;

use rafx::api::{RafxExtents2D, RafxResult, RafxSwapchainHelper};
use rafx::assets::AssetManager;

use crate::scenes::SceneManager;
use crate::time::{PeriodicEvent, TimeState};
use rafx::assets::AssetResource;
use rafx::render_features::ExtractResources;
use rafx::renderer::{AssetSource, Renderer};
use rafx::renderer::{RendererConfigResource, ViewportsResource};
use rafx::visibility::VisibilityResource;

mod demo_ui;
mod init;
mod input;
mod scenes;
mod time;

mod demo_renderer_thread_pool;

use crate::input::InputResource;
use demo_ui::*;
use hydrate_base::handle::Handle;
use rafx_plugins::assets::font::FontAsset;
#[cfg(feature = "egui")]
use rafx_plugins::features::egui::WinitEguiManager;
use rafx_plugins::features::skybox::SkyboxResource;
use rafx_plugins::features::text::TextResource;
use rafx_plugins::features::tile_layer::TileLayerResource;
use winit::event_loop::ControlFlow;

#[cfg(feature = "basic-pipeline")]
use rafx_plugins::features::mesh_basic::{
    MeshBasicRenderObjectSet as MeshRenderObjectSet, MeshBasicRenderOptions as MeshRenderOptions,
};
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::pipelines::basic::BasicPipelineRenderOptions as PipelineRenderOptions;

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::features::mesh_adv::{
    MeshAdvRenderObjectSet as MeshRenderObjectSet, MeshAdvRenderOptions as MeshRenderOptions,
};
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::ModernPipelineMeshCullingDebugData;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::{
    ModernPipelineRenderOptions as PipelineRenderOptions, ModernPipelineTonemapDebugData,
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

fn default_build_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../demo-editor/data/build_data"
    ))
}

#[derive(StructOpt)]
pub struct DemoArgs {
    /// Path to the packfile
    #[structopt(name = "packfile", long, parse(from_os_str))]
    pub build_dir: Option<std::path::PathBuf>,

    #[structopt(skip)]
    pub packbuffer: Option<&'static [u8]>,

    #[structopt(name = "external-daemon", long)]
    pub external_daemon: bool,
}

impl DemoArgs {
    fn asset_source(&self) -> Option<AssetSource> {
        if let Some(build_dir) = &self.build_dir {
            return Some(AssetSource::BuildDir(build_dir.clone()));
        } else {
            return Some(AssetSource::BuildDir(default_build_dir()));
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
        profiling::tracy_client::Client::start();
        #[cfg(feature = "profile-with-tracy")]
        profiling::tracy_client::set_thread_name!("Main Thread");
        #[cfg(feature = "profile-with-optick")]
        profiling::optick::register_thread("Main Thread");

        let scene_manager = SceneManager::default();

        let mut resources = Resources::default();
        resources.insert(TimeState::new());
        resources.insert(InputResource::new());
        resources.insert(RenderOptions::default_2d());
        resources.insert(MeshRenderOptions::default());
        resources.insert(PipelineRenderOptions::default());
        #[cfg(not(feature = "basic-pipeline"))]
        resources.insert(ModernPipelineTonemapDebugData::default());
        #[cfg(not(feature = "basic-pipeline"))]
        resources.insert(ModernPipelineMeshCullingDebugData::default());
        resources.insert(DebugUiState::default());

        let asset_source = args.asset_source().unwrap();

        let physical_size = window.inner_size();
        init::rendering_init(
            &mut resources,
            asset_source,
            window,
            window,
            physical_size.width,
            physical_size.height,
        )?;

        let world = World::default();
        let print_time_event = crate::time::PeriodicEvent::default();

        let font = {
            let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
            let font = asset_resource.load_asset_symbol_name::<FontAsset>(
                "assets://rafx-plugins/fonts/mplus-1p-regular.ttf",
            );
            let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
            let renderer = resources.get::<Renderer>().unwrap();

            renderer.wait_for_asset_to_load(
                &mut asset_manager,
                &font,
                &mut *asset_resource,
                "demo font",
            )?;

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
                    renderer.wait_for_render_thread_idle();
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
            let debug_ui_state = self.resources.get::<RenderOptions>().unwrap();
            if debug_ui_state.show_lights_debug_draw {
                crate::scenes::util::add_light_debug_draw(&self.resources, &self.world);
            }
        }

        {
            self.scene_manager
                .update_scene(&mut self.world, &mut self.resources);
        }

        #[cfg(feature = "egui")]
        demo_ui::draw_ui(&self.resources);

        {
            let render_options = self.resources.get::<RenderOptions>().unwrap();

            let mut pipeline_render_options =
                self.resources.get_mut::<PipelineRenderOptions>().unwrap();
            pipeline_render_options.anti_alias_method = render_options.anti_alias_method;
            pipeline_render_options.enable_hdr = render_options.enable_hdr;
            pipeline_render_options.enable_bloom = render_options.enable_bloom;
            pipeline_render_options.enable_textures = render_options.enable_textures;
            pipeline_render_options.show_surfaces = render_options.show_surfaces;
            pipeline_render_options.show_wireframes = render_options.show_wireframes;
            pipeline_render_options.show_debug3d = render_options.show_debug3d;
            pipeline_render_options.show_text = render_options.show_text;
            pipeline_render_options.show_skybox = render_options.show_skybox;
            pipeline_render_options.show_feature_toggles = render_options.show_feature_toggles;
            pipeline_render_options.blur_pass_count = render_options.blur_pass_count;
            pipeline_render_options.tonemapper_type = render_options.tonemapper_type;
            pipeline_render_options.enable_visibility_update =
                render_options.enable_visibility_update;
            #[cfg(not(feature = "basic-pipeline"))]
            {
                pipeline_render_options.enable_ssao = render_options.enable_ssao;
                pipeline_render_options.taa_options = render_options.taa_options.clone();
                pipeline_render_options.enable_sharpening = render_options.enable_sharpening;
                pipeline_render_options.sharpening_amount = render_options.sharpening_amount;
                pipeline_render_options.enable_occlusion_culling =
                    render_options.enable_occlusion_culling;
            }

            let mut render_config_resource =
                self.resources.get_mut::<RendererConfigResource>().unwrap();
            render_config_resource
                .visibility_config
                .enable_visibility_update = render_options.enable_visibility_update;

            let mut mesh_render_options = self.resources.get_mut::<MeshRenderOptions>().unwrap();
            mesh_render_options.show_surfaces = render_options.show_surfaces;
            mesh_render_options.show_shadows = render_options.show_shadows;
            mesh_render_options.enable_lighting = render_options.enable_lighting;
            #[cfg(not(feature = "basic-pipeline"))]
            {
                mesh_render_options.ndf_filter_amount = render_options.ndf_filter_amount;
                mesh_render_options.use_clustered_lighting = render_options.use_clustered_lighting;
            }
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

            add_to_extract_resources!(VisibilityResource);
            add_to_extract_resources!(RafxSwapchainHelper);
            add_to_extract_resources!(ViewportsResource);
            add_to_extract_resources!(AssetManager);
            add_to_extract_resources!(AssetResource);
            add_to_extract_resources!(TimeState);
            add_to_extract_resources!(RenderOptions);
            add_to_extract_resources!(PipelineRenderOptions);
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

            #[cfg(not(feature = "basic-pipeline"))]
            add_to_extract_resources!(ModernPipelineTonemapDebugData);
            #[cfg(not(feature = "basic-pipeline"))]
            add_to_extract_resources!(ModernPipelineMeshCullingDebugData);

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
        _window: &winit::window::Window,
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

                    //if *virtual_keycode == VirtualKeyCode::G {
                    //    window
                    //        .set_cursor_grab(true)
                    //        .expect("Failed to grab mouse cursor");
                    //}

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
