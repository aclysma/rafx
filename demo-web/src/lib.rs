
#[cfg(not(target_arch = "wasm32"))]
mod main_native;
#[cfg(not(target_arch = "wasm32"))]
pub use main_native::*;

#[cfg(target_arch = "wasm32")]
mod main_web;
#[cfg(target_arch = "wasm32")]
pub use main_web::*;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use rafx::api::{RafxApi, RafxSwapchainDef, RafxSwapchainHelper, RafxQueueType, RafxResult, RafxCommandPoolDef, RafxBufferDef, RafxCommandBufferDef, RafxShaderModuleDef, RafxShaderModuleDefGl, RafxShaderResource, RafxGlUniformMember, RafxShaderStageDef, RafxShaderStageReflection, RafxShaderStageFlags, RafxResourceType, RafxRootSignatureDef, RafxDescriptorSetArrayDef, RafxDescriptorUpdate, RafxDescriptorKey, RafxDescriptorElements, RafxVertexLayout, RafxVertexLayoutAttribute, RafxFormat, RafxVertexLayoutBuffer, RafxVertexAttributeRate, RafxGraphicsPipelineDef, RafxSampleCount, RafxPrimitiveTopology, RafxTextureBarrier, RafxResourceState, RafxColorRenderTargetBinding, RafxStoreOp, RafxColorClearValue, RafxLoadOp, RafxVertexBufferBinding};
use rafx::api::gl::NONE_RENDERBUFFER;

