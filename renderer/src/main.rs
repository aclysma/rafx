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
use renderer_ext::pipeline_manager::{ShaderLoadHandler, PipelineLoadHandler};
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

fn write_example_pipeline_file(name: &'static str, pipeline: &dsc::GraphicsPipeline) {
    let pipeline_str = serde_json::to_string_pretty(&pipeline);
    match pipeline_str {
        Ok(string) => std::fs::File::create(format!("example_pipeline_{}.json.pipeline", name)).unwrap().write_all(string.as_bytes()).unwrap(),
        Err(err) => println!("Could not create json: {:?}", err)
    }

    let pipeline_str = ron::ser::to_string_pretty(&pipeline, ron::ser::PrettyConfig::default());
    match pipeline_str {
        Ok(string) => std::fs::File::create(format!("example_pipeline_{}.ron.pipeline", name)).unwrap().write_all(string.as_bytes()).unwrap(),
        Err(err) => println!("Could not create ron: {:?}", err)
    }
}

fn hash_pipeline(pipeline: &dsc::GraphicsPipeline) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    pipeline.hash(&mut hasher);
    hasher.finish()
}

fn write_example_pipeline_files() {
    let graphics_pipeline = dsc::GraphicsPipeline::default();
    write_example_pipeline_file("default", &graphics_pipeline);
    println!("default hash: {}", hash_pipeline(&graphics_pipeline));

    let graphics_pipeline = renderer_ext::renderpass::sprite_renderpass::create_sprite_pipeline();
    write_example_pipeline_file("sprite", &graphics_pipeline);
    println!("sprite hash: {}", hash_pipeline(&graphics_pipeline));

    let graphics_pipeline = create_kitchen_sink_pipeline();
    write_example_pipeline_file("kitchen_sink", &graphics_pipeline);
    println!("kitchen sink hash: {}", hash_pipeline(&graphics_pipeline));
}

