use log::LevelFilter;

use crate::demo_plugin::DemoRendererPlugin;
use crate::demo_render_graph_generator::DemoRenderGraphGenerator;
use crate::features::{DemoRenderFeature, DemoRenderFeaturePlugin};
use crate::phases::OpaqueRenderPhase;
use glam::Vec3;
use legion::{Resources, World};
use rafx::api::*;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::framework::render_features::{
    ExtractResources, RenderFeatureFlagMask, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder,
    RenderRegistry, RenderViewDepthRange,
};
use rafx::framework::visibility::VisibilityRegion;
use rafx::rafx_visibility::{DepthRange, OrthographicParameters, Projection};
use rafx::renderer::{
    AssetSource, RenderViewMeta, Renderer, RendererBuilder, RendererConfigResource,
    SwapchainHandler, ViewportsResource,
};
use rafx_renderer::daemon::AssetDaemonOpt;
use std::sync::Arc;
use std::time;

mod demo_plugin;
mod demo_render_graph_generator;
mod features;
mod phases;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 600;

#[derive(Clone)]
pub struct TimeState {
    app_start_instant: time::Instant,
    now_instant: time::Instant,
}

impl TimeState {
    pub fn new() -> TimeState {
        let now_instant = time::Instant::now();

        TimeState {
            app_start_instant: now_instant,
            now_instant,
        }
    }

    pub fn update(&mut self) {
        self.now_instant = time::Instant::now();
    }

    pub fn total_time(&self) -> time::Duration {
        self.now_instant.duration_since(self.app_start_instant)
    }

    pub fn current_instant(&self) -> time::Instant {
        self.now_instant
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Info)
        .init();

    run().unwrap();
}

