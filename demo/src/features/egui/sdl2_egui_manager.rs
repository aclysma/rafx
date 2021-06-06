use std::sync::Arc;
use std::sync::Mutex;

use super::EguiManager;
use rafx::api::RafxResult;
use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::video::Window;

struct CursorHandler {
    system_cursor: Option<sdl2::mouse::SystemCursor>,
    cursor: Option<sdl2::mouse::Cursor>,
    mouse: sdl2::mouse::MouseUtil,
}

impl CursorHandler {
    fn new(mouse: sdl2::mouse::MouseUtil) -> Self {
        CursorHandler {
            system_cursor: None,
            cursor: None,
            mouse,
        }
    }

    fn set_cursor(
        &mut self,
        system_cursor: Option<sdl2::mouse::SystemCursor>,
    ) {
        if system_cursor != self.system_cursor {
            if system_cursor.is_none() {
                self.system_cursor = None;
                self.cursor = None;
                self.mouse.show_cursor(false);
            } else {
                let cursor = sdl2::mouse::Cursor::from_system(system_cursor.unwrap()).unwrap();
                cursor.set();

                self.system_cursor = system_cursor;
                self.cursor = Some(cursor);
                self.mouse.show_cursor(true);
            }
        }
    }
}

struct Sdl2EguiManagerInner {
    clipboard: sdl2::clipboard::ClipboardUtil,
    video_subsystem: sdl2::VideoSubsystem,
    cursor: CursorHandler,
}

// For sdl2::mouse::Cursor, a member of egui_sdl2::Sdl2EguiManager
unsafe impl Send for Sdl2EguiManagerInner {}

/// Full egui API and the SDL2 abstraction/platform integration
#[derive(Clone)]
pub struct Sdl2EguiManager {
    egui_manager: EguiManager,
    inner: Arc<Mutex<Sdl2EguiManagerInner>>,
}

// Wraps egui (and winit integration logic)
impl Sdl2EguiManager {
    pub fn egui_manager(&self) -> EguiManager {
        self.egui_manager.clone()
    }

    // egui and winit platform are expected to be pre-configured
    pub fn new(
        sdl2_video_subsystem: &sdl2::VideoSubsystem,
        sdl2_mouse: sdl2::mouse::MouseUtil,
    ) -> Self {
        let egui_manager = EguiManager::new();

        let inner = Sdl2EguiManagerInner {
            clipboard: sdl2_video_subsystem.clipboard(),
            video_subsystem: sdl2_video_subsystem.clone(),
            cursor: CursorHandler::new(sdl2_mouse),
        };

        Sdl2EguiManager {
            egui_manager,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    // Call when a window event is received
    //TODO: Taking a lock per event sucks
    #[profiling::function]
    pub fn handle_event(
        &self,
        event: &sdl2::event::Event,
    ) {
        self.egui_manager.with_context_and_input(|_, input| {
            match event {
                Event::KeyDown {
                    keycode, keymod, ..
                } => {
                    Self::handle_key_press(
                        input,
                        *keycode,
                        *keymod,
                        &self.inner.lock().unwrap().clipboard,
                        true,
                    );
                }
                Event::KeyUp {
                    keycode, keymod, ..
                } => {
                    Self::handle_key_press(
                        input,
                        *keycode,
                        *keymod,
                        &self.inner.lock().unwrap().clipboard,
                        false,
                    );
                }
                Event::TextInput { text, .. } => {
                    input.events.push(egui::Event::Text(text.clone()));
                }
                Event::MouseMotion { x, y, .. } => {
                    let dpi = input.pixels_per_point.unwrap_or(1.0);
                    input.events.push(egui::Event::PointerMoved(egui::Pos2::new(
                        *x as f32 / dpi,
                        *y as f32 / dpi,
                    )));
                }
                Event::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    Self::handle_mouse_press(input, *x, *y, *mouse_btn, true);
                }
                Event::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    Self::handle_mouse_press(input, *x, *y, *mouse_btn, false);
                }
                Event::MouseWheel { x, y, .. } => {
                    // hook up to zoom if ctrl held?
                    input.scroll_delta.x += *x as f32;
                    input.scroll_delta.y += *y as f32;
                }
                //Event::FingerDown { .. } => {}
                //Event::FingerUp { .. } => {}
                //Event::FingerMotion { .. } => {}
                _ => {}
            }
        });
    }

