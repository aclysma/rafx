use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk;
use log::LevelFilter;
use renderer::resources::vk_description as dsc;
use renderer::resources::vk_description::{FramebufferMeta, SwapchainSurfaceInfo};
use renderer::resources::{FramebufferResource, RenderPassResource, ResourceArc, ResourceManager};
use renderer_shell_vulkan::{
    MsaaLevel, VkContextBuilder, VkDeviceContext, VkImageRaw, VkSurface, VkSwapchain, Window,
};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::EventPump;
use std::sync::Arc;

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
        .filter_level(LevelFilter::Info)
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

struct SwapchainResources {
    swapchain_surface_info: SwapchainSurfaceInfo,
    renderpass: ResourceArc<RenderPassResource>,
    framebuffers: Vec<ResourceArc<FramebufferResource>>,
}

impl SwapchainResources {
    fn new(
        swapchain: &VkSwapchain,
        resource_manager: &mut ResourceManager,
    ) -> VkResult<Self> {
        let swapchain_surface_info = SwapchainSurfaceInfo {
            color_format: swapchain.swapchain_info.color_format,
            depth_format: swapchain.swapchain_info.depth_format,
            extents: swapchain.swapchain_info.extents,
            msaa_level: swapchain.swapchain_info.msaa_level,
            surface_format: swapchain.swapchain_info.surface_format,
        };

        let renderpass_dsc = Arc::new(dsc::RenderPass {
            attachments: vec![dsc::AttachmentDescription {
                flags: dsc::AttachmentDescriptionFlags::None,
                format: dsc::AttachmentFormat::MatchSurface,
                samples: dsc::SampleCountFlags::MatchSwapchain,
                load_op: dsc::AttachmentLoadOp::Clear,
                store_op: dsc::AttachmentStoreOp::Store,
                stencil_load_op: dsc::AttachmentLoadOp::DontCare,
                stencil_store_op: dsc::AttachmentStoreOp::DontCare,
                initial_layout: dsc::ImageLayout::Undefined,
                final_layout: dsc::ImageLayout::PresentSrcKhr,
            }],
            subpasses: vec![dsc::SubpassDescription {
                color_attachments: vec![dsc::AttachmentReference {
                    attachment: dsc::AttachmentIndex::Index(0),
                    layout: dsc::ImageLayout::ColorAttachmentOptimal,
                }],
                input_attachments: vec![],
                resolve_attachments: vec![],
                depth_stencil_attachment: None,
                pipeline_bind_point: dsc::PipelineBindPoint::Graphics,
            }],
            dependencies: vec![dsc::SubpassDependency {
                src_subpass: dsc::SubpassDependencyIndex::External,
                dst_subpass: dsc::SubpassDependencyIndex::Index(0),
                src_stage_mask: dsc::PipelineStageFlags::TOP_OF_PIPE,
                dst_stage_mask: dsc::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: vec![],
                dst_access_mask: vec![
                    dsc::AccessFlags::ColorAttachmentRead,
                    dsc::AccessFlags::ColorAttachmentWrite,
                ],
                dependency_flags: dsc::DependencyFlags::ByRegion,
            }],
        });

        let renderpass = resource_manager
            .resources()
            .get_or_create_renderpass(renderpass_dsc, &swapchain_surface_info)?;

        let mut framebuffers = Vec::with_capacity(swapchain.swapchain_images.len());
        for &image in &swapchain.swapchain_images {
            let image = resource_manager.resources().insert_raw_image(VkImageRaw {
                image,
                allocation: None,
            });

            let image_view = resource_manager.resources().get_or_create_image_view(
                &image,
                &dsc::ImageViewMeta::default_2d_no_mips_or_layers(
                    swapchain_surface_info.surface_format.format.into(),
                    dsc::ImageAspectFlag::Color.into(),
                ),
            )?;

            framebuffers.push(resource_manager.resources().get_or_create_framebuffer(
                renderpass.clone(),
                &[image_view],
                &FramebufferMeta {
                    width: swapchain_surface_info.extents.width,
                    height: swapchain_surface_info.extents.height,
                    layers: 1,
                },
            )?);
        }

        Ok(SwapchainResources {
            swapchain_surface_info,
            renderpass,
            framebuffers,
        })
    }
}

fn run(
    window: &dyn Window,
    event_pump: &mut EventPump,
) -> VkResult<()> {
    let context = VkContextBuilder::new()
        .use_vulkan_debug_layer(true)
        .msaa_level_priority(vec![MsaaLevel::Sample1])
        .prefer_mailbox_present_mode();

    let render_registry = renderer::nodes::RenderRegistryBuilder::default()
        //.register_render_phase::<OpaqueRenderPhase>("Opaque")
        .build();

    let vk_context = context.build(window).unwrap();
    let device_context = vk_context.device_context().clone();
    let mut resource_manager = ResourceManager::new(&device_context, &render_registry);

    let mut surface = VkSurface::new(&vk_context, window, None).unwrap();

    let mut swapchain_resources =
        SwapchainResources::new(surface.swapchain(), &mut resource_manager)?;

    loop {
        //
        // Process input
        //
        if !process_input(&device_context, &resource_manager, event_pump) {
            break;
        }

        //
        // Redraw
        //
        {
            let frame_in_flight_result = surface.acquire_next_swapchain_image(window);
            match frame_in_flight_result {
                Ok(frame_in_flight) => {
                    let clear_values = [vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    }];

                    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                        .render_pass(swapchain_resources.renderpass.get_raw().renderpass)
                        .framebuffer(
                            swapchain_resources.framebuffers
                                [frame_in_flight.present_index() as usize]
                                .get_raw()
                                .framebuffer,
                        )
                        .render_area(vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: swapchain_resources.swapchain_surface_info.extents,
                        })
                        .clear_values(&clear_values);

                    let mut writer = resource_manager
                        .dyn_command_writer_allocator()
                        .allocate_writer(
                            device_context
                                .queue_family_indices()
                                .graphics_queue_family_index,
                            vk::CommandPoolCreateFlags::TRANSIENT,
                            0,
                        )?;

                    let command_buffer = writer.begin_command_buffer(
                        vk::CommandBufferLevel::PRIMARY,
                        vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                        None,
                    )?;

                    //TODO: Draw a triangle

                    unsafe {
                        let device = device_context.device();

                        device.cmd_begin_render_pass(
                            command_buffer,
                            &render_pass_begin_info,
                            vk::SubpassContents::INLINE,
                        );

                        device.cmd_end_render_pass(command_buffer);
                    }

                    writer.end_command_buffer()?;

                    frame_in_flight.present(&[command_buffer])?;
                    resource_manager.on_frame_complete()?;
                    Ok(())
                }
                Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    surface.rebuild_swapchain(window, None)?;
                    swapchain_resources =
                        SwapchainResources::new(surface.swapchain(), &mut resource_manager)?;
                    Ok(())
                }
                Err(ash::vk::Result::SUCCESS) => Ok(()),
                Err(ash::vk::Result::SUBOPTIMAL_KHR) => Ok(()),
                Err(e) => {
                    log::warn!("Unexpected rendering error");
                    return Err(e);
                }
            }?;
        }
    }

    surface.tear_down(None);
    Ok(())
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
