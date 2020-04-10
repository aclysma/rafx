// This example shows how to use the renderer with SDL2 directly.

use renderer_base::{RendererBuilder, LogicalSize, ScaleToFit, Rect, CoordinateSystem};
use renderer_sdl2::Sdl2Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

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

    let window = Sdl2Window::new(&sdl_window);

    let renderer = RendererBuilder::new()
        .use_vulkan_debug_layer(true)
        .build(&window);

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

        //
        // Redraw
        //
        renderer.draw(&window).unwrap();
    }
}