pub fn update_loop(
    window: winit::window::Window,
    event_loop: winit::event_loop::EventLoop<()>,
) -> RafxResult<()> {

    //
    // Create the api
    //
    log::trace!("Creating the API");
    let mut api = RafxApi::new(&window, &Default::default())?;

    // Wrap all of this so that it gets dropped before we drop the API object. This ensures a nice
    // clean shutdown.
    {
        // A cloneable device handle, these are lightweight and can be passed across threads
        let device_context = api.device_context();

        //
        // Create a swapchain
        //
        log::trace!("Creating swapchain");
        let window_size = window.inner_size();
        let swapchain = device_context.create_swapchain(
            &window,
            &RafxSwapchainDef {
                width: window_size.width,
                height: window_size.height,
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
        log::trace!("Creating swapchain helper");
        let mut swapchain_helper = RafxSwapchainHelper::new(&device_context, swapchain, None)?;

        //
        // Allocate a graphics queue. By default, there is just one graphics queue and it is shared.
        // There currently is no API for customizing this but the code would be easy to adapt to act
        // differently. Most recommendations I've seen are to just use one graphics queue. (The
        // rendering hardware is shared among them)
        //
        log::trace!("Creating graphics queue");
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
            log::trace!("Creating command pool");
            let mut command_pool =
                graphics_queue.create_command_pool(&RafxCommandPoolDef { transient: true })?;

            log::trace!("Creating command buffer");
            let command_buffer = command_pool.create_command_buffer(&RafxCommandBufferDef {
                is_secondary: false,
            })?;

            log::trace!("Creating vertex buffer");
            let vertex_buffer = device_context
                .create_buffer(&RafxBufferDef::for_staging_vertex_buffer_data(&vertex_data))?;
            log::trace!("Populating vertex buffer");
            vertex_buffer.copy_to_host_visible_buffer(&vertex_data)?;

            log::trace!("Creating uniform buffer");
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
        // source code or even better a pre-compiled library. The web demo is GL-only, and it only
        // supports loading from src.
        //
        // The resulting shader modules represent a loaded shader GPU object that is used to create
        // shaders. Shader modules can be discarded once the graphics pipeline is built.
        //
        log::trace!("Creating shader modules");
        let vert_shader_module = device_context.create_shader_module(RafxShaderModuleDef {
            gl: Some(RafxShaderModuleDefGl::GlSrc(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/shader.vert.gles"
            )))),
            ..Default::default()
        })?;

        let frag_shader_module = device_context.create_shader_module(RafxShaderModuleDef {
            gl: Some(RafxShaderModuleDefGl::GlSrc(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/shader.frag.gles"
            )))),
            ..Default::default()
        })?;

        //
        // Create the shader object by combining the stages
        //
        // Hardcode the reflecton data required to interact with the shaders. This can be generated
        // offline and loaded with the shader but this is not currently provided in rafx-api itself.
        // (But see the shader pipeline in higher-level rafx crates for example usage, generated
        // from spirv_cross)
        //
        log::trace!("Creating shader resources");
        let color_shader_resource = RafxShaderResource {
            name: Some("color".to_string()),
            set_index: 0,
            binding: 0,
            resource_type: RafxResourceType::UNIFORM_BUFFER,
            gl_name: Some("uniform_data".to_string()),
            gl_uniform_members: vec![
                RafxGlUniformMember::new("uniform_data.uniform_color", 0)
            ],
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
        log::trace!("Creating shader");
        let shader =
            device_context.create_shader(vec![vert_shader_stage_def, frag_shader_stage_def])?;

        //
        // Create the root signature object - it represents the pipeline layout and can be shared among
        // shaders. But one per shader is fine.
        //
        log::trace!("Creating root signature");
        let root_signature = device_context.create_root_signature(&RafxRootSignatureDef {
            shaders: &[shader.clone()],
            immutable_samplers: &[],
        })?;

        //
        // Descriptors are allocated in blocks and never freed. Normally you will want to build a
        // pooling system around this. (Higher-level rafx crates provide this.) But they're small
        // and cheap. We need one per swapchain image.
        //
        log::trace!("Creating descriptor set array");
        let mut descriptor_set_array =
            device_context.create_descriptor_set_array(&RafxDescriptorSetArrayDef {
                set_index: 0,
                root_signature: &root_signature,
                array_length: 3, // One per swapchain image.
            })?;

        // Initialize them all at once here.. this can be done per-frame as well.
        log::trace!("Set up descriptor sets");
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
                    byte_offset: 0,
                    gl_attribute_name: Some("pos".to_string())
                },
                RafxVertexLayoutAttribute {
                    format: RafxFormat::R32G32B32_SFLOAT,
                    buffer_index: 0,
                    location: 1,
                    byte_offset: 8,
                    gl_attribute_name: Some("in_color".to_string())
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

        log::trace!("Starting event loop");
        let start_time = instant::Instant::now();

        let mut i = 0;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } => *control_flow = ControlFlow::Exit,
                Event::MainEventsCleared => {
                    window.request_redraw();
                },
                Event::WindowEvent {
                    event: window_event,
                    window_id: _
                } => {
                    match window_event {
                        WindowEvent::KeyboardInput { .. } | WindowEvent::MouseInput { .. } => {
                            log::debug!("{:?}", window_event);
                        }
                        _ => {}
                    }
                },
                Event::RedrawRequested(_) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        i += 1;
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        i += 100;
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
                    let window_size = window.inner_size();
                    let presentable_frame =
                        swapchain_helper.acquire_next_image(window_size.width, window_size.height, None).unwrap();
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
                    vertex_buffer.copy_to_host_visible_buffer(&vertex_data).unwrap();
                    uniform_buffer.copy_to_host_visible_buffer(&uniform_data).unwrap();

                    //
                    // Record the command buffer. For now just transition it between layouts
                    //
                    cmd_pool.reset_command_pool().unwrap();
                    cmd_buffer.begin().unwrap();

                    // Put it into a layout where we can draw on it
                    cmd_buffer.cmd_resource_barrier(
                        &[],
                        &[RafxTextureBarrier::state_transition(
                            &swapchain_texture,
                            RafxResourceState::PRESENT,
                            RafxResourceState::RENDER_TARGET,
                        )],
                    ).unwrap();

                    cmd_buffer.cmd_begin_render_pass(
                        &[RafxColorRenderTargetBinding {
                            texture: &swapchain_texture,
                            load_op: RafxLoadOp::Clear,
                            store_op: RafxStoreOp::Store,
                            array_slice: None,
                            mip_slice: None,
                            clear_value: RafxColorClearValue([elapsed_seconds.sin() * 0.5 + 0.5, 0.0, 1.0, 1.0]),
                            resolve_target: None,
                            resolve_store_op: RafxStoreOp::DontCare,
                            resolve_mip_slice: None,
                            resolve_array_slice: None,
                        }],
                        None,
                    ).unwrap();

                    cmd_buffer.cmd_bind_pipeline(&pipeline).unwrap();

                    cmd_buffer.cmd_bind_vertex_buffers(
                        0,
                        &[RafxVertexBufferBinding {
                            buffer: &vertex_buffer,
                            byte_offset: 0,
                        }],
                    ).unwrap();

                    // cmd_buffer.cmd_bind_descriptor_set(
                    //     &descriptor_set_array,
                    //     presentable_frame.rotating_frame_index() as u32,
                    // ).unwrap();
                    // cmd_buffer.cmd_draw(3, 0).unwrap();
                    //
                    // // Put it into a layout where we can present it

                    cmd_buffer.cmd_end_render_pass().unwrap();

                    cmd_buffer.cmd_resource_barrier(
                        &[],
                        &[RafxTextureBarrier::state_transition(
                            &swapchain_texture,
                            RafxResourceState::RENDER_TARGET,
                            RafxResourceState::PRESENT,
                        )],
                    ).unwrap();
                    cmd_buffer.end().unwrap();

                    //
                    // Present the image
                    //
                    presentable_frame.present(&graphics_queue, &[&cmd_buffer]).unwrap();
                }
                _ => (),
            }
        });
    }
}

pub fn update() {

}