use super::ButtonState;
use super::KeyboardKey;

use winit::event as we;
use winit::event::{DeviceEvent, VirtualKeyCode};

impl From<we::VirtualKeyCode> for KeyboardKey {
    fn from(kc: VirtualKeyCode) -> Self {
        match kc {
            VirtualKeyCode::Key1 => KeyboardKey::Key1,
            VirtualKeyCode::Key2 => KeyboardKey::Key2,
            VirtualKeyCode::Key3 => KeyboardKey::Key3,
            VirtualKeyCode::Key4 => KeyboardKey::Key4,
            VirtualKeyCode::Key5 => KeyboardKey::Key5,
            VirtualKeyCode::Key6 => KeyboardKey::Key6,
            VirtualKeyCode::Key7 => KeyboardKey::Key7,
            VirtualKeyCode::Key8 => KeyboardKey::Key8,
            VirtualKeyCode::Key9 => KeyboardKey::Key9,
            VirtualKeyCode::Key0 => KeyboardKey::Key0,
            VirtualKeyCode::A => KeyboardKey::A,
            VirtualKeyCode::B => KeyboardKey::B,
            VirtualKeyCode::C => KeyboardKey::C,
            VirtualKeyCode::D => KeyboardKey::D,
            VirtualKeyCode::E => KeyboardKey::E,
            VirtualKeyCode::F => KeyboardKey::F,
            VirtualKeyCode::G => KeyboardKey::G,
            VirtualKeyCode::H => KeyboardKey::H,
            VirtualKeyCode::I => KeyboardKey::I,
            VirtualKeyCode::J => KeyboardKey::J,
            VirtualKeyCode::K => KeyboardKey::K,
            VirtualKeyCode::L => KeyboardKey::L,
            VirtualKeyCode::M => KeyboardKey::M,
            VirtualKeyCode::N => KeyboardKey::N,
            VirtualKeyCode::O => KeyboardKey::O,
            VirtualKeyCode::P => KeyboardKey::P,
            VirtualKeyCode::Q => KeyboardKey::Q,
            VirtualKeyCode::R => KeyboardKey::R,
            VirtualKeyCode::S => KeyboardKey::S,
            VirtualKeyCode::T => KeyboardKey::T,
            VirtualKeyCode::U => KeyboardKey::U,
            VirtualKeyCode::V => KeyboardKey::V,
            VirtualKeyCode::W => KeyboardKey::W,
            VirtualKeyCode::X => KeyboardKey::X,
            VirtualKeyCode::Y => KeyboardKey::Y,
            VirtualKeyCode::Z => KeyboardKey::Z,
            VirtualKeyCode::Escape => KeyboardKey::Escape,
            VirtualKeyCode::F1 => KeyboardKey::F1,
            VirtualKeyCode::F2 => KeyboardKey::F2,
            VirtualKeyCode::F3 => KeyboardKey::F3,
            VirtualKeyCode::F4 => KeyboardKey::F4,
            VirtualKeyCode::F5 => KeyboardKey::F5,
            VirtualKeyCode::F6 => KeyboardKey::F6,
            VirtualKeyCode::F7 => KeyboardKey::F7,
            VirtualKeyCode::F8 => KeyboardKey::F8,
            VirtualKeyCode::F9 => KeyboardKey::F9,
            VirtualKeyCode::F10 => KeyboardKey::F10,
            VirtualKeyCode::F11 => KeyboardKey::F11,
            VirtualKeyCode::F12 => KeyboardKey::F12,
            VirtualKeyCode::F13 => KeyboardKey::F13,
            VirtualKeyCode::F14 => KeyboardKey::F14,
            VirtualKeyCode::F15 => KeyboardKey::F15,
            VirtualKeyCode::F16 => KeyboardKey::F16,
            VirtualKeyCode::F17 => KeyboardKey::F17,
            VirtualKeyCode::F18 => KeyboardKey::F18,
            VirtualKeyCode::F19 => KeyboardKey::F19,
            VirtualKeyCode::F20 => KeyboardKey::F20,
            VirtualKeyCode::F21 => KeyboardKey::F21,
            VirtualKeyCode::F22 => KeyboardKey::F22,
            VirtualKeyCode::F23 => KeyboardKey::F23,
            VirtualKeyCode::F24 => KeyboardKey::F24,
            VirtualKeyCode::Snapshot => KeyboardKey::Snapshot,
            VirtualKeyCode::Scroll => KeyboardKey::Scroll,
            VirtualKeyCode::Pause => KeyboardKey::Pause,
            VirtualKeyCode::Insert => KeyboardKey::Insert,
            VirtualKeyCode::Home => KeyboardKey::Home,
            VirtualKeyCode::Delete => KeyboardKey::Delete,
            VirtualKeyCode::End => KeyboardKey::End,
            VirtualKeyCode::PageDown => KeyboardKey::PageDown,
            VirtualKeyCode::PageUp => KeyboardKey::PageUp,
            VirtualKeyCode::Left => KeyboardKey::Left,
            VirtualKeyCode::Up => KeyboardKey::Up,
            VirtualKeyCode::Right => KeyboardKey::Right,
            VirtualKeyCode::Down => KeyboardKey::Down,
            VirtualKeyCode::Back => KeyboardKey::Back,
            VirtualKeyCode::Return => KeyboardKey::Return,
            VirtualKeyCode::Space => KeyboardKey::Space,
            VirtualKeyCode::Compose => KeyboardKey::Compose,
            VirtualKeyCode::Caret => KeyboardKey::Caret,
            VirtualKeyCode::Numlock => KeyboardKey::Numlock,
            VirtualKeyCode::Numpad0 => KeyboardKey::Numpad0,
            VirtualKeyCode::Numpad1 => KeyboardKey::Numpad1,
            VirtualKeyCode::Numpad2 => KeyboardKey::Numpad2,
            VirtualKeyCode::Numpad3 => KeyboardKey::Numpad3,
            VirtualKeyCode::Numpad4 => KeyboardKey::Numpad4,
            VirtualKeyCode::Numpad5 => KeyboardKey::Numpad5,
            VirtualKeyCode::Numpad6 => KeyboardKey::Numpad6,
            VirtualKeyCode::Numpad7 => KeyboardKey::Numpad7,
            VirtualKeyCode::Numpad8 => KeyboardKey::Numpad8,
            VirtualKeyCode::Numpad9 => KeyboardKey::Numpad9,
            VirtualKeyCode::NumpadAdd => KeyboardKey::NumpadAdd,
            VirtualKeyCode::NumpadDivide => KeyboardKey::NumpadDivide,
            VirtualKeyCode::NumpadDecimal => KeyboardKey::NumpadDecimal,
            VirtualKeyCode::NumpadComma => KeyboardKey::NumpadComma,
            VirtualKeyCode::NumpadEnter => KeyboardKey::NumpadEnter,
            VirtualKeyCode::NumpadEquals => KeyboardKey::NumpadEquals,
            VirtualKeyCode::NumpadMultiply => KeyboardKey::NumpadMultiply,
            VirtualKeyCode::NumpadSubtract => KeyboardKey::NumpadSubtract,
            VirtualKeyCode::AbntC1 => KeyboardKey::AbntC1,
            VirtualKeyCode::AbntC2 => KeyboardKey::AbntC2,
            VirtualKeyCode::Apostrophe => KeyboardKey::Apostrophe,
            VirtualKeyCode::Apps => KeyboardKey::Apps,
            VirtualKeyCode::Asterisk => KeyboardKey::Asterisk,
            VirtualKeyCode::At => KeyboardKey::At,
            VirtualKeyCode::Ax => KeyboardKey::Ax,
            VirtualKeyCode::Backslash => KeyboardKey::Backslash,
            VirtualKeyCode::Calculator => KeyboardKey::Calculator,
            VirtualKeyCode::Capital => KeyboardKey::Capital,
            VirtualKeyCode::Colon => KeyboardKey::Colon,
            VirtualKeyCode::Comma => KeyboardKey::Comma,
            VirtualKeyCode::Convert => KeyboardKey::Convert,
            VirtualKeyCode::Equals => KeyboardKey::Equals,
            VirtualKeyCode::Grave => KeyboardKey::Grave,
            VirtualKeyCode::Kana => KeyboardKey::Kana,
            VirtualKeyCode::Kanji => KeyboardKey::Kanji,
            VirtualKeyCode::LAlt => KeyboardKey::LAlt,
            VirtualKeyCode::LBracket => KeyboardKey::LBracket,
            VirtualKeyCode::LControl => KeyboardKey::LControl,
            VirtualKeyCode::LShift => KeyboardKey::LShift,
            VirtualKeyCode::LWin => KeyboardKey::LWin,
            VirtualKeyCode::Mail => KeyboardKey::Mail,
            VirtualKeyCode::MediaSelect => KeyboardKey::MediaSelect,
            VirtualKeyCode::MediaStop => KeyboardKey::MediaStop,
            VirtualKeyCode::Minus => KeyboardKey::Minus,
            VirtualKeyCode::Mute => KeyboardKey::Mute,
            VirtualKeyCode::MyComputer => KeyboardKey::MyComputer,
            VirtualKeyCode::NavigateForward => KeyboardKey::NavigateForward,
            VirtualKeyCode::NavigateBackward => KeyboardKey::NavigateBackward,
            VirtualKeyCode::NextTrack => KeyboardKey::NextTrack,
            VirtualKeyCode::NoConvert => KeyboardKey::NoConvert,
            VirtualKeyCode::OEM102 => KeyboardKey::OEM102,
            VirtualKeyCode::Period => KeyboardKey::Period,
            VirtualKeyCode::PlayPause => KeyboardKey::PlayPause,
            VirtualKeyCode::Plus => KeyboardKey::Plus,
            VirtualKeyCode::Power => KeyboardKey::Power,
            VirtualKeyCode::PrevTrack => KeyboardKey::PrevTrack,
            VirtualKeyCode::RAlt => KeyboardKey::RAlt,
            VirtualKeyCode::RBracket => KeyboardKey::RBracket,
            VirtualKeyCode::RControl => KeyboardKey::RControl,
            VirtualKeyCode::RShift => KeyboardKey::RShift,
            VirtualKeyCode::RWin => KeyboardKey::RWin,
            VirtualKeyCode::Semicolon => KeyboardKey::Semicolon,
            VirtualKeyCode::Slash => KeyboardKey::Slash,
            VirtualKeyCode::Sleep => KeyboardKey::Sleep,
            VirtualKeyCode::Stop => KeyboardKey::Stop,
            VirtualKeyCode::Sysrq => KeyboardKey::Sysrq,
            VirtualKeyCode::Tab => KeyboardKey::Tab,
            VirtualKeyCode::Underline => KeyboardKey::Underline,
            VirtualKeyCode::Unlabeled => KeyboardKey::Unlabeled,
            VirtualKeyCode::VolumeDown => KeyboardKey::VolumeDown,
            VirtualKeyCode::VolumeUp => KeyboardKey::VolumeUp,
            VirtualKeyCode::Wake => KeyboardKey::Wake,
            VirtualKeyCode::WebBack => KeyboardKey::WebBack,
            VirtualKeyCode::WebFavorites => KeyboardKey::WebFavorites,
            VirtualKeyCode::WebForward => KeyboardKey::WebForward,
            VirtualKeyCode::WebHome => KeyboardKey::WebHome,
            VirtualKeyCode::WebRefresh => KeyboardKey::WebRefresh,
            VirtualKeyCode::WebSearch => KeyboardKey::WebSearch,
            VirtualKeyCode::WebStop => KeyboardKey::WebStop,
            VirtualKeyCode::Yen => KeyboardKey::Yen,
            VirtualKeyCode::Copy => KeyboardKey::Copy,
            VirtualKeyCode::Paste => KeyboardKey::Paste,
            VirtualKeyCode::Cut => KeyboardKey::Cut,
        }
    }
}

