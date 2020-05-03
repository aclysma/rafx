// This example shows how to use the renderer with SDL2 directly.

use renderer_shell_vulkan::{RendererBuilder, LogicalSize, RendererEventListener, Window, VkDevice, VkSwapchain, Renderer, CreateRendererError, VkDeviceContext, VkTransferUpload, VkTransferUploadState, VkImage};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ash::prelude::VkResult;
use renderer_ext::imgui_support::{VkImGuiRenderPassFontAtlas};
use imgui::sys::ImGuiStorage_GetBoolRef;
use sdl2::mouse::MouseState;
use renderer_ext::GameRendererWithShell;
use image::{GenericImageView, load};
use atelier_assets::loader as atelier_loader;


use atelier_assets::core::asset_uuid;
use atelier_assets::core as atelier_core;

mod daemon;
use renderer_ext::asset_resource::AssetResource;
use renderer_ext::image_importer::ImageAsset;
use renderer_ext::image_utils::{DecodedTexture, enqueue_load_images};
use imgui::{Key, Image};
use renderer_ext::asset_storage::{StorageUploader, ResourceHandle};
use std::mem::ManuallyDrop;
//use renderer_ext::renderpass::sprite::LoadingSprite;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;
use atelier_loader::AssetLoadOp;
use std::error::Error;
use renderer_ext::renderpass::sprite::{VkSpriteResourceManager, SpriteUpdate};

// This is registered with the asset storage which lets us hook when assets are updated
struct ImageUploader {
    device_context: VkDeviceContext,
    tx: Sender<PendingImageUpload>
}

impl ImageUploader {
    pub fn new(device_context: VkDeviceContext, tx: Sender<PendingImageUpload>) -> Self {
        ImageUploader {
            device_context,
            tx
        }
    }
}

impl StorageUploader<ImageAsset> for ImageUploader {
    fn upload(
        &self,
        asset: &ImageAsset,
        load_op: AssetLoadOp,
        resource_handle: ResourceHandle<ImageAsset>
    ) {
        let texture = DecodedTexture {
            width: asset.width,
            height: asset.height,
            data: asset.data.clone()
        };

        //TODO: This is not respecting commit() - it just updates sprites as soon as it can
        self.tx.send(PendingImageUpload {
            load_op,
            texture,
            resource_handle
        }).unwrap(); //TODO: Better error handling
    }

    fn free(&self, resource_handle: ResourceHandle<ImageAsset>) {
        //TODO: We are not unloading images
    }
}









// A message sent to ImageUploadQueue
struct PendingImageUpload {
    load_op: AssetLoadOp,
    texture: DecodedTexture,
    resource_handle: ResourceHandle<ImageAsset>,
}

// The result from polling a single upload (which may contain multiple images in it)
pub enum InProgressImageUploadPollResult {
    Pending,
    Complete(Vec<ManuallyDrop<VkImage>>, Vec<ResourceHandle<ImageAsset>>),
    Error(Box<Error + 'static + Send>),
    Destroyed
}

// This is an inner of InProgressImageUpload - it is wrapped in a Option to avoid borrowing issues
// when polling by allowing us to temporarily take ownership of it and then put it back
struct InProgressImageUploadInner {
    load_ops: Vec<AssetLoadOp>,
    images: Vec<ManuallyDrop<VkImage>>,
    resource_handles: Vec<ResourceHandle<ImageAsset>>,
    upload: VkTransferUpload
}

// A single upload which may contain multiple images
struct InProgressImageUpload {
    inner: Option<InProgressImageUploadInner>
}

impl InProgressImageUpload {
    pub fn new(
        load_ops: Vec<AssetLoadOp>,
        images: Vec<ManuallyDrop<VkImage>>,
        resource_handles: Vec<ResourceHandle<ImageAsset>>,
        upload: VkTransferUpload
    ) -> Self {
        let inner = InProgressImageUploadInner {
            load_ops,
            images,
            resource_handles,
            upload
        };

        InProgressImageUpload {
            inner: Some(inner)
        }
    }

