use log::Level;
use wasm_bindgen::prelude::*;

use raw_window_handle::HasRawWindowHandle;
use winit::platform::web::WindowExtWebSys;
use winit::{event_loop::EventLoop, window::WindowBuilder};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_log::init_with_level(Level::Debug).unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Winit Web GL Example")
        .build(&event_loop)
        .unwrap();

    // Winit created a canvas element, we add it to the DOM here
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| {
            body.append_child(&web_sys::Element::from(window.canvas()))
                .ok()
        })
        .expect("couldn't append canvas to document body");

    log::debug!("{:?}", window.raw_window_handle());

    let result = crate::update_loop(window, event_loop);
    log::error!("Returned with result {:?}", result);

    Ok(())
}