#[derive(Copy, Clone)]
pub struct WinitElementState {
    element_state: we::ElementState,
}

impl WinitElementState {
    pub fn new(element_state: we::ElementState) -> Self {
        WinitElementState { element_state }
    }
}

impl Into<ButtonState> for WinitElementState {
    fn into(self) -> ButtonState {
        match self.element_state {
            we::ElementState::Pressed => ButtonState::Pressed,
            we::ElementState::Released => ButtonState::Released,
        }
    }
}

#[derive(Copy, Clone)]
pub struct WinitMouseButton {
    mouse_button: we::MouseButton,
}

impl WinitMouseButton {
    pub fn new(mouse_button: we::MouseButton) -> Self {
        WinitMouseButton { mouse_button }
    }
}

impl Into<super::MouseButton> for WinitMouseButton {
    fn into(self) -> super::MouseButton {
        let button_index = match self.mouse_button {
            we::MouseButton::Left => 0,
            we::MouseButton::Right => 1,
            we::MouseButton::Middle => 2,
            we::MouseButton::Other(x) => x + 3,
        };

        super::MouseButton(button_index)
    }
}

#[derive(Copy, Clone)]
pub struct WinitMouseScrollDelta {
    mouse_scroll_delta: we::MouseScrollDelta,
}

impl WinitMouseScrollDelta {
    pub fn new(mouse_scroll_delta: we::MouseScrollDelta) -> Self {
        WinitMouseScrollDelta { mouse_scroll_delta }
    }
}

