use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

pub fn main_native() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(log::LevelFilter::Debug)
        .init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Winit GL Example")
        .build(&event_loop)
        .unwrap();

    crate::update_loop(window, event_loop).unwrap();
}
