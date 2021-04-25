use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

pub fn main_native() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Winit GL Example")
        .build(&event_loop)
        .unwrap();

    crate::update_loop(window, event_loop).unwrap();
}
