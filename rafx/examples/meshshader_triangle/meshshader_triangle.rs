use log::LevelFilter;

use rafx::api::*;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 600;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Debug)
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
    let mut api = unsafe {
        RafxApi::new(
            &sdl2_systems.window,
            &sdl2_systems.window,
            &Default::default(),
        )?
    };

    // Wrap all of this so that it gets dropped before we drop the API object. This ensures a nice
    // clean shutdown.
    {
        // A cloneable device handle, these are lightweight and can be passed across threads
        let device_context = api.device_context();

        //
        // Allocate a graphics queue. By default, there is just one graphics queue and it is shared.
        // There currently is no API for customizing this but the code would be easy to adapt to act
        // differently. Most recommendations I've seen are to just use one graphics queue. (The
        // rendering hardware is shared among them)
        //
        let graphics_queue = device_context.create_queue(RafxQueueType::Graphics)?;

        //
        // Create a swapchain
        //
        let (window_width, window_height) = sdl2_systems.window.drawable_size();
        let swapchain = device_context.create_swapchain(
            &sdl2_systems.window,
            &sdl2_systems.window,
            &graphics_queue,
            &RafxSwapchainDef {
                width: window_width,
                height: window_height,
                enable_vsync: true,
                color_space_priority: vec![RafxSwapchainColorSpace::Srgb],
            },
        )?;

        //
        // Wrap the swapchain in this helper to cut down on boilerplate. This helper is
        // multithreaded-rendering friendly! The PresentableFrame it returns can be sent to another
        // thread and presented from there, and any errors are returned back to the main thread
        // when the next image is acquired. The helper also ensures that the swapchain is rebuilt
        // as necessary.
        //
        let mut swapchain_helper = RafxSwapchainHelper::new(&device_context, swapchain, None)?;

        //
        // Some default data we can render
        //
        #[rustfmt::skip]
        let vertex_data = [
            0.0f32, 0.5, 1.0, 0.0, 0.0,
            -0.5, -0.5, 0.0, 1.0, 0.0,
            0.5, 0.5, 0.0, 0.0, 1.0,
        ];

        let uniform_data = [1.0f32, 0.0, 1.0, 1.0];

        //
        // Create command pools/command buffers. The command pools need to be immutable while they are
        // being processed by a queue, so create one per swapchain image.
        //
        // Create vertex buffers (with position/color information) and a uniform buffers that we
        // can bind to pass additional info.
        //
        // In this demo, the color data in the shader is pulled from
        // the uniform instead of the vertex buffer. Buffers also need to be immutable while
        // processed, so we need one per swapchain image
        //
        let mut command_pools = Vec::with_capacity(swapchain_helper.rotating_frame_count());
        let mut command_buffers = Vec::with_capacity(swapchain_helper.rotating_frame_count());
        let mut vertex_buffers = Vec::with_capacity(swapchain_helper.rotating_frame_count());
        let mut uniform_buffers = Vec::with_capacity(swapchain_helper.rotating_frame_count());

        for _ in 0..swapchain_helper.rotating_frame_count() {
            let mut command_pool =
                graphics_queue.create_command_pool(&RafxCommandPoolDef { transient: true })?;

            let command_buffer = command_pool.create_command_buffer(&RafxCommandBufferDef {
                is_secondary: false,
            })?;

            let vertex_buffer = device_context
                .create_buffer(&RafxBufferDef::for_staging_vertex_buffer_data(&vertex_data))?;
            vertex_buffer.copy_to_host_visible_buffer(&vertex_data)?;

            let uniform_buffer = device_context.create_buffer(
                &RafxBufferDef::for_staging_uniform_buffer_data(&uniform_data),
            )?;
            uniform_buffer.copy_to_host_visible_buffer(&uniform_data)?;

            command_pools.push(command_pool);
            command_buffers.push(command_buffer);
            vertex_buffers.push(vertex_buffer);
            uniform_buffers.push(uniform_buffer);
        }

        //
        // Load a shader from source - this part is API-specific. vulkan will want SPV, metal wants
        // source code or even better a pre-compiled library. But the metal compiler toolchain only
        // works on mac/windows and is a command line tool without programmatic access.
        //
        // In an engine, it would be better to pack different formats depending on the platform
        // being built. Higher level rafx crates can help with this. But this is meant as a simple
        // example without needing those crates.
        //
        // RafxShaderPackage holds all the data needed to create a GPU shader module object. It is
        // heavy-weight, fully owning the data. We create by loading files from disk. This object
        // can be stored as an opaque, binary object and loaded directly if you prefer.
        //
        // RafxShaderModuleDef is a lightweight reference to this data. Here we create it from the
        // RafxShaderPackage, but you can create it yourself if you already loaded the data in some
        // other way.
        //
        // The resulting shader modules represent a loaded shader GPU object that is used to create
        // shaders. Shader modules can be discarded once the graphics pipeline is built.
        //
        let shaders_base_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples/meshshader_triangle/shaders");

        let mut mesh_shader_package = RafxShaderPackage::default();
        let hlsl_shader_string =
            std::fs::read_to_string(shaders_base_path.join("shaders.hlsl")).unwrap();
        let msl_shader_string =
            std::fs::read_to_string(shaders_base_path.join("shaders.metal")).unwrap();
        mesh_shader_package.dx12 = Some(RafxShaderPackageDx12::Src(hlsl_shader_string));
        mesh_shader_package.metal = Some(RafxShaderPackageMetal::Src(msl_shader_string));

        let mesh_shader_module =
            device_context.create_shader_module(mesh_shader_package.module_def())?;
        let frag_shader_module =
            device_context.create_shader_module(mesh_shader_package.module_def())?;

        //
        // Create the shader object by combining the stages
        //
        // Hardcode the reflecton data required to interact with the shaders. This can be generated
        // offline and loaded with the shader but this is not currently provided in rafx-api itself.
        // (But see the shader pipeline in higher-level rafx crates for example usage, generated
        // from spirv_cross)
        //

        let mesh_shader_stage_def = RafxShaderStageDef {
            shader_module: mesh_shader_module,
            reflection: RafxShaderStageReflection {
                entry_point_name: "main_ms".to_string(),
                shader_stage: RafxShaderStageFlags::MESH,
                compute_threads_per_group: Some([128, 1, 1]),
                resources: vec![],
            },
        };

        let frag_shader_stage_def = RafxShaderStageDef {
            shader_module: frag_shader_module,
            reflection: RafxShaderStageReflection {
                entry_point_name: "main_ps".to_string(),
                shader_stage: RafxShaderStageFlags::FRAGMENT,
                compute_threads_per_group: None,
                resources: vec![],
            },
        };

        //
        // Combine the shader stages into a single shader
        //
        let shader =
            device_context.create_shader(vec![mesh_shader_stage_def, frag_shader_stage_def])?;

        //
        // Create the root signature object - it represents the pipeline layout and can be shared among
        // shaders. But one per shader is fine.
        //
        let root_signature = device_context.create_root_signature(&RafxRootSignatureDef {
            shaders: &[shader.clone()],
            immutable_samplers: &[],
        })?;

        let vertex_layout = RafxVertexLayout::default();

        let pipeline = device_context.create_graphics_pipeline(&RafxGraphicsPipelineDef {
            shader: &shader,
            root_signature: &root_signature,
            vertex_layout: &vertex_layout,
            blend_state: &Default::default(),
            depth_state: &Default::default(),
            rasterizer_state: &Default::default(),
            color_formats: &[swapchain_helper.format()],
            sample_count: RafxSampleCount::SampleCount1,
            depth_stencil_format: None,
            primitive_topology: RafxPrimitiveTopology::TriangleList,
            debug_name: None,
        })?;

        let start_time = std::time::Instant::now();

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

            let elapsed_seconds = start_time.elapsed().as_secs_f32();

            #[rustfmt::skip]
            let vertex_data = [
                0.0f32, 0.5, 1.0, 0.0, 0.0,
                0.5 - (elapsed_seconds.cos() / 2. + 0.5), -0.5, 0.0, 1.0, 0.0,
                -0.5 + (elapsed_seconds.cos() / 2. + 0.5), -0.5, 0.0, 0.0, 1.0,
            ];

            let color = (elapsed_seconds.cos() + 1.0) / 2.0;
            let uniform_data = [color, 0.0, 1.0 - color, 1.0];

            //
            // Acquire swapchain image
            //
            let (window_width, window_height) = sdl2_systems.window.vulkan_drawable_size();
            let presentable_frame =
                swapchain_helper.acquire_next_image(window_width, window_height, None)?;
            let swapchain_texture = presentable_frame.swapchain_texture();

            //
            // Use the command pool/buffer assigned to this frame
            //
            let cmd_pool = &mut command_pools[presentable_frame.rotating_frame_index()];
            let cmd_buffer = &command_buffers[presentable_frame.rotating_frame_index()];
            let vertex_buffer = &vertex_buffers[presentable_frame.rotating_frame_index()];
            let uniform_buffer = &uniform_buffers[presentable_frame.rotating_frame_index()];

            //
            // Update the buffers
            //
            vertex_buffer.copy_to_host_visible_buffer(&vertex_data)?;
            uniform_buffer.copy_to_host_visible_buffer(&uniform_data)?;

            //
            // Record the command buffer. For now just transition it between layouts
            //
            cmd_pool.reset_command_pool()?;

            cmd_buffer.begin()?;
            // Put it into a layout where we can draw on it
            cmd_buffer.cmd_resource_barrier(
                &[],
                &[RafxTextureBarrier::state_transition(
                    &swapchain_texture,
                    RafxResourceState::PRESENT,
                    RafxResourceState::RENDER_TARGET,
                )],
            )?;

            cmd_buffer.cmd_begin_render_pass(
                &[RafxColorRenderTargetBinding {
                    texture: &swapchain_texture,
                    load_op: RafxLoadOp::Clear,
                    store_op: RafxStoreOp::Store,
                    array_slice: None,
                    mip_slice: None,
                    clear_value: RafxColorClearValue([0.2, 0.2, 0.2, 1.0]),
                    resolve_target: None,
                    resolve_store_op: RafxStoreOp::DontCare,
                    resolve_mip_slice: None,
                    resolve_array_slice: None,
                }],
                None,
            )?;

            cmd_buffer.cmd_bind_pipeline(&pipeline)?;

            // use windows::core::Interface;
            // use windows::Win32::Graphics::Direct3D12 as d3d12;
            // let command_list = &cmd_buffer.dx12_command_buffer().unwrap().dx12_graphics_command_list();
            // let command_list6 = command_list.cast::<d3d12::ID3D12GraphicsCommandList6>().unwrap();
            //unsafe
            //{
            //command_list6.DispatchMesh(1, 1, 1);
            cmd_buffer.cmd_draw_mesh(1, 1, 1)?;
            //}

            cmd_buffer.cmd_end_render_pass()?;

            // Put it into a layout where we can present it
            cmd_buffer.cmd_resource_barrier(
                &[],
                &[RafxTextureBarrier::state_transition(
                    &swapchain_texture,
                    RafxResourceState::RENDER_TARGET,
                    RafxResourceState::PRESENT,
                )],
            )?;
            cmd_buffer.end()?;

            //
            // Present the image
            //
            let result = presentable_frame.present(&graphics_queue, &[&cmd_buffer]);
            result.unwrap();
        }

        // Wait for all GPU work to complete before destroying resources it is using
        graphics_queue.wait_for_queue_idle()?;
    }

    // Optional, but calling this verifies that all rafx objects/device contexts have been
    // destroyed and where they were created. Good for finding unintended leaks!
    api.destroy()?;

    Ok(())
}

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
    let mut window_binding = video_subsystem.window("Rafx Example", WINDOW_WIDTH, WINDOW_HEIGHT);

    let window_builder = window_binding
        .position_centered()
        .allow_highdpi()
        .resizable();

    #[cfg(target_os = "macos")]
    let window_builder = window_builder.metal_view();

    let window = window_builder.build().expect("Failed to create window");

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
