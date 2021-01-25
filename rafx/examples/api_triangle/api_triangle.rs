use log::LevelFilter;

use rafx::api::raw_window_handle::HasRawWindowHandle;
use rafx::api::*;
use std::path::Path;

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
    // Init SDL2
    //
    let sdl2_systems = sdl2_init();

    //
    // Create the api
    //
    let mut api = create_api(&sdl2_systems.window)?;

    // Wrap all of this so that it gets dropped
    {
        let device_context = api.device_context();

        //
        // Create a swapchain
        //
        let (window_width, window_height) = sdl2_systems.window.drawable_size();
        let swapchain = device_context.create_swapchain(
            &sdl2_systems.window,
            &RafxSwapchainDef {
                width: window_width,
                height: window_height,
                enable_vsync: true,
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
        // Allocate a graphics queue. By default, there is just one graphics queue and it is shared.
        // There currently is no API for customizing this but the code would be easy to adapt to act
        // differently. Most recommendations I've seen are to just use one graphics queue. (The
        // rendering hardware is shared among them)
        //
        let graphics_queue = device_context.create_queue(RafxQueueType::Graphics)?;

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
        let mut command_pools = Vec::with_capacity(swapchain_helper.image_count());
        let mut command_buffers = Vec::with_capacity(swapchain_helper.image_count());
        let mut vertex_buffers = Vec::with_capacity(swapchain_helper.image_count());
        let mut uniform_buffers = Vec::with_capacity(swapchain_helper.image_count());

        for _ in 0..swapchain_helper.image_count() {
            let mut command_pool =
                graphics_queue.create_command_pool(&RafxCommandPoolDef { transient: true })?;

            let command_buffer = command_pool.create_command_buffer(&RafxCommandBufferDef {
                is_secondary: false,
            })?;

            let vertex_buffer = device_context
                .create_buffer(&RafxBufferDef::for_staging_vertex_buffer_data(&vertex_data))?;
            vertex_buffer.copy_to_host_visible_buffer(&vertex_data)?;

            let uniform_buffer = device_context
                .create_buffer(&RafxBufferDef::for_staging_uniform_data(&uniform_data))?;
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
        let processed_shaders_base_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples/api_triangle/processed_shaders");

        let vert_shader_package = load_shader_packages(
            &processed_shaders_base_path,
            "shader.vert.metal",
            "shader.vert.spv",
        )?;

        let frag_shader_package = load_shader_packages(
            &processed_shaders_base_path,
            "shader.frag.metal",
            "shader.frag.spv",
        )?;

        let vert_shader_module =
            device_context.create_shader_module(vert_shader_package.module_def())?;
        let frag_shader_module =
            device_context.create_shader_module(frag_shader_package.module_def())?;

        //
        // Create the shader object by combining the stages
        //
        // Hardcode the reflecton data required to interact with the shaders. This can be generated
        // offline and loaded with the shader but this is not currently provided in rafx-api itself.
        // (But see the shader pipeline in higher-level rafx crates for example usage, generated
        // from spirv_cross)
        //
        let color_shader_resource = RafxShaderResource {
            name: Some("color".to_string()),
            set_index: 0,
            binding: 0,
            resource_type: RafxResourceType::UNIFORM_BUFFER,
            ..Default::default()
        };

        let vert_shader_stage_def = RafxShaderStageDef {
            shader_module: vert_shader_module,
            reflection: RafxShaderStageReflection {
                entry_point_name: "main".to_string(),
                shader_stage: RafxShaderStageFlags::VERTEX,
                compute_threads_per_group: None,
                resources: vec![color_shader_resource.clone()],
            },
        };

        let frag_shader_stage_def = RafxShaderStageDef {
            shader_module: frag_shader_module,
            reflection: RafxShaderStageReflection {
                entry_point_name: "main".to_string(),
                shader_stage: RafxShaderStageFlags::FRAGMENT,
                compute_threads_per_group: None,
                resources: vec![color_shader_resource],
            },
        };

        //
        // Combine the shader stages into a single shader
        //
        let shader =
            device_context.create_shader(vec![vert_shader_stage_def, frag_shader_stage_def])?;

        //
        // Create the root signature object - it represents the pipeline layout and can be shared among
        // shaders. But one per shader is fine.
        //
        let root_signature = device_context.create_root_signature(&RafxRootSignatureDef {
            shaders: &[shader.clone()],
            immutable_samplers: &[],
        })?;

        //
        // Descriptors are allocated in blocks and never freed. Normally you will want to build a
        // pooling system around this. (Higher-level rafx crates provide this.) But they're small
        // and cheap. We need one per swapchain image.
        //
        let mut descriptor_set_array =
            device_context.create_descriptor_set_array(&RafxDescriptorSetArrayDef {
                set_index: 0,
                root_signature: &root_signature,
                array_length: 3, // One per swapchain image.
            })?;

        // Initialize them all at once here.. this can be done per-frame as well.
        for i in 0..swapchain_helper.image_count() {
            descriptor_set_array.update_descriptor_set(&[RafxDescriptorUpdate {
                array_index: i as u32,
                descriptor_key: RafxDescriptorKey::Name("color"),
                elements: RafxDescriptorElements {
                    buffers: Some(&[&uniform_buffers[i]]),
                    ..Default::default()
                },
                ..Default::default()
            }])?;
        }

        //
        // Now set up the pipeline. LOTS of things can be configured here, but aside from the vertex
        // layout most of it can be left as default.
        //
        let vertex_layout = RafxVertexLayout {
            attributes: vec![
                RafxVertexLayoutAttribute {
                    format: RafxFormat::R32G32_SFLOAT,
                    buffer_index: 0,
                    location: 0,
                    offset: 0,
                },
                RafxVertexLayoutAttribute {
                    format: RafxFormat::R32G32B32_SFLOAT,
                    buffer_index: 0,
                    location: 1,
                    offset: 8,
                },
            ],
            buffers: vec![RafxVertexLayoutBuffer {
                stride: 20,
                rate: RafxVertexAttributeRate::Vertex,
            }],
        };

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

            let current_time = std::time::Instant::now();
            let seconds = (current_time - start_time).as_secs_f32();

            #[rustfmt::skip]
            let vertex_data = [
                0.0f32, 0.5, 1.0, 0.0, 0.0,
                0.5 - (seconds.cos() / 2. + 0.5), -0.5, 0.0, 1.0, 0.0,
                -0.5 + (seconds.cos() / 2. + 0.5), -0.5, 0.0, 0.0, 1.0,
            ];

            let color = (seconds.cos() + 1.0) / 2.0;
            let uniform_data = [color, 0.0, 1.0 - color, 1.0];

            //
            // Acquire swapchain image
            //
            let (window_width, window_height) = sdl2_systems.window.vulkan_drawable_size();
            let presentable_frame =
                swapchain_helper.acquire_next_image(window_width, window_height, None)?;
            let render_target = presentable_frame.render_target();

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
                &[],
                &[RafxRenderTargetBarrier::state_transition(
                    &render_target,
                    RafxResourceState::PRESENT,
                    RafxResourceState::RENDER_TARGET,
                )],
            )?;

            cmd_buffer.cmd_bind_render_targets(
                &[RafxColorRenderTargetBinding {
                    render_target: &render_target,
                    load_op: RafxLoadOp::Clear,
                    store_op: RafxStoreOp::Store,
                    array_slice: None,
                    mip_slice: None,
                    clear_value: RafxColorClearValue([0.0, 0.0, 0.0, 0.0]),
                    resolve_target: None,
                    resolve_store_op: RafxStoreOp::DontCare,
                    resolve_mip_slice: None,
                    resolve_array_slice: None,
                }],
                None,
            )?;

            cmd_buffer.cmd_bind_pipeline(&pipeline)?;

            cmd_buffer.cmd_bind_vertex_buffers(
                0,
                &[RafxVertexBufferBinding {
                    buffer: &vertex_buffer,
                    offset: 0,
                }],
            )?;
            cmd_buffer.cmd_bind_descriptor_set(
                &descriptor_set_array,
                presentable_frame.rotating_frame_index() as u32,
            )?;
            cmd_buffer.cmd_draw(3, 0)?;

            // Put it into a layout where we can present it

            cmd_buffer.cmd_unbind_render_targets()?;

            cmd_buffer.cmd_resource_barrier(
                &[],
                &[],
                &[RafxRenderTargetBarrier::state_transition(
                    &render_target,
                    RafxResourceState::RENDER_TARGET,
                    RafxResourceState::PRESENT,
                )],
            )?;
            cmd_buffer.end()?;

            //
            // Present the image
            //
            presentable_frame.present(&graphics_queue, &[&cmd_buffer])?;
        }

        // Wait until all the submitted work gets flushed before continuing
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

#[cfg(feature = "rafx-metal")]
fn create_metal_api(window: &dyn HasRawWindowHandle) -> RafxResult<RafxApi> {
    RafxApi::new_metal(
        window,
        &RafxApiDef {
            validation_mode: RafxValidationMode::EnabledIfAvailable,
        },
        &Default::default(),
    )
}

#[cfg(feature = "rafx-vulkan")]
fn create_vulkan_api(window: &dyn HasRawWindowHandle) -> RafxResult<RafxApi> {
    RafxApi::new_vulkan(
        window,
        &RafxApiDef {
            validation_mode: RafxValidationMode::EnabledIfAvailable,
        },
        &Default::default(),
    )
}

#[allow(unreachable_code)]
fn create_api(_window: &dyn HasRawWindowHandle) -> RafxResult<RafxApi> {
    #[cfg(feature = "rafx-metal")]
    {
        return create_metal_api(_window);
    }

    #[cfg(feature = "rafx-vulkan")]
    {
        return create_vulkan_api(_window);
    }

    Err("Rafx was compiled with no backend enabled. Add feature rafx-vulkan, rafx-metal, etc. to enable at least one backend")?
}

// Shader packages are serializable. The shader processor tool uses spirv_cross to compile the
// shaders for multiple platforms and package them in an easy to use opaque binary form. For this
// example, we'll just hard-code constructing this package.
fn load_shader_packages(
    _base_path: &Path,
    _metal_src_file: &str,
    _vk_spv_file: &str,
) -> RafxResult<RafxShaderPackage> {
    let mut _package = RafxShaderPackage::default();

    #[cfg(feature = "rafx-metal")]
    {
        let metal_path = _base_path.join(_metal_src_file);
        let metal_src = std::fs::read_to_string(metal_path)?;
        _package.metal = Some(RafxShaderPackageMetal::Src(metal_src));
    }

    #[cfg(feature = "rafx-vulkan")]
    {
        let vk_path = _base_path.join(_vk_spv_file);
        let vk_bytes = std::fs::read(vk_path)?;
        _package.vk = Some(RafxShaderPackageVulkan::SpvBytes(vk_bytes));
    }

    Ok(_package)
}
