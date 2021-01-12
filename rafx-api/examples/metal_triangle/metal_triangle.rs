use log::LevelFilter;

use rafx_api::metal::{RenderpassColorAttachmentDef, RenderpassDef};
use rafx_api::{
    RafxBuffer, RafxCommandBuffer, RafxCommandBufferDef, RafxCommandPoolDef, RafxDeviceDef,
    RafxGraphicsPipeline, RafxPresentableFrame, RafxQueue, RafxQueueType, RafxRenderpass,
    RafxShaderModule, RafxValidationMode,
};

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

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Trace)
        .init();

    let sdl2_systems = sdl2_init();

    //
    // Create the device and surface
    //
    let device = RafxDevice::new_metal(
        &Default::default(),
        &RafxDeviceDef {
            validation_mode: RafxValidationMode::EnabledIfAvailable,
        },
        &sdl2_systems.window,
    )
    .unwrap();

    const FRAME_COUNT: usize = 3;
    let mut command_pools = Vec::with_capacity(3);
    let mut command_buffers = Vec::with_capacity(3);

    let graphics_queue = device.create_queue(RafxQueueType::Graphics).unwrap();
    for _ in 0..FRAME_COUNT {
        let mut command_pool = device
            .create_command_pool(&graphics_queue, &RafxCommandPoolDef { transient: true })
            .unwrap();

        let command_buffer = device
            .create_command_buffer(
                &mut command_pool,
                &RafxCommandBufferDef {
                    is_secondary: false,
                },
            )
            .unwrap();
        command_pools.push(command_pool);
        command_buffers.push(command_buffer);
    }

    let (window_width, window_height) = sdl2_systems.window.drawable_size();
    let mut surface = device
        .create_surface(&sdl2_systems.window, window_width, window_height)
        .unwrap();

    //
    // Load a shader from source - this part is API-specific. vulkan will want SPV, metal wants
    // source code or even better a pre-compiled library. But their compile toolchain only works on
    // mac/windows and is a command line tool without programmatic access. So the "example" way to
    // do this is just load source code. The engine way would be to pack different formats depending
    // on the platform being built
    //
    // let library_source_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    //     .join("examples/metal_triangle/shaders.metal");
    // let library_source = std::fs::read_to_string(library_source_path).unwrap();
    // let shader_module = RafxShaderModule::Metal(device.metal_device().unwrap().create_shader_module_from_source(
    //     &library_source,
    //     &metal::CompileOptions::new()
    // ).unwrap());

    let library_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/metal_triangle/shaders.metallib");
    let shader_module = RafxShaderModule::Metal(
        device
            .metal_device()
            .unwrap()
            .create_shader_module_from_library_file(&library_path)
            .unwrap(),
    );

    //
    // Create a graphics pipeline
    //
    let pipeline_state_def = create_render_pipeline_descriptor_def();
    let graphics_pipeline = device
        .create_graphics_pipeline(&shader_module, &pipeline_state_def)
        .unwrap();

    //
    // Expose internals to work with an API directly
    //
    let renderpass = device.create_renderpass(create_renderpass_def()).unwrap();

    let vertex_buffer = {
        #[rustfmt::skip]
        let vertex_data = [
            0.0f32, 0.5, 1.0, 0.0, 0.0,
            -0.5, -0.5, 0.0, 1.0, 0.0,
            0.5, 0.5, 0.0, 0.0, 1.0,
        ];

        // MTLResourceOptions::CPUCacheModeDefaultCache | MTLResourceOptions::StorageModeManaged,
        device.create_buffer_with_data(&vertex_data).unwrap()
    };

    let mut frame_index = 0;
    let mut r = 0.0f32;

    log::info!("Starting window event loop");
    let mut event_pump = sdl2_systems
        .context
        .event_pump()
        .expect("Could not create sdl event pump");

    'running: loop {
        if !process_input(&mut event_pump) {
            break 'running;
        }

        objc::rc::autoreleasepool(|| {
            command_pools[frame_index].reset_command_pool();

            #[rustfmt::skip]
            let vertex_data = [
                0.0f32, 0.5, 1.0, 0.0, 0.0,
                -0.5 + (r.cos() / 2. + 0.5), -0.5, 0.0, 1.0, 0.0,
                0.5 - (r.cos() / 2. + 0.5), -0.5, 0.0, 0.0, 1.0,
            ];

            vertex_buffer.copy_to_buffer(&vertex_data);

            let (window_width, window_height) = sdl2_systems.window.vulkan_drawable_size();
            let presentable_frame = match surface.begin_frame(window_width, window_height) {
                Ok(drawable) => drawable,
                Err(e) => panic!(e),
            };

            draw_window(
                &device,
                &renderpass,
                presentable_frame,
                //&command_queue,
                &graphics_pipeline,
                &vertex_buffer,
                &command_buffers[frame_index],
                &graphics_queue,
            );

            r += 0.01f32;
            frame_index = (frame_index + 1) % (FRAME_COUNT - 1);
        });
    }
}

