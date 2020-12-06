use ash::prelude::VkResult;
use ash::vk;
use log::LevelFilter;
use rafx::graph::{
    RenderGraphBuilder, RenderGraphExecutor, RenderGraphImageConstraint, RenderGraphImageExtents,
    RenderGraphImageSpecification, RenderGraphImageUsageId, RenderGraphNodeCallbacks,
    RenderGraphQueue,
};
use rafx::resources::vk_description::SwapchainSurfaceInfo;
use rafx::resources::{vk_description as dsc, ResourceArc};
use rafx::resources::{ImageResource, ResourceManager};
use rafx_shell_vulkan::{
    FrameInFlight, MsaaLevel, VkContextBuilder, VkDeviceContext, VkImageRaw, VkSurface, Window,
};

use rafx_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::EventPump;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 600;

pub struct Sdl2Systems {
    pub context: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
}

// Setup SDL2 and create a window
pub fn sdl2_init() -> Sdl2Systems {
    // Setup SDL
    let context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Create the window
    let window = video_subsystem
        .window("Render Graph Example", WINDOW_WIDTH, WINDOW_HEIGHT)
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

fn main() {
    // Turn on some logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Warn)
        .init();

    // Set up SDL2
    let sdl2_systems = sdl2_init();

    // Wrap the SDL2 window for the renderer to use. (allows us to support other windowing libraries
    // like winit
    let window_wrapper = Sdl2Window::new(&sdl2_systems.window);

    // Create the SDL2 event loop
    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems
        .context
        .event_pump()
        .expect("Could not create sdl event pump");

    run(&window_wrapper, &mut event_pump).unwrap();
}

fn run(
    window: &dyn Window,
    event_pump: &mut EventPump,
) -> VkResult<()> {
    // This is used for the material system which is not part of this sample.
    let render_registry = rafx::nodes::RenderRegistryBuilder::default().build();

    // The context sets up the instance and device. This object will tear down all vulkan
    // initialization when dropped. You generally just want one of these.
    let vk_context = VkContextBuilder::new()
        .use_vulkan_debug_layer(true)
        .msaa_level_priority(vec![MsaaLevel::Sample1])
        .prefer_mailbox_present_mode()
        .build(window)
        .unwrap();

    // The device context is a cloneable, multi-threading friendly accessor into what was created by
    // the context
    let device_context = vk_context.device_context().clone();

    // The resource manager sets up reference counting/hashing for most vulkan objects as well as
    // some multi-threading friendly helpers for creating dynamic resources. It also includes hooks
    // for atelier assets to register data. (But atelier assets is not used in this example)
    let mut resource_manager = ResourceManager::new(&device_context, &render_registry);

    // The surface is associated with a window and handles creating the swapchain and re-creating it
    // if the swapchain becomes out of date (commonly due to window resize)
    let mut surface = VkSurface::new(&vk_context, window, None).unwrap();

    // The surface creates the swapchain immediately, and that swapchain contains images. We
    // register those images with the resource manager so that other systems can handle them like
    // reference counted images we might create.
    let mut swapchain_images = register_swapchain_images(&mut resource_manager, &mut surface);

    loop {
        // Process input, mainly to quit when hitting escape
        if !process_input(&device_context, &resource_manager, event_pump) {
            break;
        }

        // Get the swapchain image. This will block until it is available.  The returned
        // FrameInFlight must be submitted or cancelled to free up an image. An error returned here
        // might be from a previous swapchain cancel_present call that happened in another thread.
        // This design is meant to be multithread friendly. (i.e. allows simulating frame N + 1 while
        // still rendering frame N). Multithreading is not part of the demo and some of the work
        // in render_frame() would need to happen before spawning a job to finish rendering. (The
        // graph would need to be built before returning but executing the graph could happen
        // asynchronously.
        let frame_in_flight_result = surface.acquire_next_swapchain_image(window);
        match frame_in_flight_result {
            Ok(frame_in_flight) => {
                render_frame(
                    &device_context,
                    &mut resource_manager,
                    &mut surface,
                    swapchain_images.as_slice(),
                    frame_in_flight,
                )?;

                // Update graphics resources, this mainly handles dropping resources that are no
                // longer in use.
                resource_manager.on_frame_complete()
            }
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(ash::vk::Result::SUBOPTIMAL_KHR) => {
                surface.rebuild_swapchain(window, None)?;

                // Rebuild the image list since it's now stale
                swapchain_images = register_swapchain_images(&mut resource_manager, &mut surface);
                Ok(())
            }
            Err(e) => {
                log::warn!("Unexpected rendering error");
                return Err(e);
            }
        }?;
    }

    surface.tear_down(None);
    Ok(())
}

// This takes all the swapchain images, registers them with the resource manager, and returns the
// ResourceArcs. While ResourceArcs are reference counted, the swapchain images still belong to the
// swapchain.
fn register_swapchain_images(
    resource_manager: &mut ResourceManager,
    surface: &mut VkSurface,
) -> Vec<ResourceArc<ImageResource>> {
    surface
        .swapchain()
        .swapchain_images
        .iter()
        .map(|&image| {
            // Register the swapchain image as an image resource. This lets us pass it to the render graph
            resource_manager.resources().insert_raw_image(VkImageRaw {
                image,
                allocation: None,
            })
        })
        .collect()
}

fn process_input(
    device_context: &VkDeviceContext,
    resource_manager: &ResourceManager,
    event_pump: &mut sdl2::EventPump,
) -> bool {
    for event in event_pump.poll_iter() {
        //log::trace!("{:?}", event);
        match event {
            // Halt if the user requests to close the window
            Event::Quit { .. } => return false,

            Event::KeyDown {
                keycode: Some(keycode),
                keymod: _modifiers,
                ..
            } => {
                // Close if the escape key is hit
                if keycode == Keycode::Escape {
                    return false;
                }

                // Dump memory stats
                if keycode == Keycode::D {
                    let stats = device_context.allocator().calculate_stats().unwrap();
                    println!("{:#?}", stats);
                }

                // Show resource usage metrics
                if keycode == Keycode::M {
                    let metrics = resource_manager.metrics();
                    println!("{:#?}", metrics);
                }
            }

            _ => {}
        }
    }

    true
}

fn render_frame(
    device_context: &VkDeviceContext,
    resource_manager: &mut ResourceManager,
    surface: &mut VkSurface,
    swapchain_images: &[ResourceArc<ImageResource>],
    frame_in_flight: FrameInFlight,
) -> VkResult<()> {
    // Gather info for the frame to use
    let swapchain_surface_info = SwapchainSurfaceInfo {
        color_format: surface.swapchain().swapchain_info.color_format,
        depth_format: surface.swapchain().swapchain_info.depth_format,
        extents: surface.swapchain().swapchain_info.extents,
        msaa_level: surface.swapchain().swapchain_info.msaa_level,
        surface_format: surface.swapchain().swapchain_info.surface_format,
    };

    // Create an image view for the swapchain. These will actually get cached/reused because the
    // rendergraph will cache renderpasses/framebuffers for a few frames, which keep image views
    // alive. "Recreating" an image view of the same image with the same parameters in this case
    // will just fetch the same view we created previously (unless the swapchain changes!)
    let swapchain_image_view = resource_manager.resources().get_or_create_image_view(
        &swapchain_images[frame_in_flight.present_index() as usize],
        &dsc::ImageViewMeta::default_2d_no_mips_or_layers(
            swapchain_surface_info.surface_format.format.into(),
            dsc::ImageAspectFlag::Color.into(),
        ),
    )?;

    // Graph callbacks take a user-defined T, allowing you to pass extra state through to the
    // callback.
    struct RenderGraphUserContext {}

    // Create an empty graph and callback set. These are later fed to an "executor" object which
    // will iterate across the graph, start renderpasses, and dispatch callbacks.
    let mut graph = RenderGraphBuilder::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphUserContext>::default();

    // Create a basic cleared screen. The node IDs and image IDs returned here can be used later. In
    // this example we will associate the color attachment with the swapchain image, which is what
    // causes it to render to screen.
    let opaque_pass = {
        struct Opaque {
            color: RenderGraphImageUsageId,
        }

        let node = graph.add_node("Opaque", RenderGraphQueue::DefaultGraphics);
        let color = graph.create_color_attachment(
            node,
            0,
            Some(vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            }),
            RenderGraphImageConstraint {
                samples: Some(swapchain_surface_info.msaa_level.into()),
                format: Some(swapchain_surface_info.surface_format.format.into()),
                ..Default::default()
            },
        );
        graph.set_image_name(color, "color");

        // Set up a callback for when we dispatch the opaque pass. This will happen during execute_graph()
        graph_callbacks.set_renderpass_callback(node, |_args, _context| {
            //TODO: Draw triangle into command buffer
            Ok(())
        });

        Opaque { color }
    };

    // Associate the color attachment output data with the swapchain image for this frame
    graph.set_output_image(
        opaque_pass.color,
        swapchain_image_view,
        RenderGraphImageSpecification {
            samples: swapchain_surface_info.msaa_level.into(),
            format: swapchain_surface_info.surface_format.format.into(),
            aspect_flags: vk::ImageAspectFlags::COLOR,
            usage_flags: surface.swapchain().swapchain_info.image_usage_flags,
            create_flags: Default::default(),
            extents: RenderGraphImageExtents::MatchSurface,
            layer_count: 1,
            mip_count: 1,
        },
        Default::default(),
        Default::default(),
        dsc::ImageLayout::PresentSrcKhr,
        vk::AccessFlags::empty(),
        vk::PipelineStageFlags::empty(),
    );

    // Create the executor, it needs to have access to the resource manager to add framebuffers
    // and renderpasses to the resource lookups
    let executor = RenderGraphExecutor::new(
        &device_context,
        &resource_manager.resource_context(),
        graph,
        &swapchain_surface_info,
        graph_callbacks,
    )?;

    // Dispatch the graph, producing command buffers that represent work to queue into the GPU
    // NOTE: This point onward could be performed asynchronously to the next frame.
    let user_context = RenderGraphUserContext {};
    let command_buffers = executor.execute_graph(&user_context)?;

    // Present using the command buffers created via the graph
    frame_in_flight.present(&command_buffers)?;
    Ok(())
}