    // The main state machine for an upload:
    // - Submits on the transfer queue and waits
    // - Submits on the graphics queue and waits
    //
    // Calls load_op.complete() or load_op.error() as appropriate
    pub fn poll_load(
        &mut self,
        device: &VkDevice
    ) -> InProgressImageUploadPollResult {
        loop {
            if let Some(mut inner) = self.take_inner() {
                match inner.upload.state() {
                    Ok(state) => {
                        match state {
                            VkTransferUploadState::Writable => {
                                println!("VkTransferUploadState::Writable");
                                inner.upload.submit_transfer(device.queues.transfer_queue);
                                self.inner = Some(inner);
                            },
                            VkTransferUploadState::SentToTransferQueue => {
                                println!("VkTransferUploadState::SentToTransferQueue");
                                self.inner = Some(inner);
                                break InProgressImageUploadPollResult::Pending;
                            },
                            VkTransferUploadState::PendingSubmitDstQueue => {
                                println!("VkTransferUploadState::PendingSubmitDstQueue");
                                inner.upload.submit_dst(device.queues.graphics_queue);
                                self.inner = Some(inner);
                            },
                            VkTransferUploadState::SentToDstQueue => {
                                println!("VkTransferUploadState::SentToDstQueue");
                                self.inner = Some(inner);
                                break InProgressImageUploadPollResult::Pending;
                            },
                            VkTransferUploadState::Complete => {
                                println!("VkTransferUploadState::Complete");
                                for load_op in inner.load_ops {
                                    load_op.complete();
                                }
                                break InProgressImageUploadPollResult::Complete(inner.images, inner.resource_handles);
                            },
                        }
                    },
                    Err(err) => {
                        for load_op in inner.load_ops {
                            load_op.error(err);
                        }
                        break InProgressImageUploadPollResult::Error(Box::new(err));
                    },
                }
            } else {
                break InProgressImageUploadPollResult::Destroyed;
            }
        }
    }

    // Allows taking ownership of the inner object
    fn take_inner(&mut self) -> Option<InProgressImageUploadInner> {
        let mut inner = None;
        std::mem::swap(&mut self.inner, &mut inner);
        inner
    }
}

// Receives sets of images that need to be uploaded and kicks off the upload
struct ImageUploadQueue {
    device_context: VkDeviceContext,

    // The ImageUploader associated with the asset storage passes messages via these channels.
    tx: Sender<PendingImageUpload>,
    rx: Receiver<PendingImageUpload>,

    // These are uploads that are currently in progress
    uploads_in_progress: Vec<InProgressImageUpload>,

    // This channel forwards completed uploads to the sprite resource manager
    sprite_update_tx: Sender<SpriteUpdate>
}

impl ImageUploadQueue {
    pub fn new(device: &VkDevice, sprite_update_tx: Sender<SpriteUpdate>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        ImageUploadQueue {
            device_context: device.context.clone(),
            tx,
            rx,
            uploads_in_progress: Default::default(),
            sprite_update_tx
        }
    }

    pub fn tx(&self) -> &Sender<PendingImageUpload> {
        &self.tx
    }

    fn start_new_uploads(&mut self) -> VkResult<()> {
        let mut load_ops = vec![];
        let mut decoded_textures = vec![];
        let mut resource_handles = vec![];

        while let Ok(pending_upload) = self.rx.recv_timeout(Duration::from_secs(0)) {
            load_ops.push(pending_upload.load_op);
            decoded_textures.push(pending_upload.texture);
            resource_handles.push(pending_upload.resource_handle);

            //TODO: Handle budgeting how much we can upload at once
        }

        if decoded_textures.is_empty() {
            return Ok(());
        }

        let mut upload = VkTransferUpload::new(
            &self.device_context,
            self.device_context.queue_family_indices().transfer_queue_family_index,
            self.device_context.queue_family_indices().graphics_queue_family_index,
            1024 * 1024 * 16
        )?;

        let images = enqueue_load_images(
            &self.device_context,
            &mut upload,
            self.device_context.queue_family_indices().transfer_queue_family_index,
            self.device_context.queue_family_indices().graphics_queue_family_index,
            &decoded_textures
        )?;

        upload.submit_transfer(self.device_context.queues().transfer_queue)?;
        self.uploads_in_progress.push(InProgressImageUpload::new(
            load_ops,
            images,
            resource_handles,
            upload
        ));

        Ok(())
    }

