use std::sync::Arc;
use std::sync::Mutex;

use super::EguiManager;
use copypasta::{ClipboardContext, ClipboardProvider};
use rafx::api::RafxResult;
use winit::event::MouseButton;
use winit::event::{Event, MouseScrollDelta, WindowEvent};
use winit::window::Window;

struct WinitEguiManagerInner {
    clipboard: Option<ClipboardContext>,
    mouse_position: Option<egui::Pos2>,
    cursor: Option<winit::window::CursorIcon>,
    pending_cursor: Option<winit::window::CursorIcon>,
}

/// Full egui API and the SDL2 abstraction/platform integration
#[derive(Clone)]
pub struct WinitEguiManager {
    egui_manager: EguiManager,
    inner: Arc<Mutex<WinitEguiManagerInner>>,
}

// Wraps egui (and winit integration logic)
impl WinitEguiManager {
    pub fn egui_manager(&self) -> EguiManager {
        self.egui_manager.clone()
    }

    // egui and winit platform are expected to be pre-configured
    pub fn new() -> Self {
        let egui_manager = EguiManager::new();

        let inner = WinitEguiManagerInner {
            clipboard: ClipboardContext::new().ok(),
            mouse_position: Default::default(),
            cursor: None,
            pending_cursor: None,
        };

        if inner.clipboard.is_none() {
            log::warn!("Clipboard could not be initialized");
        }

        WinitEguiManager {
            egui_manager,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    // Call when a window event is received
    //TODO: Taking a lock per event sucks
    #[profiling::function]
    pub fn handle_event(
        &self,
        event: &winit::event::Event<()>,
    ) {
        self.egui_manager.with_context_and_input(|_, input| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            input.pixels_per_point = Some(*scale_factor as f32);
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            let mouse_position = self.inner.lock().unwrap().mouse_position;
                            if let Some(mouse_position) = mouse_position {
                                let pressed = *state == winit::event::ElementState::Pressed;
                                Self::handle_mouse_press(
                                    input,
                                    mouse_position.x,
                                    mouse_position.y,
                                    *button,
                                    pressed,
                                );
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let mouse_position =
                                position.to_logical(input.pixels_per_point.unwrap_or(1.0) as f64);
                            let mut inner = self.inner.lock().unwrap();
                            let position = egui::pos2(mouse_position.x, mouse_position.y);
                            inner.mouse_position = Some(position);
                            input.events.push(egui::Event::PointerMoved(position));
                        }
                        WindowEvent::CursorLeft { .. } => {
                            let mut inner = self.inner.lock().unwrap();
                            inner.mouse_position = None;
                            input.events.push(egui::Event::PointerGone);
                        }
                        WindowEvent::ReceivedCharacter(c) => {
                            if Self::is_printable_char(*c)
                                && !input.modifiers.ctrl
                                && !input.modifiers.mac_cmd
                            {
                                input.events.push(egui::Event::Text(c.to_string()));
                            }
                        }
                        WindowEvent::KeyboardInput {
                            input: keyboard_input,
                            ..
                        } => {
                            let mut inner = self.inner.lock().unwrap();
                            let pressed =
                                keyboard_input.state == winit::event::ElementState::Pressed;
                            Self::handle_key_press(
                                input,
                                keyboard_input.virtual_keycode,
                                &mut inner.clipboard,
                                pressed,
                            );
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            let mut delta = match delta {
                                MouseScrollDelta::LineDelta(x, y) => {
                                    // from egui_glium
                                    let line_height = 8.0;
                                    egui::vec2(*x, *y) * line_height
                                }
                                MouseScrollDelta::PixelDelta(delta) => {
                                    egui::vec2(delta.x as f32, delta.y as f32)
                                        / input.pixels_per_point.unwrap_or(1.0)
                                }
                            };

                            // from egui_glium
                            if cfg!(target_os = "macos") {
                                delta.x *= -1.0;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        });
    }

    fn handle_key_press(
        input: &mut egui::RawInput,
        keycode: Option<winit::event::VirtualKeyCode>,
        clipboard: &mut Option<copypasta::ClipboardContext>,
        pressed: bool,
    ) {
        use winit::event::VirtualKeyCode;

        if let Some(keycode) = keycode {
            if matches!(keycode, VirtualKeyCode::LAlt | VirtualKeyCode::RAlt) {
                input.modifiers.alt = pressed;
            }
            if matches!(keycode, VirtualKeyCode::LControl | VirtualKeyCode::RControl) {
                input.modifiers.ctrl = pressed;
            }
            if matches!(keycode, VirtualKeyCode::LShift | VirtualKeyCode::RShift) {
                input.modifiers.shift = pressed;
            }

            if cfg!(target_os = "macos") {
                if matches!(keycode, VirtualKeyCode::LWin | VirtualKeyCode::RWin) {
                    input.modifiers.mac_cmd = pressed;
                    input.modifiers.command = pressed;
                }
            } else {
                input.modifiers.mac_cmd = false;
                input.modifiers.command = input.modifiers.ctrl;
            }

            if let Some(key) = Self::egui_key(keycode) {
                // intercept cut/copy/paste
                if pressed {
                    if input.modifiers.command {
                        match key {
                            egui::Key::X => {
                                input.events.push(egui::Event::Cut);
                            }
                            egui::Key::C => {
                                input.events.push(egui::Event::Copy);
                            }
                            egui::Key::V => {
                                if let Some(clipboard) = clipboard {
                                    if let Ok(text) = clipboard.get_contents() {
                                        input.events.push(egui::Event::Text(text));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                input.events.push(egui::Event::Key {
                    key,
                    pressed,
                    modifiers: input.modifiers,
                });
            }
        }
    }

    fn handle_mouse_press(
        input: &mut egui::RawInput,
        x: f32,
        y: f32,
        mouse_btn: winit::event::MouseButton,
        pressed: bool,
    ) {
        if let Some(button) = Self::egui_mouse_button(mouse_btn) {
            input.events.push(egui::Event::PointerButton {
                pos: egui::Pos2::new(x as _, y as _),
                button,
                pressed,
                modifiers: input.modifiers,
            });
        }
    }

    pub fn ignore_event(
        &self,
        event: &Event<()>,
    ) -> bool {
        let mut ignore = false;
        self.egui_manager.with_context(|ctx| {
            ignore = match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::ReceivedCharacter(_) => ctx.wants_keyboard_input(),
                    WindowEvent::KeyboardInput { .. } => ctx.wants_keyboard_input(),
                    WindowEvent::MouseInput { .. } => ctx.wants_pointer_input(),
                    WindowEvent::CursorMoved { .. } => ctx.wants_pointer_input(),
                    WindowEvent::MouseWheel { .. } => ctx.wants_pointer_input(),
                    _ => false,
                },
                _ => false,
            };
        });

        ignore
    }

    // Start a new frame
    #[profiling::function]
    pub fn begin_frame(
        &self,
        window: &Window,
    ) -> RafxResult<()> {
        // raw pixels
        let physical_size = window.inner_size();
        let pixels_per_point = window.scale_factor() as f32;

        let mut inner = self.inner.lock().unwrap();
        if inner.cursor != inner.pending_cursor || inner.cursor.is_none() {
            if let Some(pending_cursor) = inner.pending_cursor {
                window.set_cursor_visible(true);
                window.set_cursor_icon(pending_cursor);
            } else {
                window.set_cursor_visible(false);
                window.set_cursor_icon(winit::window::CursorIcon::Default);
            }

            inner.cursor = inner.pending_cursor;
        }

        self.egui_manager
            .begin_frame(physical_size.width, physical_size.height, pixels_per_point);
        Ok(())
    }

    // Finishes the frame. Draw data becomes available via get_draw_data()
    #[profiling::function]
    pub fn end_frame(&self) {
        let mut inner = self.inner.lock().unwrap();

        let output = self.egui_manager.end_frame();
        if !output.copied_text.is_empty() {
            if let Some(clipboard) = &mut inner.clipboard {
                clipboard.set_contents(output.copied_text).unwrap();
            }
        }

        inner.pending_cursor = Self::winit_mouse_cursor(output.cursor_icon);
    }

    fn egui_mouse_button(mouse_button: winit::event::MouseButton) -> Option<egui::PointerButton> {
        match mouse_button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            _ => None,
        }
    }

    fn egui_key(key: winit::event::VirtualKeyCode) -> Option<egui::Key> {
        use egui::Key;
        use winit::event::VirtualKeyCode;

        Some(match key {
            VirtualKeyCode::Down => Key::ArrowDown,
            VirtualKeyCode::Left => Key::ArrowLeft,
            VirtualKeyCode::Right => Key::ArrowRight,
            VirtualKeyCode::Up => Key::ArrowUp,

            VirtualKeyCode::Escape => Key::Escape,
            VirtualKeyCode::Tab => Key::Tab,
            VirtualKeyCode::Back => Key::Backspace,
            VirtualKeyCode::Return => Key::Enter,
            VirtualKeyCode::Space => Key::Space,

            VirtualKeyCode::Insert => Key::Insert,
            VirtualKeyCode::Delete => Key::Delete,
            VirtualKeyCode::Home => Key::Home,
            VirtualKeyCode::End => Key::End,
            VirtualKeyCode::PageUp => Key::PageUp,
            VirtualKeyCode::PageDown => Key::PageDown,

            VirtualKeyCode::Numpad0 | VirtualKeyCode::Key0 => Key::Num0,
            VirtualKeyCode::Numpad1 | VirtualKeyCode::Key1 => Key::Num1,
            VirtualKeyCode::Numpad2 | VirtualKeyCode::Key2 => Key::Num2,
            VirtualKeyCode::Numpad3 | VirtualKeyCode::Key3 => Key::Num3,
            VirtualKeyCode::Numpad4 | VirtualKeyCode::Key4 => Key::Num4,
            VirtualKeyCode::Numpad5 | VirtualKeyCode::Key5 => Key::Num5,
            VirtualKeyCode::Numpad6 | VirtualKeyCode::Key6 => Key::Num6,
            VirtualKeyCode::Numpad7 | VirtualKeyCode::Key7 => Key::Num7,
            VirtualKeyCode::Numpad8 | VirtualKeyCode::Key8 => Key::Num8,
            VirtualKeyCode::Numpad9 | VirtualKeyCode::Key9 => Key::Num9,

            VirtualKeyCode::A => Key::A,
            VirtualKeyCode::B => Key::B,
            VirtualKeyCode::C => Key::C,
            VirtualKeyCode::D => Key::D,
            VirtualKeyCode::E => Key::E,
            VirtualKeyCode::F => Key::F,
            VirtualKeyCode::G => Key::G,
            VirtualKeyCode::H => Key::H,
            VirtualKeyCode::I => Key::I,
            VirtualKeyCode::J => Key::J,
            VirtualKeyCode::K => Key::K,
            VirtualKeyCode::L => Key::L,
            VirtualKeyCode::M => Key::M,
            VirtualKeyCode::N => Key::N,
            VirtualKeyCode::O => Key::O,
            VirtualKeyCode::P => Key::P,
            VirtualKeyCode::Q => Key::Q,
            VirtualKeyCode::R => Key::R,
            VirtualKeyCode::S => Key::S,
            VirtualKeyCode::T => Key::T,
            VirtualKeyCode::U => Key::U,
            VirtualKeyCode::V => Key::V,
            VirtualKeyCode::W => Key::W,
            VirtualKeyCode::X => Key::X,
            VirtualKeyCode::Y => Key::Y,
            VirtualKeyCode::Z => Key::Z,
            _ => return None,
        })
    }

    fn winit_mouse_cursor(egui_cursor: egui::CursorIcon) -> Option<winit::window::CursorIcon> {
        use egui::CursorIcon as EguiCursorIcon;
        use winit::window::CursorIcon as WinitCursorIcon;

        Some(match egui_cursor {
            EguiCursorIcon::Default => WinitCursorIcon::Default,
            EguiCursorIcon::None => return None,
            EguiCursorIcon::ContextMenu => WinitCursorIcon::ContextMenu,
            EguiCursorIcon::Help => WinitCursorIcon::Help,
            EguiCursorIcon::PointingHand => WinitCursorIcon::Hand,
            EguiCursorIcon::Progress => WinitCursorIcon::Progress,
            EguiCursorIcon::Wait => WinitCursorIcon::Wait,
            EguiCursorIcon::Cell => WinitCursorIcon::Cell,
            EguiCursorIcon::Crosshair => WinitCursorIcon::Crosshair,
            EguiCursorIcon::Text => WinitCursorIcon::Text,
            EguiCursorIcon::VerticalText => WinitCursorIcon::VerticalText,
            EguiCursorIcon::Alias => WinitCursorIcon::Alias,
            EguiCursorIcon::Copy => WinitCursorIcon::Copy,
            EguiCursorIcon::Move => WinitCursorIcon::Move,
            EguiCursorIcon::NoDrop => WinitCursorIcon::NoDrop,
            EguiCursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
            EguiCursorIcon::Grab => WinitCursorIcon::Grab,
            EguiCursorIcon::Grabbing => WinitCursorIcon::Grabbing,
            EguiCursorIcon::AllScroll => WinitCursorIcon::AllScroll,
            EguiCursorIcon::ResizeHorizontal => WinitCursorIcon::EwResize,
            EguiCursorIcon::ResizeNeSw => WinitCursorIcon::NeswResize,
            EguiCursorIcon::ResizeNwSe => WinitCursorIcon::NwseResize,
            EguiCursorIcon::ResizeVertical => WinitCursorIcon::NsResize,
            EguiCursorIcon::ZoomIn => WinitCursorIcon::ZoomIn,
            EguiCursorIcon::ZoomOut => WinitCursorIcon::ZoomOut,
        })
    }

    // From egui_glium:
    /// Glium sends special keys (backspace, delete, F1, ...) as characters.
    /// Ignore those.
    /// We also ignore '\r', '\n', '\t'.
    /// Newlines are handled by the `Key::Enter` event.
    fn is_printable_char(chr: char) -> bool {
        let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
            || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
            || '\u{100000}' <= chr && chr <= '\u{10fffd}';

        !is_in_private_use_area && !chr.is_ascii_control()
    }
}
