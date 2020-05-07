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
use renderer_ext::image_importer::ImageAsset;
use renderer_ext::image_utils::{DecodedTexture, enqueue_load_images};
use imgui::{Key, Image};
use renderer_ext::asset_storage::{StorageUploader, ResourceHandle};
use std::mem::ManuallyDrop;
//use renderer_ext::renderpass::sprite::LoadingSprite;
use crossbeam_channel::{Sender, Receiver};
use std::time::Duration;
use atelier_loader::AssetLoadOp;
use std::error::Error;
use renderer_ext::renderpass::sprite::{
    VkSpriteResourceManager, ImageUpdate, UploadQueue, ImageUploader,
};
use renderer_ext::gltf_importer::{MaterialAsset, MeshAsset};

fn load_asset<T>(asset_uuid: AssetUuid, asset_resource: &AssetResource) -> atelier_assets::loader::handle::Handle::<T> {
    use atelier_loader::Loader;
    let load_handle = asset_resource.loader().add_ref(asset_uuid);
    atelier_assets::loader::handle::Handle::<T>::new(
        asset_resource.tx().clone(),
        load_handle,
    )
}

fn main() {
    // let u32_value : u32 = 2000000000;
    // let u16_value : u16 = u32_value.try_into();


    //renderer_ext::test_gltf();
    //return;

    // Setup logging
    env_logger::Builder::from_default_env()
        //.filter_level(log::LevelFilter::Error)
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    std::thread::spawn(move || {
        daemon::run();
    });

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
    let mut image_upload_queue = UploadQueue::new(
        renderer.context().device_context(),
        //renderer.sprite_resource_manager().image_update_tx().clone(),
    );

    // Force an image to load and stay resident in memory
    let mut asset_resource = {
        let device_context = renderer.context().device_context();

        let mut asset_resource = AssetResource::default();
        asset_resource.add_storage_with_uploader::<ImageAsset, ImageUploader>(Box::new(
            ImageUploader::new(image_upload_queue.tx().clone(), renderer.sprite_resource_manager().image_update_tx().clone()),
        ));
        asset_resource.add_storage::<MaterialAsset>();
        asset_resource.add_storage::<MeshAsset>();
        asset_resource
    };

    let cat_handle = load_asset::<ImageAsset>(asset_uuid!("7c42f3bc-e96b-49f6-961b-5bfc799dee50"), &asset_resource);
    //let image_handle = load_asset::<ImageAsset>(asset_uuid!("337fe670-fb88-441e-bf87-33ed6fcfe269"), &asset_resource);
    //let material_handle = load_asset::<MaterialAsset>(asset_uuid!("742f5d82-0770-45de-907f-91ebe4834d7a"), &asset_resource);
    //let mesh_handle = load_asset::<MeshAsset>(asset_uuid!("25829306-59bb-4db3-a535-e542948abea0"), &asset_resource);

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
        image_upload_queue.update(renderer.context().device());

        imgui_manager.with_ui(|ui| {
            let mut opened = true;
            ui.show_demo_window(&mut opened);
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
