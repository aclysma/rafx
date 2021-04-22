use web_sys::WebGlRenderingContext;
use wasm_bindgen::prelude::*;
use log::Level;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use raw_window_handle::HasRawWindowHandle;
use winit::platform::web::WindowExtWebSys;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue>
{
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

    // let mut i = 0;
    // event_loop.run(move |event, _, control_flow| {
    //     *control_flow = ControlFlow::Poll;
    //
    //     match event {
    //         Event::WindowEvent {
    //             event: WindowEvent::CloseRequested,
    //             window_id,
    //         } if window_id == window.id() => *control_flow = ControlFlow::Exit,
    //         Event::MainEventsCleared => {
    //             window.request_redraw();
    //         },
    //         Event::WindowEvent {
    //             event: window_event,
    //             window_id: _
    //         } => {
    //             match window_event {
    //                 WindowEvent::KeyboardInput { .. } | WindowEvent::MouseInput { .. } => {
    //                     log::debug!("{:?}", window_event);
    //                 }
    //                 _ => {}
    //             }
    //         },
    //         Event::RedrawRequested(_) => {
    //             i += 1;
    //             ctx.gl_clear_color((i as f32 / 50.0).sin() * 0.5 + 0.5, 0.0, 1.0, 1.0);
    //             ctx.gl_clear(crate::gles20::COLOR_BUFFER_BIT);
    //         }
    //         _ => (),
    //     }
    // });
}