    fn handle_key_press(
        input: &mut egui::RawInput,
        keycode: Option<sdl2::keyboard::Keycode>,
        keymod: sdl2::keyboard::Mod,
        clipboard: &sdl2::clipboard::ClipboardUtil,
        pressed: bool,
    ) {
        use sdl2::keyboard::Mod;

        input.modifiers.alt = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
        input.modifiers.ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
        input.modifiers.shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
        if cfg!(target_os = "macos") {
            input.modifiers.mac_cmd = keymod.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
            input.modifiers.command = input.modifiers.mac_cmd;
        } else {
            input.modifiers.mac_cmd = false;
            input.modifiers.command = input.modifiers.ctrl;
        }

        if let Some(sdl2_keycode) = keycode {
            if let Some(key) = Self::egui_key(sdl2_keycode) {
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
                                if let Ok(text) = clipboard.clipboard_text() {
                                    input.events.push(egui::Event::Text(text));
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
        x: i32,
        y: i32,
        mouse_btn: sdl2::mouse::MouseButton,
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
        event: &sdl2::event::Event,
    ) -> bool {
        let mut ignore = false;
        self.egui_manager.with_context(|ctx| {
            ignore = match event {
                Event::KeyDown { .. } => ctx.wants_keyboard_input(),
                Event::KeyUp { .. } => ctx.wants_keyboard_input(),
                Event::TextInput { .. } => ctx.wants_keyboard_input(),
                Event::MouseMotion { .. } => ctx.wants_pointer_input(),
                Event::MouseButtonDown { .. } => ctx.wants_pointer_input(),
                Event::MouseButtonUp { .. } => ctx.wants_pointer_input(),
                Event::MouseWheel { .. } => ctx.wants_pointer_input(),
                //Event::FingerDown { .. } => {}
                //Event::FingerUp { .. } => {}
                //Event::FingerMotion { .. } => {}
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
        let (physical_width, physical_height) = window.vulkan_drawable_size();

        let pixels_per_point = if cfg!(target_os = "windows") {
            let display_index = window.display_index()?;
            let (_, display_dpi, _) = window.subsystem().display_dpi(display_index)?;
            display_dpi / 96.0
        } else {
            let (logical_width, _logical_height) = window.size();
            if logical_width > 0 {
                physical_width as f32 / logical_width as f32
            } else {
                1.0
            }
        };

        self.egui_manager
            .begin_frame(physical_width, physical_height, pixels_per_point);
        Ok(())
    }

    // Finishes the frame. Draw data becomes available via get_draw_data()
    #[profiling::function]
    pub fn end_frame(&self) {
        let mut inner = self.inner.lock().unwrap();

        let output = self.egui_manager.end_frame();
        if !output.copied_text.is_empty() {
            inner
                .clipboard
                .set_clipboard_text(&output.copied_text)
                .unwrap();
        }

        let cursor = Self::sdl2_mouse_cursor(output.cursor_icon);
        inner.cursor.set_cursor(cursor);
    }

    fn egui_mouse_button(mouse_button: sdl2::mouse::MouseButton) -> Option<egui::PointerButton> {
        match mouse_button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            _ => None,
        }
    }

    fn egui_key(key: sdl2::keyboard::Keycode) -> Option<egui::Key> {
        use egui::Key;
        use sdl2::keyboard::Keycode;

        Some(match key {
            Keycode::Down => Key::ArrowDown,
            Keycode::Left => Key::ArrowLeft,
            Keycode::Right => Key::ArrowRight,
            Keycode::Up => Key::ArrowUp,

            Keycode::Escape => Key::Escape,
            Keycode::Tab => Key::Tab,
            Keycode::Backspace => Key::Backspace,
            Keycode::Return => Key::Enter,
            Keycode::Space => Key::Space,

            Keycode::Insert => Key::Insert,
            Keycode::Delete => Key::Delete,
            Keycode::Home => Key::Home,
            Keycode::End => Key::End,
            Keycode::PageUp => Key::PageUp,
            Keycode::PageDown => Key::PageDown,

            Keycode::Num0 | Keycode::Kp0 => Key::Num0,
            Keycode::Num1 | Keycode::Kp1 => Key::Num1,
            Keycode::Num2 | Keycode::Kp2 => Key::Num2,
            Keycode::Num3 | Keycode::Kp3 => Key::Num3,
            Keycode::Num4 | Keycode::Kp4 => Key::Num4,
            Keycode::Num5 | Keycode::Kp5 => Key::Num5,
            Keycode::Num6 | Keycode::Kp6 => Key::Num6,
            Keycode::Num7 | Keycode::Kp7 => Key::Num7,
            Keycode::Num8 | Keycode::Kp8 => Key::Num8,
            Keycode::Num9 | Keycode::Kp9 => Key::Num9,

            Keycode::A => Key::A,
            Keycode::B => Key::B,
            Keycode::C => Key::C,
            Keycode::D => Key::D,
            Keycode::E => Key::E,
            Keycode::F => Key::F,
            Keycode::G => Key::G,
            Keycode::H => Key::H,
            Keycode::I => Key::I,
            Keycode::J => Key::J,
            Keycode::K => Key::K,
            Keycode::L => Key::L,
            Keycode::M => Key::M,
            Keycode::N => Key::N,
            Keycode::O => Key::O,
            Keycode::P => Key::P,
            Keycode::Q => Key::Q,
            Keycode::R => Key::R,
            Keycode::S => Key::S,
            Keycode::T => Key::T,
            Keycode::U => Key::U,
            Keycode::V => Key::V,
            Keycode::W => Key::W,
            Keycode::X => Key::X,
            Keycode::Y => Key::Y,
            Keycode::Z => Key::Z,
            _ => return None,
        })
    }

    fn sdl2_mouse_cursor(egui_cursor: egui::CursorIcon) -> Option<sdl2::mouse::SystemCursor> {
        use egui::CursorIcon;
        use sdl2::mouse::SystemCursor;

        Some(match egui_cursor {
            CursorIcon::None => return None,
            CursorIcon::PointingHand => SystemCursor::Hand,
            CursorIcon::Progress => SystemCursor::Wait,
            CursorIcon::Wait => SystemCursor::Wait,
            CursorIcon::Crosshair => SystemCursor::Crosshair,
            CursorIcon::Text => SystemCursor::IBeam,
            CursorIcon::VerticalText => SystemCursor::IBeam,
            CursorIcon::NoDrop => SystemCursor::No,
            CursorIcon::NotAllowed => SystemCursor::No,
            CursorIcon::Grab => SystemCursor::Hand,
            CursorIcon::Grabbing => SystemCursor::Hand,
            CursorIcon::ResizeHorizontal => SystemCursor::SizeWE,
            CursorIcon::ResizeNeSw => SystemCursor::SizeNESW,
            CursorIcon::ResizeNwSe => SystemCursor::SizeNWSE,
            CursorIcon::ResizeVertical => SystemCursor::SizeNS,
            _ => SystemCursor::Arrow,
        })
    }
}
