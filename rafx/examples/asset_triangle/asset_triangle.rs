use log::LevelFilter;

use rafx::api::*;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::MaterialAsset;
use rafx::distill::loader::{storage::DefaultIndirectionResolver, Loader, RpcIO};
use rafx::framework::VertexDataLayout;
use rafx::graph::{
    RenderGraphBuilder, RenderGraphExecutor, RenderGraphImageConstraint, RenderGraphImageExtents,
    RenderGraphImageSpecification, RenderGraphNodeCallbacks, RenderGraphQueue,
    SwapchainSurfaceInfo,
};
use rafx::nodes::RenderPhase;
use rafx::nodes::RenderPhaseIndex;
use rafx::nodes::SubmitNode;
use std::sync::Arc;

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 600;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Info)
        .init();

    run().unwrap();
}

#[derive(Default, Clone, Copy)]
struct PositionColorVertex {
    position: [f32; 2],
    color: [f32; 3],
}

fn run() -> RafxResult<()> {
    //
    // For this example, we'll run the `distill` daemon in-process. This is the most convenient
    // method during development. (You could also build a packfile ahead of time and run from that)
    //
    let db_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/asset_triangle/.assets_db");
    let asset_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/asset_triangle/assets");
    let connect_string = "127.0.0.1:9999";

    // Daemon will runs in a background thread for the life of the process
    std::thread::spawn(move || {
        rafx::assets::distill_impl::default_daemon()
            .with_db_path(db_dir)
            .with_address(connect_string.parse().unwrap())
            .with_asset_dirs(vec![asset_dir])
            .run();
    });

    //
    // Init SDL2 (winit and anything that uses raw-window-handle works too!)
    //
    let sdl2_systems = sdl2_init();

    //
    // Create the api
    //
    let mut api = RafxApi::new(&sdl2_systems.window, &Default::default())?;

    // Wrap all of this so that it gets dropped before we drop the API object. This ensures a nice
    // clean shutdown.
    {
        // A cloneable device handle, these are lightweight and can be passed across threads
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
        // multithread friendly! The PresentableFrame it returns can be sent to another
        // thread and presented from there, and any errors are returned back to the main thread
        // when it acquires the next image. The helper also ensures that the swapchain is rebuilt
        // as necessary.
        //
        let mut swapchain_helper = RafxSwapchainHelper::new(&device_context, swapchain, None)?;

        //
        // Allocate a graphics queue. By default, there is just one graphics queue and it is shared.
        // There currently is no API for customizing this but the code would be easy to adapt to act
        // differently. Most recommendations I've seen are to just use one graphics queue. (The
        // rendering hardware is shared among them)
        //
        // Also create a transfer queue, it's used by the asset manager for async GPU upload
        //
        let graphics_queue = device_context.create_queue(RafxQueueType::Graphics)?;
        let transfer_queue = device_context.create_queue(RafxQueueType::Transfer)?;

        //
        // Create a ResourceContext. The render registry is more useful when there's a variety of
        // things to render, but since we just have a triangle we'll just set up a single phase.
        // (Multiple "features" can render in a single "phase". Sorting behavior for draw calls
        // across those features is defined by the phase)
        //
        let render_registry = rafx::nodes::RenderRegistryBuilder::default()
            .register_render_phase::<OpaqueRenderPhase>("Opaque")
            .build();

        //
        // Set up the client that connects to the distill daemon. The AssetResource is a utility
        // struct that encapsulates most of the distill-related parts of the asset pipeline
        // system and is something you can insert into an ECS.
        //
        let mut asset_resource = {
            let rpc_loader = RpcIO::new(connect_string.to_string()).unwrap();
            let loader = Loader::new(Box::new(rpc_loader));
            let resolver = Box::new(DefaultIndirectionResolver);
            AssetResource::new(loader, resolver)
        };

        //
        // Create the asset manager which encapsulates most of the rafx-related parts of the asset
        // pipeline. It creates a ResourceManager internally.
        //
        let mut asset_manager = rafx::assets::AssetManager::new(
            &device_context,
            &render_registry,
            asset_resource.loader(),
            rafx::assets::UploadQueueConfig {
                max_concurrent_uploads: 4,
                max_new_uploads_in_single_frame: 4,
                max_bytes_per_upload: 64 * 1024 * 1024,
            },
            &graphics_queue,
            &transfer_queue,
        );

        // // The asset resource by default is not set up to handle any asset types. You can use
        // // `add_default_asset_storage` to populate it with all the loaders implemented in rafx.
        // // Then use `add_storage_with_loader` to set up any additional types you have.
        // asset_resource.add_default_asset_storage(&asset_manager);
        asset_manager.register_default_asset_types(&mut asset_resource);

        // Grab a resource context for use later
        let resource_context = asset_manager.resource_manager().resource_context();

        //
        // Load the triangle material. Materials can contain multiple passes (but this one only has
        // one.) A material pass specifies shaders and fixed function state. Generally a material
        // is 1:1 with a GPU pipeline object with the material specifying *most* of the necessary
        // parameters to create the pipeline. (Some things like the size of the window are not
        // known until runtime.)
        //
        // When a material asset is loaded, rafx automatically creates shader modules, shaders,
        // descriptor set layouts, root signatures, and the material passes in it for you. They are
        // registered in the resource manager. We can use the handle to get the MaterialAsset which
        // has a reference to those resources. The resources will remain loaded until the handle
        // is dropped and there are no more references to those resources.
        //
        let triangle_material_handle =
            asset_resource.load_asset_path::<MaterialAsset, _>("triangle.material");

        //
        // The vertex format does not need to be specified up-front to create the material pass.
        // This allows a single material to be used with vertex data stored in any format. While we
        // don't need to create it just yet, we'll do it here once and put it in an arc so we can
        // easily use it later without having to reconstruct every frame.
        //
        let vertex_layout = Arc::new(
            VertexDataLayout::build_vertex_layout(
                &PositionColorVertex::default(),
                |builder, vertex| {
                    builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32_SFLOAT);
                    builder.add_member(&vertex.color, "COLOR", RafxFormat::R32G32B32_SFLOAT);
                },
            )
            .into_set(RafxPrimitiveTopology::TriangleList),
        );

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

            asset_resource.update();
            asset_manager.update_asset_loaders()?;

            let current_time = std::time::Instant::now();
            let seconds = (current_time - start_time).as_secs_f32();

            //
            // Acquire swapchain image
            //
            let (window_width, window_height) = sdl2_systems.window.vulkan_drawable_size();
            let presentable_frame =
                swapchain_helper.acquire_next_image(window_width, window_height, None)?;

            //
            // Mark the previous frame complete. This causes old resources that are no longer in
            // use to be dropped. It needs to go after the acquire image, because the acquire image
            // waits on the *gpu* to finish the frame.
            //
            asset_manager.on_frame_complete()?;

            // Try to do this as late as possible, but before rendering anything
            asset_manager.on_begin_frame()?;

            //
            // Register the swapchain image as a resource - this allows us to treat it like any
            // other resource. However keep in mind the image belongs to the swapchain. So holding
            // references to it beyond a single frame is dangerous!
            //
            let swapchain_image = resource_context
                .resources()
                .insert_image(presentable_frame.swapchain_texture().clone());

            let swapchain_image_view = resource_context
                .resources()
                .get_or_create_image_view(&swapchain_image, None)?;

            //
            // Create a graph to describe how we will draw the frame. Here we just have a single
            // renderpass with a color attachment. See the demo for more complex example usage.
            //
            let mut graph_builder = RenderGraphBuilder::default();
            let mut graph_callbacks = RenderGraphNodeCallbacks::<()>::default();

            let node = graph_builder.add_node("opaque", RenderGraphQueue::DefaultGraphics);
            let color_attachment = graph_builder.create_color_attachment(
                node,
                0,
                Some(RafxColorClearValue([0.0, 0.0, 0.0, 0.0])),
                RenderGraphImageConstraint {
                    samples: Some(RafxSampleCount::SampleCount4),
                    format: Some(swapchain_helper.format()),
                    ..Default::default()
                },
                Default::default(),
            );
            graph_builder.set_image_name(color_attachment, "color");

            //
            // Set a callback to be run when the graph is executed. We clone a few things and
            // capture them in this closure. We could alternatively create an arbitrary struct and
            // pass it in as a "user context".
            //
            // Only run it if the triangle material is loaded.
            //
            if let Some(triangle_material) =
                asset_manager.get_material_pass_by_index(&triangle_material_handle, 0)
            {
                let captured_vertex_layout = vertex_layout.clone();
                graph_callbacks.set_renderpass_callback(node, move |args| {
                    let vertex_layout = &captured_vertex_layout;
                    let material_pass = &triangle_material;

                    //
                    // Some data we will draw
                    //
                    #[rustfmt::skip]
                        let vertex_data = [
                        PositionColorVertex { position: [0.0, 0.5], color: [1.0, 0.0, 0.0] },
                        PositionColorVertex { position: [-0.5 + (seconds.cos() / 2. + 0.5), -0.5], color: [0.0, 1.0, 0.0] },
                        PositionColorVertex { position: [0.5 - (seconds.cos() / 2. + 0.5), -0.5], color: [0.0, 0.0, 1.0] },
                    ];

                    assert_eq!(20, std::mem::size_of::<PositionColorVertex>());

                    let color = (seconds.cos() + 1.0) / 2.0;
                    let uniform_data = [color, 0.0, 1.0 - color, 1.0];

                    //
                    // Here we create a vertex buffer. Since we only use it once we won't bother putting
                    // it into dedicated GPU memory.
                    //
                    // The vertex_buffer is ref-counted and can be kept around as long as you like. The
                    // resource manager will ensure it stays allocated until enough frames are presented
                    // that it's safe to delete.
                    //
                    // The resource allocators should be used and dropped, not kept around. They are
                    // pooled/re-used.
                    //
                    let resource_allocator = args.graph_context.resource_context().create_dyn_resource_allocator_set();
                    let vertex_buffer = args.graph_context.device_context().create_buffer(
                        &RafxBufferDef::for_staging_vertex_buffer_data(&vertex_data)
                    )?;

                    vertex_buffer.copy_to_host_visible_buffer(&vertex_data)?;

                    let vertex_buffer = resource_allocator.insert_buffer(vertex_buffer);

                    //
                    // Create a descriptor set. USUALLY - you can use the autogenerated code from the shader pipeline
                    // in higher level rafx crates to make this more straightforward - this is shown in the demo.
                    // Also, flush_changes is automatically called when dropped, we only have to call it
                    // here because we immediately use the descriptor set.
                    //
                    // Once the descriptor set is created, it's ref-counted and you can keep it around
                    // as long as you like. The resource manager will ensure it stays allocated
                    // until enough frames are presented that it's safe to delete.
                    //
                    // The allocator should be used and dropped, not kept around. It is pooled/re-used.
                    // flush_changes is automatically called on drop.
                    //
                    let descriptor_set_layout = material_pass
                        .get_raw()
                        .descriptor_set_layouts[0]
                        .clone();

                    let mut descriptor_set_allocator = args.graph_context.resource_context().create_descriptor_set_allocator();
                    let mut dyn_descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&descriptor_set_layout)?;
                    dyn_descriptor_set.set_buffer_data(0, &uniform_data);
                    dyn_descriptor_set.flush(&mut descriptor_set_allocator)?;
                    descriptor_set_allocator.flush_changes()?;

                    // At this point if we don't intend to change the descriptor, we can grab the
                    // descriptor set inside and use it as a ref-counted resource.
                    let descriptor_set = dyn_descriptor_set.descriptor_set();

                    //
                    // Fetch the pipeline. If we have a pipeline for this material that's compatible with
                    // the render target and vertex layout, we'll use it. Otherwise, we create it.
                    //
                    // The render phase is not really utilized to the full extent in this demo, but it
                    // would normally help pair materials with render targets, ensuring newly loaded
                    // materials can create pipelines ahead-of-time, off the render codepath.
                    //
                    let pipeline = args
                        .graph_context
                        .resource_context()
                        .graphics_pipeline_cache()
                        .get_or_create_graphics_pipeline(
                            OpaqueRenderPhase::render_phase_index(),
                            &material_pass,
                            &args.render_target_meta,
                            &vertex_layout
                        )?;

                    //
                    // We have everything needed to draw now, write instruction to the command buffer
                    //
                    let cmd_buffer = args.command_buffer;
                    cmd_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;
                    cmd_buffer.cmd_bind_vertex_buffers(
                        0,
                        &[RafxVertexBufferBinding {
                            buffer: &vertex_buffer.get_raw().buffer,
                            byte_offset: 0,
                        }],
                    )?;

                    descriptor_set.bind(&cmd_buffer)?;
                    cmd_buffer.cmd_draw(3, 0)?;

                    Ok(())
                });
            }

            //
            // Flag the color attachment as needing to output to the swapchain image. This is not a
            // copy - the graph walks backwards from outputs so that it operates directly on the
            // intended output image where possible. It only creates additional resources if
            // necessary.
            //
            graph_builder.set_output_image(
                color_attachment,
                swapchain_image_view,
                RenderGraphImageSpecification {
                    samples: RafxSampleCount::SampleCount1,
                    format: swapchain_helper.format(),
                    resource_type: RafxResourceType::TEXTURE
                        | RafxResourceType::RENDER_TARGET_COLOR,
                    extents: RenderGraphImageExtents::MatchSurface,
                    layer_count: 1,
                    mip_count: 1,
                },
                Default::default(),
                RafxResourceState::PRESENT,
            );

            //
            // Prepare to run the graph. We create an executor to allocate resources and run through
            // the graph, dispatching callbacks as needed to record instructions to command buffers
            //
            let swapchain_def = swapchain_helper.swapchain_def();
            let swapchain_surface_info = SwapchainSurfaceInfo {
                format: swapchain_helper.format(),
                extents: RafxExtents2D {
                    width: swapchain_def.width,
                    height: swapchain_def.height,
                },
            };

            let executor = RenderGraphExecutor::new(
                &device_context,
                &resource_context,
                graph_builder,
                &swapchain_surface_info,
                graph_callbacks,
            )?;

            //
            // Execute the graph. This will write out command buffer(s)
            //
            let command_buffers = executor.execute_graph(&(), &graphics_queue)?;

            //
            // Submit the command buffers to the GPU
            //
            let refs: Vec<&RafxCommandBuffer> = command_buffers.iter().map(|x| &**x).collect();
            presentable_frame.present(&graphics_queue, &refs)?;
        }

        // Wait for all GPU work to complete before destroying resources it is using
        graphics_queue.wait_for_queue_idle()?;
        transfer_queue.wait_for_queue_idle()?;

        // Drop resources before the asset manager. This ensures there are no remaining references
        // to resources that need to be cleaned up
        std::mem::drop(asset_resource);
    }

    // Optional, but calling this verifies that all rafx objects/device contexts have been
    // destroyed and where they were created. Good for finding unintended leaks!
    api.destroy()?;

    Ok(())
}

//
// A phase combines renderables that may come from different features. This example doesnt't use
// render nodes fully, but the pipeline cache uses it to define which renderpass/material pairs
//

rafx::declare_render_phase!(
    OpaqueRenderPhase,
    OPAQUE_RENDER_PHASE_INDEX,
    opaque_render_phase_sort_submit_nodes
);

#[profiling::function]
fn opaque_render_phase_sort_submit_nodes(mut submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
    // Sort by feature
    log::trace!(
        "Sort phase {}",
        OpaqueRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));

    submit_nodes
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
