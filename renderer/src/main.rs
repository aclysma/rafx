use renderer_shell_vulkan::{
    LogicalSize, VkSurfaceEventListener, Window, VkDevice, VkSwapchain, VkSurface, VkDeviceContext,
    VkTransferUpload, VkTransferUploadState, VkImage,
};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ash::prelude::VkResult;
use renderer_ext::imgui_support::{VkImGuiRenderPassFontAtlas};
use imgui::sys::ImGuiStorage_GetBoolRef;
use sdl2::mouse::MouseState;
use renderer_ext::GameRendererWithContext;
use image::{GenericImageView, load};
use atelier_assets::loader as atelier_loader;

use atelier_assets::core::asset_uuid;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;

mod daemon;
use renderer_ext::asset_resource::AssetResource;
use renderer_ext::image_utils::{DecodedTexture, enqueue_load_images};
use imgui::{Key, Image};
use renderer_ext::asset_storage::{ResourceLoadHandler, ResourceHandle};
use std::mem::ManuallyDrop;
//use renderer_ext::renderpass::sprite::LoadingSprite;
use crossbeam_channel::{Sender, Receiver};
use std::time::Duration;
use atelier_loader::AssetLoadOp;
use std::error::Error;
use renderer_ext::upload::UploadQueue;
use renderer_ext::load_handlers::{ImageLoadHandler, MeshLoadHandler, MaterialLoadHandler, SpriteLoadHandler};
//use renderer_ext::pipeline_manager::{ShaderLoadHandler, PipelineLoadHandler};
use renderer_ext::pipeline::image::ImageAsset;
use renderer_ext::pipeline::gltf::{MaterialAsset, MeshAsset};
use renderer_ext::pipeline::sprite::SpriteAsset;
use renderer_ext::pipeline_description::GraphicsPipeline;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;

fn load_asset<T>(
    asset_uuid: AssetUuid,
    asset_resource: &AssetResource,
) -> atelier_assets::loader::handle::Handle<T> {
    use atelier_loader::Loader;
    let load_handle = asset_resource.loader().add_ref(asset_uuid);
    atelier_assets::loader::handle::Handle::<T>::new(asset_resource.tx().clone(), load_handle)
}

use renderer_ext::pipeline_description as dsc;
use renderer_ext::pipeline::shader::ShaderAsset;
use renderer_ext::pipeline::pipeline::PipelineAsset;
use std::hint::unreachable_unchecked;

fn create_kitchen_sink_pipeline() -> dsc::GraphicsPipeline {
    let mut kitchen_sink_pipeline = dsc::GraphicsPipeline::default();
    kitchen_sink_pipeline.pipeline_layout.descriptor_set_layouts = vec![
        dsc::DescriptorSetLayout {
            descriptor_set_layout_bindings: vec! [
                Default::default(),
                dsc::DescriptorSetLayoutBinding {
                    binding: 1,
                    ..Default::default()
                },
            ]
        },
        dsc::DescriptorSetLayout {
            descriptor_set_layout_bindings: vec! [
                Default::default()
            ]
        }
    ];
    kitchen_sink_pipeline.pipeline_layout.push_constant_ranges = vec![
        dsc::PushConstantRange {
            ..Default::default()
        }
    ];
    kitchen_sink_pipeline.renderpass.attachments = vec![
        Default::default()
    ];

    kitchen_sink_pipeline.renderpass.subpasses = vec![
        Default::default()
    ];
    kitchen_sink_pipeline.renderpass.dependencies = vec![
        Default::default()
    ];
    kitchen_sink_pipeline.fixed_function_state.vertex_input_state.binding_descriptions = vec![
        Default::default()
    ];

    kitchen_sink_pipeline.fixed_function_state.vertex_input_state.attribute_descriptions = vec![
        Default::default()
    ];
    kitchen_sink_pipeline.fixed_function_state.viewport_state.viewports = vec![
        Default::default()
    ];
    kitchen_sink_pipeline.fixed_function_state.viewport_state.scissors = vec![
        Default::default()
    ];
    kitchen_sink_pipeline.fixed_function_state.color_blend_state.attachments = vec![
        Default::default()
    ];
    kitchen_sink_pipeline.fixed_function_state.dynamic_state.dynamic_states = vec![
        dsc::DynamicState::Viewport,
        dsc::DynamicState::Scissor
    ];

    kitchen_sink_pipeline.pipeline_shader_stages.stages = vec![
        Default::default()
    ];

    kitchen_sink_pipeline
}