    fn update_existing_uploads(&mut self, device: &VkDevice) {
        // iterate backwards so we can use swap_remove
        for i in (0..self.uploads_in_progress.len()).rev() {
            let result = self.uploads_in_progress[i].poll_load(device);
            match result {
                InProgressImageUploadPollResult::Pending => {
                    // do nothing
                },
                InProgressImageUploadPollResult::Complete(images, resource_handles) => {
                    let upload = self.uploads_in_progress.swap_remove(i);
                    self.sprite_update_tx.send(SpriteUpdate {
                        images,
                        resource_handles
                    });
                },
                InProgressImageUploadPollResult::Error(e) => {
                    let upload = self.uploads_in_progress.swap_remove(i);
                    //TODO: error() probably needs to accept a box so we can relay the error
                    // image.load_op.error(e);
                },
                InProgressImageUploadPollResult::Destroyed => {
                    // not expected - this only occurs if polling the upload when it is already in a complete or error state
                    unreachable!();
                }
            }
        }
    }

    pub fn update(&mut self, device: &VkDevice) -> VkResult<()> {
        self.start_new_uploads()?;
        self.update_existing_uploads(device);
        Ok(())
    }
}

fn main() {
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
        .window("Renderer Prototype", logical_size.width, logical_size.height)
        .position_centered()
        .allow_highdpi()
        .resizable()
        .vulkan()
        .build()
        .expect("Failed to create window");
    log::info!("window created");

    let imgui_manager = renderer_ext::imgui_support::init_imgui_manager(&sdl_window);

    let window = Sdl2Window::new(&sdl_window);
    let renderer = GameRendererWithShell::new(&window, imgui_manager.build_font_atlas());

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

    let mut time = renderer_ext::time::TimeState::new();
    time.update();


    let mut image_upload_queue = ImageUploadQueue::new(
        renderer.shell().device(),
        renderer.sprite_resource_manager().unwrap().sprite_update_tx().clone()
    );

    let (mut asset_resource, image_handle) = {
        let device = renderer.shell().device();
        let device_context = &device.context;


        let mut asset_resource = AssetResource::default();
        asset_resource.add_storage_with_uploader::<ImageAsset, ImageUploader>(Box::new(ImageUploader::new(
            device_context.clone(),
            image_upload_queue.tx().clone()
        )));



        let asset_uuid = asset_uuid!("d60aa147-e1c7-42dc-9e99-40ba882544a7");

        use atelier_assets::loader::Loader;
        use atelier_assets::loader::handle::AssetHandle;

        let load_handle = asset_resource.loader().add_ref(asset_uuid);
        let image_handle = atelier_assets::loader::handle::Handle::<ImageAsset>::new(
            asset_resource.tx().clone(),
            load_handle,
        );

        // let version = loop {
        //     asset_resource.update();
        //     if let atelier_assets::loader::LoadStatus::Loaded = image_handle
        //         .load_status::<atelier_assets::loader::rpc_loader::RpcLoader>(
        //             asset_resource.loader(),
        //         ) {
        //         break image_handle
        //             .asset_version::<ImageAsset, _>(asset_resource.storage())
        //             .unwrap();
        //     }
        // };

        // let image_asset = image_handle.asset(asset_resource.storage()).unwrap();
        // let decoded_image = DecodedTexture {
        //     width: image_asset.width,
        //     height: image_asset.height,
        //     data: image_asset.data.clone()
        // };

        (asset_resource, image_handle)
    };






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

        let window = Sdl2Window::new(&sdl_window);
        imgui_manager.begin_frame(&sdl_window, &MouseState::new(&event_pump));

        asset_resource.update();
        image_upload_queue.update(renderer.shell().device());

        imgui_manager.with_ui(|ui| {
            let mut opened = true;
            ui.show_demo_window(&mut opened);
        });

        imgui_manager.render(&sdl_window);

        //
        // Redraw
        //
        renderer.draw(&window).unwrap();
        time.update();

        if print_time_event.try_take_event(time.current_instant(), std::time::Duration::from_secs_f32(1.0)) {
            println!("FPS: {}", time.updates_per_second());
            //renderer.dump_stats();
        }
    }
}