fn run() -> RafxResult<()> {
    //
    // Init SDL2 (winit and anything that uses raw-window-handle works too!)
    //
    let sdl2_systems = sdl2_init();

    //
    // Create the api. GPU programming is fundamentally unsafe, so all rafx APIs should be
    // considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    // behavior on the CPU for reasons other than interacting with the GPU.
    //
    let api = unsafe { RafxApi::new(&sdl2_systems.window, &Default::default())? };
    let mut resources = Resources::default();
    resources.insert(TimeState::new());

    // Wrap all of this so that it gets dropped before we drop the API object. This ensures a nice
    // clean shutdown.
    {
        //
        // For this example, we'll run the `distill` daemon in-process. This is the most convenient
        // method during development. (You could also build a packfile ahead of time and run from that)
        //
        let db_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples/renderer_triangle/.assets_db");
        let asset_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples/renderer_triangle/assets");
        let connect_string = "127.0.0.1:9999";

        let asset_source = AssetSource::Daemon {
            external_daemon: false,
            daemon_args: AssetDaemonOpt {
                db_dir,
                address: connect_string.parse().unwrap(),
                asset_dirs: [asset_dir].to_vec(),
            },
        };

        let sdl2_window = &sdl2_systems.window;

        resources.insert(VisibilityRegion::new());
        resources.insert(ViewportsResource::default());

        let demo_render_feature_plugin = Arc::new(DemoRenderFeaturePlugin::default());
        demo_render_feature_plugin.legion_init(&mut resources);

        let mut renderer_builder = RendererBuilder::default();
        renderer_builder = renderer_builder
            .add_asset(Arc::new(DemoRendererPlugin))
            .add_render_feature(demo_render_feature_plugin);

        let mut renderer_builder_result = {
            let extract_resources = ExtractResources::default();

            let render_graph_generator = Box::new(DemoRenderGraphGenerator);

            renderer_builder.build(
                extract_resources,
                &api,
                asset_source,
                render_graph_generator,
                || None,
            )
        }?;

        let (width, height) = sdl2_window.vulkan_drawable_size();
        let swapchain_helper = SwapchainHandler::create_swapchain(
            &mut renderer_builder_result.asset_manager,
            &mut renderer_builder_result.renderer,
            sdl2_window,
            width,
            height,
        )?;

        resources.insert(api.device_context());
        resources.insert(api);
        resources.insert(swapchain_helper);
        resources.insert(renderer_builder_result.asset_resource);
        resources.insert(
            renderer_builder_result
                .asset_manager
                .resource_manager()
                .render_registry()
                .clone(),
        );
        resources.insert(renderer_builder_result.asset_manager);
        resources.insert(renderer_builder_result.renderer);
        resources.insert(RendererConfigResource::default());

        let mut world = World::default();

        let main_view_frustum = {
            let visibility_region = resources.get::<VisibilityRegion>().unwrap();
            visibility_region.register_view_frustum()
        };

        //
        // SDL2 window pumping
        //
        log::info!("Starting window event loop");
        let mut event_pump = sdl2_systems
            .context
            .event_pump()
            .expect("Could not create sdl event pump");

        'running: loop {
            if !process_input(&mut event_pump) {
                break 'running;
            }

            {
                let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
                let (width, height) = sdl2_systems.window.vulkan_drawable_size();
                viewports_resource.main_window_size = RafxExtents2D { width, height };

                let main_camera_phase_mask = RenderPhaseMaskBuilder::default()
                    .add_render_phase::<OpaqueRenderPhase>()
                    .build();

                let main_camera_feature_mask = RenderFeatureMaskBuilder::default()
                    .add_render_feature::<DemoRenderFeature>()
                    .build();

                const CAMERA_Z: f32 = 1000.0;

                let eye = Vec3::new(0., 0., CAMERA_Z);

                let half_width = viewports_resource.main_window_size.width as f32 / 2.0;
                let half_height = viewports_resource.main_window_size.height as f32 / 2.0;

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
                    render_phase_mask: main_camera_phase_mask,
                    render_feature_mask: main_camera_feature_mask,
                    render_feature_flag_mask: RenderFeatureFlagMask::empty(),
                    debug_name: "main".to_string(),
                })
            }

            {
                let mut time_state = resources.get_mut::<TimeState>().unwrap();
                time_state.update();
            }

            {
                let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
                asset_resource.update();
            }

            {
                let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
                asset_manager.update_asset_loaders().unwrap();
            }

            {
                let renderer = resources.get::<Renderer>().unwrap();

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
                add_to_extract_resources!(RendererConfigResource);

                extract_resources.insert(&mut world);

                renderer.start_rendering_next_frame(&mut extract_resources)?;
            }
        }
    }

    // Destroy these first
    {
        {
            let swapchain_helper = resources.remove::<RafxSwapchainHelper>().unwrap();
            let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
            let renderer = resources.get::<Renderer>().unwrap();
            SwapchainHandler::destroy_swapchain(swapchain_helper, &mut *asset_manager, &*renderer)?;
        }

        resources.remove::<Renderer>();

        DemoRenderFeaturePlugin::legion_destroy(&mut resources);

        resources.remove::<RenderRegistry>();

        // Remove the asset resource because we have asset storages that reference resources
        resources.remove::<AssetResource>();

        resources.remove::<AssetManager>();
        resources.remove::<RafxDeviceContext>();
    }

    // Optional, but calling this verifies that all rafx objects/device contexts have been
    // destroyed and where they were created. Good for finding unintended leaks!
    let mut api = resources.remove::<RafxApi>().unwrap();
    std::mem::drop(resources);
    api.destroy()?;

    Ok(())
}

//
// SDL2 helpers
//
pub struct Sdl2Systems {
    pub context: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
}

pub fn sdl2_init() -> Sdl2Systems {
    // Setup SDL
    let context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Create the window
    let window = video_subsystem
        .window("Rafx Example", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .allow_highdpi()
        .resizable()
        .build()
        .expect("Failed to create window");

    Sdl2Systems {
        context,
        video_subsystem,
        window,
    }
}

fn process_input(event_pump: &mut sdl2::EventPump) -> bool {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;

    for event in event_pump.poll_iter() {
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
            }

            _ => {}
        }
    }

    true
}