/*
fn create_sprite_pipeline() -> dsc::GraphicsPipeline {
    use renderer_ext::renderpass::SpriteVertex;

    let mut sprite_pipeline = dsc::GraphicsPipeline::default();
    sprite_pipeline.pipeline_layout.descriptor_set_layouts = vec![
        dsc::DescriptorSetLayout {
            descriptor_set_layout_bindings: vec! [
                dsc::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: dsc::DescriptorType::UniformBuffer,
                    descriptor_count: 1,
                    stage_flags: dsc::ShaderStageFlags::Vertex
                },
                dsc::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: dsc::DescriptorType::Sampler,
                    descriptor_count: 1,
                    stage_flags: dsc::ShaderStageFlags::Fragment
                },
            ]
        },
        dsc::DescriptorSetLayout {
            descriptor_set_layout_bindings: vec! [
                dsc::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: dsc::DescriptorType::SampledImage,
                    descriptor_count: 1,
                    stage_flags: dsc::ShaderStageFlags::Fragment
                }
            ]
        }
    ];
    sprite_pipeline.fixed_function_state.input_assembly_state.primitive_topology = dsc::PrimitiveTopology::TriangleList;
    sprite_pipeline.fixed_function_state.vertex_input_state.binding_descriptions = vec![
        dsc::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<renderer_ext::renderpass::SpriteVertex>() as u32,
            input_rate: dsc::VertexInputRate::Vertex
        }
    ];
    sprite_pipeline.fixed_function_state.vertex_input_state.attribute_descriptions = vec![
        dsc::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: dsc::Format::R32G32_SFLOAT,
            offset: renderer_shell_vulkan::offset_of!(renderer_ext::renderpass::SpriteVertex, pos) as u32,
        },
        dsc::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: dsc::Format::R32G32_SFLOAT,
            offset: renderer_shell_vulkan::offset_of!(renderer_ext::renderpass::SpriteVertex, tex_coord) as u32,
        },
    ];

    sprite_pipeline.fixed_function_state.viewport_state.viewports = vec![
        dsc::Viewport {
            dimensions: Default::default(),
            min_depth: Decimal::from_f32(0.0).unwrap(),
            max_depth: Decimal::from_f32(1.0).unwrap(),
        }
    ];
    sprite_pipeline.fixed_function_state.viewport_state.scissors = vec![
        Default::default()
    ];

    sprite_pipeline.fixed_function_state.rasterization_state = dsc::PipelineRasterizationState {
        front_face: dsc::FrontFace::CounterClockwise,
        line_width: Decimal::from_f32(1.0).unwrap(),
        polygon_mode: dsc::PolygonMode::Fill,
        cull_mode: dsc::CullModeFlags::None,
        ..Default::default()
    };

    sprite_pipeline.fixed_function_state.multisample_state.rasterization_samples = dsc::SampleCountFlags::SampleCount1;

    sprite_pipeline.fixed_function_state.color_blend_state.attachments = vec![
        dsc::PipelineColorBlendAttachmentState {
            color_write_mask: dsc::ColorComponentFlags {
                red: true,
                green: true,
                blue: true,
                alpha: true
            },
            blend_enable: true,
            src_color_blend_factor: dsc::BlendFactor::SrcAlpha,
            dst_color_blend_factor: dsc::BlendFactor::OneMinusSrcAlpha,
            color_blend_op: dsc::BlendOp::Add,
            src_alpha_blend_factor: dsc::BlendFactor::One,
            dst_alpha_blend_factor: dsc::BlendFactor::Zero,
            alpha_blend_op: dsc::BlendOp::Add
        }
    ];

    sprite_pipeline
}
*/