fn draw_window(
    device: &RafxDevice,
    renderpass: &RafxRenderpass,
    presentable_frame: RafxPresentableFrame,
    pipeline: &RafxGraphicsPipeline,
    vertex_buffer: &RafxBuffer,
    command_buffer: &RafxCommandBuffer,
    queue: &RafxQueue,
) {
    let encoder = command_buffer
        .begin_renderpass(renderpass, &[&presentable_frame.texture()])
        .unwrap();
    let metal_encoder = encoder.metal_render_command_encoder().unwrap();

    metal_encoder.encoder().set_render_pipeline_state(
        pipeline
            .metal_graphics_pipeline()
            .unwrap()
            .render_pipeline_state(),
    );
    metal_encoder.encoder().set_vertex_buffer(
        0,
        Some(vertex_buffer.metal_buffer().unwrap().buffer()),
        0,
    );
    metal_encoder
        .encoder()
        .draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 3);
    metal_encoder.encoder().end_encoding();

    // This is the way it shows in examples but it's a convenience function for the below behavior
    //TODO: On complete callback
    //command_buffer.command_buffer().present_drawable(&drawable);
    //command_buffer.metal_command_buffer().unwrap().command_buffer().commit();
    //command_buffer.metal_command_buffer().unwrap().command_buffer().wait_until_completed();
    //presentable_frame.present();

    queue.submit()
}

fn create_render_pipeline_descriptor_def() -> rafx_api::metal::RenderPipelineDescriptorDef {
    rafx_api::metal::RenderPipelineDescriptorDef {
        vertex_shader: Some("triangle_vertex".to_string()),
        fragment_shader: Some("triangle_fragment".to_string()),
        color_attachments: vec![
            rafx_api::metal::RenderPipelineColorAttachmentDescriptorDef {
                pixel_format: metal::MTLPixelFormat::BGRA8Unorm,
                blending_enabled: true,
                rgb_blend_operation: metal::MTLBlendOperation::Add,
                alpha_blend_operation: metal::MTLBlendOperation::Add,
                source_rgb_blend_factor: metal::MTLBlendFactor::SourceAlpha,
                source_alpha_blend_factor: metal::MTLBlendFactor::SourceAlpha,
                destination_rgb_blend_factor: metal::MTLBlendFactor::OneMinusSourceAlpha,
                destination_alpha_blend_factor: metal::MTLBlendFactor::OneMinusSourceAlpha,
            },
        ],
    }
}

fn create_renderpass_def() -> rafx_api::metal::RenderpassDef {
    RenderpassDef {
        color_attachments: vec![RenderpassColorAttachmentDef {
            attachment_index: 0,
            load_action: metal::MTLLoadAction::Clear,
            store_action: metal::MTLStoreAction::Store,
            clear_color: metal::MTLClearColor::new(0.0, 0.0, 0.0, 1.0),
        }],
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
