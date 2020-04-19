// This example shows how to use the renderer with SDL2 directly.

use renderer_shell_vulkan::{RendererBuilder, LogicalSize, ScaleToFit, Rect, CoordinateSystem, RendererEventListener, Window, VkDevice, VkSwapchain, Renderer, CreateRendererError};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ash::prelude::VkResult;
use renderer_ext::imgui_support::{ImguiRenderEventListener, VkImGuiRenderPassFontAtlas};
use imgui::sys::ImGuiStorage_GetBoolRef;
use sdl2::mouse::MouseState;

struct ResourceManager {

}

impl ResourceManager {
    pub fn new() -> Self {
        ResourceManager {}
    }
}

impl RendererEventListener for ResourceManager {
    fn swapchain_created(&mut self, device: &VkDevice, swapchain: &VkSwapchain) -> VkResult<()> {
        println!("resource manager swapchain created");
        VkResult::Ok(())
    }

    fn swapchain_destroyed(&mut self) {
        println!("resource manager swapchain destroyed");
    }

    fn render(&mut self, window: &Window, device: &VkDevice, present_index: usize) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        println!("resource manager render");
        VkResult::Ok(vec![])
    }
}

struct GameRenderer {
    // Handles uploading resources to GPU
    resource_manager: ResourceManager,

    imgui_event_listener: ImguiRenderEventListener,
}

impl GameRenderer {
    pub fn new(window: &dyn Window, imgui_font_atlas: VkImGuiRenderPassFontAtlas) -> Self {
        let mut resource_manager = ResourceManager::new();

        let imgui_event_listener = ImguiRenderEventListener::new(imgui_font_atlas);

        GameRenderer {
            resource_manager,
            imgui_event_listener,
        }
    }
}


impl RendererEventListener for GameRenderer {
    fn swapchain_created(&mut self, device: &VkDevice, swapchain: &VkSwapchain) -> VkResult<()> {
        println!("game renderer swapchain created");
        self.resource_manager.swapchain_created(device, swapchain)?;
        self.imgui_event_listener.swapchain_created(device, swapchain)?;
        VkResult::Ok(())
    }

    fn swapchain_destroyed(&mut self) {
        println!("game renderer swapchain destroyed");
        self.resource_manager.swapchain_destroyed();
        self.imgui_event_listener.swapchain_destroyed();
    }

    fn render(&mut self, window: &Window, device: &VkDevice, present_index: usize) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        println!("game renderer render");
        let mut command_buffers = vec![];

        {
            let mut commands = self.resource_manager.render(window, device, present_index)?;
            command_buffers.append(&mut commands);
        }

        {
            let mut commands = self.imgui_event_listener.render(window, device, present_index)?;
            command_buffers.append(&mut commands);
        }

        VkResult::Ok(command_buffers)
    }
}

struct GameRendererWithShell {
    game_renderer: GameRenderer,

    // Handles setting up device/instance/swapchain and windowing integration
    shell: Renderer
}

impl GameRendererWithShell {
    pub fn new(window: &dyn Window, imgui_font_atlas: VkImGuiRenderPassFontAtlas) -> Result<GameRendererWithShell, CreateRendererError> {
        let mut game_renderer = GameRenderer::new(window, imgui_font_atlas);

        let shell = RendererBuilder::new()
            .use_vulkan_debug_layer(true)
            .build(window, Some(&mut game_renderer))?;

        Ok(GameRendererWithShell {
            game_renderer,
            shell
        })
    }

    pub fn draw(&mut self, window: &dyn Window) -> VkResult<()> {
        self.shell.draw(window, Some(&mut self.game_renderer))
    }
}

impl Drop for GameRendererWithShell {
    fn drop(&mut self) {
        self.shell.tear_down(Some(&mut self.game_renderer));
    }
}

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

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
    let scale_to_fit = ScaleToFit::Center;
    let visible_range = Rect {
        left: 0.0,
        right: logical_size.width as f32,
        top: 0.0,
        bottom: logical_size.height as f32,
    };

    let sdl_window = video_subsystem
        .window("Skulpin", logical_size.width, logical_size.height)
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
        println!("Error during renderer construction: {:?}", e);
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

    'running: loop {
        for event in event_pump.poll_iter() {

            imgui_manager.handle_event(&event);
            if !imgui_manager.ignore_event(&event) {

                log::info!("{:?}", event);
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
                        log::info!("Key Down {:?} {:?}", keycode, modifiers);
                        if keycode == Keycode::Escape {
                            break 'running;
                        }
                    }

                    _ => {}
                }
            }
        }

        let window = Sdl2Window::new(&sdl_window);
        imgui_manager.begin_frame(&sdl_window, &MouseState::new(&event_pump));

        imgui_manager.with_ui(|ui| {
            let mut opened = true;
            ui.show_demo_window(&mut opened);
        });

        imgui_manager.render(&sdl_window);

        //
        // Redraw
        //
        renderer.draw(&window).unwrap();
    }
}