// fn write_example_pipeline_file(name: &'static str, pipeline: &dsc::GraphicsPipeline) {
//     let pipeline_str = serde_json::to_string_pretty(&pipeline);
//     match pipeline_str {
//         Ok(string) => std::fs::File::create(format!("example_pipeline_{}.json.pipeline", name)).unwrap().write_all(string.as_bytes()).unwrap(),
//         Err(err) => println!("Could not create json: {:?}", err)
//     }
//
//     let pipeline_str = ron::ser::to_string_pretty(&pipeline, ron::ser::PrettyConfig::default());
//     match pipeline_str {
//         Ok(string) => std::fs::File::create(format!("example_pipeline_{}.ron.pipeline", name)).unwrap().write_all(string.as_bytes()).unwrap(),
//         Err(err) => println!("Could not create ron: {:?}", err)
//     }
// }
//
// fn hash_pipeline(pipeline: &dsc::GraphicsPipeline) -> u64 {
//     use std::hash::{Hash, Hasher};
//     let mut hasher = DefaultHasher::new();
//     pipeline.hash(&mut hasher);
//     hasher.finish()
// }

// fn write_example_pipeline_files() {
//     let graphics_pipeline = dsc::GraphicsPipeline::default();
//     write_example_pipeline_file("default", &graphics_pipeline);
//     println!("default hash: {}", hash_pipeline(&graphics_pipeline));
//
//     let graphics_pipeline = renderer_ext::renderpass::sprite_renderpass::create_sprite_pipeline();
//     write_example_pipeline_file("sprite", &graphics_pipeline);
//     println!("sprite hash: {}", hash_pipeline(&graphics_pipeline));
//
//     let graphics_pipeline = create_kitchen_sink_pipeline();
//     write_example_pipeline_file("kitchen_sink", &graphics_pipeline);
//     println!("kitchen sink hash: {}", hash_pipeline(&graphics_pipeline));
// }

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_module("renderer_shell_vulkan::buffer", log::LevelFilter::Debug)
        .filter_module("renderer_ext::game_renderer", log::LevelFilter::Debug)
        //.filter_level(log::LevelFilter::Error)
        .filter_level(log::LevelFilter::Trace)
        .init();

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    std::thread::spawn(move || {
        daemon::run();
    });

    // Something to track time
    let mut time = renderer_ext::time::TimeState::new();
    time.update();

    // Setup SDL
    let sdl_context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Default window size
    let logical_size = LogicalSize {
        width: 900,
        height: 600,
    };

    // Create the window
    let sdl_window = video_subsystem
        .window(
            "Renderer Prototype",
            logical_size.width,
            logical_size.height,
        )
        .position_centered()
        .allow_highdpi()
        .resizable()
        .vulkan()
        .build()
        .expect("Failed to create window");

    log::info!("window created");

    // Load imgui, we do it a little early because it wants to have the actual SDL2 window and
    // doesn't work with the thin window wrapper
    let imgui_manager = renderer_ext::imgui_support::init_imgui_manager(&sdl_window);

    // Thin window wrapper to decouple the renderer from a specific windowing crate
    let window = Sdl2Window::new(&sdl_window);

    // Assets will be stored here, we init it ahead of the renderer as it will register its own
    // asset types
    let mut asset_resource = AssetResource::default();

    // Create the renderer, this will init the vulkan instance, device, and set up a swap chain
    let renderer = GameRendererWithContext::new(
        &window,
        imgui_manager.build_font_atlas(),
        &time,
        &mut asset_resource
    );

    // Check if there were error setting up vulkan
    if let Err(e) = renderer {
        log::error!("Error during renderer construction: {:?}", e);
        return;
    }

    log::info!("renderer created");

    let mut renderer = renderer.unwrap();

    log::info!("Starting window event loop");
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Could not create sdl event pump");


    fn wait_for_asset_to_load<T>(asset_handle: &atelier_assets::loader::handle::Handle<T>, asset_resource: &mut AssetResource, renderer: &mut GameRendererWithContext) {
        loop {
            asset_resource.update();
            renderer.update_resources();
            use atelier_assets::loader::LoadStatus;
            use atelier_loader::handle::AssetHandle;
            match asset_handle.load_status(asset_resource.loader()) {
                LoadStatus::NotRequested => {
                    unreachable!();
                },
                LoadStatus::Loading => {
                    // keep waiting
                },
                LoadStatus::Loaded => {
                    break;
                },
                LoadStatus::Unloading => { unreachable!() },
                LoadStatus::DoesNotExist => {
                    println!("Essential asset not found");
                },
                LoadStatus::Error(err) => {
                    println!("Error loading essential asset {:?}", err);
                },
            }
        }
    }

    //PIPELINE
    let pipeline = load_asset::<PipelineAsset>(asset_uuid!("32c20111-bc4a-4dc7-bdf4-85d620ba199a"), &asset_resource);
    let pipeline_variant = load_asset::<PipelineAsset>(asset_uuid!("38126811-1892-41f9-80b0-64d9b5bdcad2"), &asset_resource);
    wait_for_asset_to_load(&pipeline, &mut asset_resource, &mut renderer);
    wait_for_asset_to_load(&pipeline_variant, &mut asset_resource, &mut renderer);

    let mesh_handle = load_asset::<MeshAsset>(asset_uuid!("6b33207a-241c-41ba-9149-3e678557a45c"), &asset_resource);

    //SPRITE
    let sprite_handle = load_asset::<SpriteAsset>(asset_uuid!("0be51c83-73a1-4780-984a-7e4accc65ae7"), &asset_resource);


    let mut print_time_event = renderer_ext::time::PeriodicEvent::default();

    'running: loop {
        for event in event_pump.poll_iter() {
            imgui_manager.handle_event(&event);
            if !imgui_manager.ignore_event(&event) {
                //log::trace!("{:?}", event);
                match event {
                    //
                    // Halt if the user requests to close the window
                    //
                    Event::Quit { .. } => break 'running,

                    //
                    // Close if the escape key is hit
                    //
                    Event::KeyDown {
                        keycode: Some(keycode),
                        keymod: modifiers,
                        ..
                    } => {
                        //log::trace!("Key Down {:?} {:?}", keycode, modifiers);
                        if keycode == Keycode::Escape {
                            break 'running;
                        }

                        if keycode == Keycode::D {
                            renderer.dump_stats();
                        }
                    }

                    _ => {}
                }
            }
        }

        let window = Sdl2Window::new(&sdl_window);
        imgui_manager.begin_frame(&sdl_window, &MouseState::new(&event_pump));

        asset_resource.update();
        //upload_queue.update(renderer.context().device());
        renderer.update_resources();

        imgui_manager.with_ui(|ui| {
            //let mut opened = true;
            //ui.show_demo_window(&mut opened);
            ui.main_menu_bar(|| {
                ui.text(imgui::im_str!(
                    "FPS: {:.1}",
                    time.updates_per_second_smoothed()
                ));
            });
        });

        imgui_manager.render(&sdl_window);

        //
        // Redraw
        //
        renderer.draw(&window, &time).unwrap();
        time.update();

        if print_time_event.try_take_event(
            time.current_instant(),
            std::time::Duration::from_secs_f32(1.0),
        ) {
            println!("FPS: {}", time.updates_per_second());
            //renderer.dump_stats();
        }
    }
}
