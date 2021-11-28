use crate::DemoArgs;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

pub fn main_native(args: &DemoArgs) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rafx Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(1600, 900))
        .build(&event_loop)
        .unwrap();

    crate::update_loop(&args, window, event_loop).unwrap();
}