fn main() {
    // let u32_value : u32 = 2000000000;
    // let u16_value : u16 = u32_value.try_into();

    //renderer_ext::test_gltf();
    //return;

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




    write_example_pipeline_files();



    //TODO: Could consider using json_comments


    // let pipeline_ron = ron::ser::to_string_pretty(&sprite_pipeline, ron::ser::PrettyConfig::default()).unwrap();
    // let pipeline_toml = toml::to_string_pretty(&sprite_pipeline).unwrap();
    // std::fs::File::create("pipeline_ron_example.ron").unwrap().write_all(pipeline_ron.as_bytes());
    // std::fs::File::create("pipeline_toml_example.toml").unwrap().write_all(pipeline_ron.as_bytes());




    let mut time = renderer_ext::time::TimeState::new();
    time.update();

    // Setup SDL
    let sdl_context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Set up the coordinate system to be fixed at 900x600, and use this as the default window size
    // This means the drawing code can be written as though the window is always 900x600. The
    // output will be automatically scaled so that it's always visible.
    let logical_size = LogicalSize {
        width: 900,
        height: 600,
    };

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

    let imgui_manager = renderer_ext::imgui_support::init_imgui_manager(&sdl_window);

    let window = Sdl2Window::new(&sdl_window);
    let renderer = GameRendererWithContext::new(&window, imgui_manager.build_font_atlas(), &time);

    // Check if there were error setting up vulkan
    if let Err(e) = renderer {
        log::error!("Error during renderer construction: {:?}", e);
        return;
    }

    log::info!("renderer created");

    let mut renderer = renderer.unwrap();

    // Increment a frame count so we can render something that moves
    let mut frame_count = 0;

    log::info!("Starting window event loop");
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Could not create sdl event pump");

    // Handles routing data between the asset system and sprite resource manager
    let mut upload_queue = UploadQueue::new(
        renderer.context().device_context(),
        //renderer.sprite_resource_manager().image_update_tx().clone(),
    );

    // Force an image to load and stay resident in memory
    let mut asset_resource = {
        let device_context = renderer.context().device_context();

        let mut asset_resource = AssetResource::default();
        asset_resource.add_storage_with_load_handler::<ShaderAsset, ShaderLoadHandler>(Box::new(
            renderer.pipeline_manager().create_shader_load_handler(),
        ));
        asset_resource.add_storage_with_load_handler::<PipelineAsset, PipelineLoadHandler>(Box::new(
            renderer.pipeline_manager().create_pipeline_load_handler(),
        ));
        // asset_resource.add_storage::<ShaderAsset>();
        // asset_resource.add_storage::<PipelineAsset>();
        asset_resource.add_storage_with_load_handler::<ImageAsset, ImageLoadHandler>(Box::new(
            ImageLoadHandler::new(
                upload_queue.pending_image_tx().clone(),
                renderer.image_resource_manager().image_update_tx().clone(),
                renderer.sprite_resource_manager().sprite_update_tx().clone(),
            ),
        ));
        asset_resource.add_storage_with_load_handler::<MaterialAsset, MaterialLoadHandler>(Box::new(
            MaterialLoadHandler::new(
                renderer.material_resource_manager().material_update_tx().clone(),
            )
        ));
        asset_resource.add_storage_with_load_handler::<MeshAsset, MeshLoadHandler>(Box::new(
            MeshLoadHandler::new(
                upload_queue.pending_buffer_tx().clone(),
                renderer.mesh_resource_manager().mesh_update_tx().clone(),
            ),
        ));
        asset_resource.add_storage_with_load_handler::<SpriteAsset, SpriteLoadHandler>(Box::new(
            SpriteLoadHandler::new(
                renderer.sprite_resource_manager().sprite_update_tx().clone(),
            ),
        ));
        asset_resource
    };

    //IMAGE
    // let cat_handle = load_asset::<ImageAsset>(
    //     asset_uuid!("7c42f3bc-e96b-49f6-961b-5bfc799dee50"),
    //     &asset_resource,
    // );
    //let image_handle = load_asset::<ImageAsset>(asset_uuid!("337fe670-fb88-441e-bf87-33ed6fcfe269"), &asset_resource);

    //MATERIAL
    //let material_handle = load_asset::<MaterialAsset>(asset_uuid!("742f5d82-0770-45de-907f-91ebe4834d7a"), &asset_resource);

    //MESHES
    // 3objects
    // let mesh_handle = load_asset::<MeshAsset>(
    //     asset_uuid!("25829306-59bb-4db3-a535-e542948abea0"),
    //     &asset_resource,
    // );

    // unit_cube
    //let mesh_handle = load_asset::<MeshAsset>(asset_uuid!("5c7c907a-9335-4d4a-bb61-4f0c7ff03d07"), &asset_resource);
    // textured cube
    let mesh_handle = load_asset::<MeshAsset>(asset_uuid!("6b33207a-241c-41ba-9149-3e678557a45c"), &asset_resource);

    //SPRITE
    let sprite_handle = load_asset::<SpriteAsset>(asset_uuid!("0be51c83-73a1-4780-984a-7e4accc65ae7"), &asset_resource);

    //PIPELINE
    let pipeline = load_asset::<PipelineAsset>(asset_uuid!("32c20111-bc4a-4dc7-bdf4-85d620ba199a"), &asset_resource);


    let mut print_time_event = renderer_ext::time::PeriodicEvent::default();

    'running: loop {
        for event in event_pump.poll_iter() {
            imgui_manager.handle_event(&event);
            if !imgui_manager.ignore_event(&event) {
                log::trace!("{:?}", event);
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
                        log::trace!("Key Down {:?} {:?}", keycode, modifiers);
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
        // use atelier_loader::handle::TypedAssetStorage;
        // let a : Option<&MaterialAsset> = material_handle.asset(asset_resource.storage());
        // match a {
        //     Some(material) => {
        //         println!("material color {:?}", material.base_color);
        //     },
        //     None => {
        //         println!("material not loaded");
        //     }
        // }

        let window = Sdl2Window::new(&sdl_window);
        imgui_manager.begin_frame(&sdl_window, &MouseState::new(&event_pump));

        asset_resource.update();
        upload_queue.update(renderer.context().device());
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