impl Into<super::MouseScrollDelta> for WinitMouseScrollDelta {
    fn into(self) -> super::MouseScrollDelta {
        let delta = match self.mouse_scroll_delta {
            we::MouseScrollDelta::LineDelta(x, y) => (x, y),
            we::MouseScrollDelta::PixelDelta(delta) => (delta.x as f32, delta.y as f32),
        };

        super::MouseScrollDelta {
            x: delta.0,
            y: delta.1,
        }
    }
}

/// Call when winit sends an event
pub fn handle_winit_event<T>(
    event: &winit::event::Event<T>,
    input_state: &mut super::InputState,
) {
    use winit::event::Event;
    use winit::event::WindowEvent;

    let _is_close_requested = false;

    match event {
        //Process keyboard input
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            log::trace!("keyboard input {:?}", input);
            if let Some(vk) = input.virtual_keycode {
                input_state
                    .handle_keyboard_event(vk.into(), WinitElementState::new(input.state).into());
            }
        }

        Event::WindowEvent {
            event:
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                    ..
                },
            ..
        } => {
            log::trace!(
                "mouse button input {:?} {:?} {:?}",
                device_id,
                state,
                button,
            );

            input_state.handle_mouse_button_event(
                WinitMouseButton::new(*button).into(),
                WinitElementState::new(*state).into(),
            );
        }

        Event::WindowEvent {
            event:
                WindowEvent::CursorMoved {
                    device_id,
                    position,
                    ..
                },
            ..
        } => {
            log::trace!("mouse move input {:?} {:?}", device_id, position);
            input_state.handle_mouse_update_position(glam::Vec2::new(
                position.x as f32,
                position.y as f32,
            ));
        }

        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            device_id,
        } => {
            log::trace!("mouse motion input {:?} {:?}", device_id, delta);
            input_state.handle_mouse_motion_event(glam::Vec2::new(delta.0 as f32, delta.1 as f32));
        }

        Event::WindowEvent {
            event: WindowEvent::MouseWheel {
                device_id, delta, ..
            },
            ..
        } => {
            log::trace!("mouse wheel {:?} {:?}", device_id, delta);
            input_state.handle_mouse_wheel_event(WinitMouseScrollDelta::new(*delta).into());
        }

        // Ignore any other events
        _ => (),
    }
}
