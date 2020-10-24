use renderer_shell_vulkan::{
    LogicalSize, VkDevice, VkContextBuilder, MsaaLevel, VkDeviceContext, VkSurface, Window,
    VkImageRaw,
};
use renderer_assets::{ResourceManager, ImageKey};
use renderer_nodes::RenderRegistryBuilder;
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ash::prelude::VkResult;
use sdl2::EventPump;
use log::LevelFilter;
use renderer::assets::{vk_description as dsc, ResourceArc};
use renderer_assets::vk_description::{SwapchainSurfaceInfo, SubpassDescription, FramebufferMeta};
use ash::vk;
use ash::version::DeviceV1_0;
use renderer::assets::graph::{
    RenderGraph, RenderGraphNodeCallbacks, RenderGraphNodeId, RenderGraphImageUsageId,
    RenderGraphImageConstraint, RenderGraphImageSpecification, RenderGraphExecutor,
};
use renderer::vulkan::FrameInFlight;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 600;

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
        .window("Renderer Prototype", WINDOW_WIDTH, WINDOW_HEIGHT)
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
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Warn)
        .init();

    let sdl2_systems = sdl2_init();
    let window_wrapper = Sdl2Window::new(&sdl2_systems.window);

    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems
        .context
        .event_pump()
        .expect("Could not create sdl event pump");

    run(&window_wrapper, &mut event_pump).unwrap();
}

fn run(
    window: &Window,
    event_pump: &mut EventPump,
) -> VkResult<()> {
    let mut context = VkContextBuilder::new()
        .use_vulkan_debug_layer(true)
        .msaa_level_priority(vec![MsaaLevel::Sample1])
        .prefer_mailbox_present_mode();

    let render_registry = renderer::nodes::RenderRegistryBuilder::default()
        //.register_render_phase::<OpaqueRenderPhase>("Opaque")
        .build();

    let vk_context = context.build(window).unwrap();
    let device_context = vk_context.device_context().clone();
    let mut resource_manager =
        renderer::assets::ResourceManager::new(&device_context, &render_registry);

    let mut surface = VkSurface::new(&vk_context, window, None).unwrap();

    let mut swapchain_images = register_swapchain_images(&mut resource_manager, &mut surface);

    loop {
        // Update graphics resources
        resource_manager.update_resources()?;

        // Process input
        if !process_input(&device_context, &resource_manager, event_pump) {
            break;
        }

        // Redraw
        let frame_in_flight_result = surface.acquire_next_swapchain_image(window);
        match frame_in_flight_result {
            Ok(frame_in_flight) => render_frame(
                &device_context,
                &mut resource_manager,
                &mut surface,
                swapchain_images.as_slice(),
                frame_in_flight,
            ),
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

fn register_swapchain_images(mut resource_manager: &mut ResourceManager, surface: &mut VkSurface) -> Vec<(ImageKey, ResourceArc<VkImageRaw>)> {
    surface
        .swapchain()
        .swapchain_images
        .iter()
        .map(|&image| {
            // Register the swapchain image as an image resource. This lets us pass it to the render graph
            resource_manager
                .resources_mut()
                .insert_raw_image(VkImageRaw {
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
                    let stats = device_context.allocator().calculate_stats().unwrap();
                    println!("{:#?}", stats);
                }

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
    swapchain_images: &[(ImageKey, ResourceArc<VkImageRaw>)],
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
    let swapchain_image_view = resource_manager.resources_mut().get_or_create_image_view(
        swapchain_images[frame_in_flight.present_index() as usize].0,
        &dsc::ImageViewMeta::default_2d_no_mips_or_layers(
            swapchain_surface_info.surface_format.format.into(),
            dsc::ImageAspectFlag::Color.into(),
        ),
    )?;

    // Graph callbacks take a user-defined T, allowing you to pass extra state through to the
    // callback.
    struct RenderGraphExecuteContext {
    }

    // Create an empty graph and callback set. These are later fed to an "executor" object which
    // will iterate across the graph, start renderpasses, and dispatch callbacks.
    let mut graph = RenderGraph::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphExecuteContext>::default();

    // Create a basic cleared screen. The node IDs and image IDs returned here can be used later. In
    // this example we will associate the color attachment with the swapchain image, which is what
    // causes it to render to screen.
    let opaque_pass = {
        struct Opaque {
            node_id: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
        }

        let mut node = graph.add_node();
        let node_id = node.id();
        node.set_name("Opaque");
        let color = node.create_color_attachment(
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

        graph_callbacks.set_renderpass_callback(node.id(), |command_buffer, context| {
            //TODO: Draw triangle into command buffer
            Ok(())
        });

        graph.configure_image(color).set_name("color");

        Opaque { node_id, color }
    };

    // Associate the color attachment output data with the swapchain image for this frame
    graph.configure_image(opaque_pass.color).set_output_image(
        swapchain_image_view,
        RenderGraphImageSpecification {
            samples: swapchain_surface_info.msaa_level.into(),
            format: swapchain_surface_info.surface_format.format.into(),
            queue: device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            aspect_flags: vk::ImageAspectFlags::COLOR,
            usage_flags: surface.swapchain().swapchain_info.image_usage_flags,
        },
        dsc::ImageLayout::PresentSrcKhr,
        vk::AccessFlags::empty(),
        vk::PipelineStageFlags::empty(),
        vk::ImageAspectFlags::COLOR,
    );

    // Create the executor, it needs to have access to the resource manager to add framebuffers
    // and renderpasses to the resource lookups
    let mut executor = RenderGraphExecutor::new(
        &device_context,
        graph,
        resource_manager,
        &swapchain_surface_info,
        graph_callbacks,
    )?;

    // Dispatch the graph, producing command buffers that represent work to queue into the GPU
    let write_context = RenderGraphExecuteContext {};
    let command_buffers = executor.execute_graph(
        &resource_manager.create_dyn_command_writer_allocator(),
        &write_context,
    )?;

    // Present using the command buffers created via the graph
    frame_in_flight.present(&command_buffers)?;
    Ok(())
}
