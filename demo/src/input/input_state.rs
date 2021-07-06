// End-users should provide their own layer to translate from these general values to something
// appropriate to their platform or windowing system
// These match winit
#[derive(Copy, Clone, Debug)]
pub enum KeyboardKey {
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    Snapshot,
    /// Scroll Lock.
    Scroll,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    // TODO: rename
    Back,
    /// The Enter key.
    Return,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    // also called "Next"
    NavigateForward,
    // also called "Prior"
    NavigateBackward,
    NextTrack,
    NoConvert,
    OEM102,
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}

#[derive(Copy, Clone)]
pub struct MouseButton(pub u16);

impl MouseButton {
    pub const LEFT: MouseButton = MouseButton(0);
    pub const RIGHT: MouseButton = MouseButton(1);
    pub const MIDDLE: MouseButton = MouseButton(2);
}

#[derive(Copy, Clone)]
pub struct MouseScrollDelta {
    pub x: f32,
    pub y: f32,
}

impl MouseScrollDelta {
    pub fn new(
        x: f32,
        y: f32,
    ) -> Self {
        MouseScrollDelta { x, y }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ButtonState {
    Released,
    Pressed,
}

/// Encapsulates the state of a mouse drag
#[derive(Copy, Clone, Debug)]
pub struct MouseDragState {
    pub begin_position: glam::Vec2,
    pub end_position: glam::Vec2,
    pub previous_frame_delta: glam::Vec2,
    pub accumulated_frame_delta: glam::Vec2,
}

/// State of input devices. This is maintained by processing events from winit
pub struct InputState {
    key_is_down: [bool; Self::KEYBOARD_BUTTON_COUNT],
    key_just_down: [bool; Self::KEYBOARD_BUTTON_COUNT],
    key_just_up: [bool; Self::KEYBOARD_BUTTON_COUNT],

    mouse_position: glam::Vec2,
    mouse_motion: glam::Vec2,
    mouse_wheel_delta: MouseScrollDelta,
    mouse_button_is_down: [bool; Self::MOUSE_BUTTON_COUNT as usize],
    mouse_button_just_down: [Option<glam::Vec2>; Self::MOUSE_BUTTON_COUNT as usize],
    mouse_button_just_up: [Option<glam::Vec2>; Self::MOUSE_BUTTON_COUNT as usize],

    mouse_button_just_clicked: [Option<glam::Vec2>; Self::MOUSE_BUTTON_COUNT as usize],

    mouse_button_went_down_position: [Option<glam::Vec2>; Self::MOUSE_BUTTON_COUNT as usize],
    mouse_button_went_up_position: [Option<glam::Vec2>; Self::MOUSE_BUTTON_COUNT as usize],

    mouse_drag_in_progress: [Option<MouseDragState>; Self::MOUSE_BUTTON_COUNT as usize],
    mouse_drag_just_finished: [Option<MouseDragState>; Self::MOUSE_BUTTON_COUNT as usize],
}

impl InputState {
    /// Number of keyboard buttons we will track. Any button with a higher virtual key code will be
    /// ignored
    pub const KEYBOARD_BUTTON_COUNT: usize = 255;

    /// Number of mouse buttons we will track. Any button with a higher index will be ignored.
    pub const MOUSE_BUTTON_COUNT: u16 = 7;

    /// Distance in LogicalPosition units that the mouse has to be dragged to be considered a drag
    /// rather than a click
    const MIN_DRAG_DISTANCE: f32 = 2.0;
}

impl InputState {
    /// Create a new input state to track the given window
    pub fn new() -> InputState {
        InputState {
            key_is_down: [false; Self::KEYBOARD_BUTTON_COUNT],
            key_just_down: [false; Self::KEYBOARD_BUTTON_COUNT],
            key_just_up: [false; Self::KEYBOARD_BUTTON_COUNT],
            mouse_position: glam::Vec2::ZERO,
            mouse_motion: glam::Vec2::ZERO,
            mouse_wheel_delta: MouseScrollDelta { x: 0.0, y: 0.0 },
            mouse_button_is_down: [false; Self::MOUSE_BUTTON_COUNT as usize],
            mouse_button_just_down: [None; Self::MOUSE_BUTTON_COUNT as usize],
            mouse_button_just_up: [None; Self::MOUSE_BUTTON_COUNT as usize],
            mouse_button_just_clicked: [None; Self::MOUSE_BUTTON_COUNT as usize],
            mouse_button_went_down_position: [None; Self::MOUSE_BUTTON_COUNT as usize],
            mouse_button_went_up_position: [None; Self::MOUSE_BUTTON_COUNT as usize],
            mouse_drag_in_progress: [None; Self::MOUSE_BUTTON_COUNT as usize],
            mouse_drag_just_finished: [None; Self::MOUSE_BUTTON_COUNT as usize],
        }
    }

    /// Returns true if the given key is down
    pub fn is_key_down(
        &self,
        key: KeyboardKey,
    ) -> bool {
        if let Some(index) = Self::keyboard_button_to_index(key) {
            self.key_is_down[index]
        } else {
            false
        }
    }

    /// Returns true if the key went down during this frame
    pub fn is_key_just_down(
        &self,
        key: KeyboardKey,
    ) -> bool {
        if let Some(index) = Self::keyboard_button_to_index(key) {
            self.key_just_down[index]
        } else {
            false
        }
    }

    /// Returns true if the key went up during this frame
    pub fn is_key_just_up(
        &self,
        key: KeyboardKey,
    ) -> bool {
        if let Some(index) = Self::keyboard_button_to_index(key) {
            self.key_just_up[index]
        } else {
            false
        }
    }

    /// Get the current mouse position
    pub fn mouse_position(&self) -> glam::Vec2 {
        self.mouse_position
    }

    pub fn mouse_motion(&self) -> glam::Vec2 {
        self.mouse_motion
    }

    /// Get the scroll delta from the current frame
    pub fn mouse_wheel_delta(&self) -> MouseScrollDelta {
        self.mouse_wheel_delta
    }

    /// Returns true if the given button is down
    pub fn is_mouse_down(
        &self,
        mouse_button: MouseButton,
    ) -> bool {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_is_down[index]
        } else {
            false
        }
    }

    /// Returns true if the button went down during this frame
    pub fn is_mouse_just_down(
        &self,
        mouse_button: MouseButton,
    ) -> bool {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_just_down[index].is_some()
        } else {
            false
        }
    }

    /// Returns the position the mouse just went down at, otherwise returns None
    pub fn mouse_just_down_position(
        &self,
        mouse_button: MouseButton,
    ) -> Option<glam::Vec2> {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_just_down[index]
        } else {
            None
        }
    }

    /// Returns true if the button went up during this frame
    pub fn is_mouse_just_up(
        &self,
        mouse_button: MouseButton,
    ) -> bool {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_just_up[index].is_some()
        } else {
            false
        }
    }

    /// Returns the position the mouse just went up at, otherwise returns None
    pub fn mouse_just_up_position(
        &self,
        mouse_button: MouseButton,
    ) -> Option<glam::Vec2> {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_just_up[index]
        } else {
            None
        }
    }

    /// Returns true if the button was just clicked. "Clicked" means the button went down and came
    /// back up without being moved much. If it was moved, it would be considered a drag.
    pub fn is_mouse_button_just_clicked(
        &self,
        mouse_button: MouseButton,
    ) -> bool {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_just_clicked[index].is_some()
        } else {
            false
        }
    }

    /// Returns the position the button was just clicked at, otherwise None. "Clicked" means the
    /// button went down and came back up without being moved much. If it was moved, it would be
    /// considered a drag.
    pub fn mouse_button_just_clicked_position(
        &self,
        mouse_button: MouseButton,
    ) -> Option<glam::Vec2> {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_just_clicked[index]
        } else {
            None
        }
    }

    /// Returns the position the button went down at previously. This could have been some time ago.
    pub fn mouse_button_went_down_position(
        &self,
        mouse_button: MouseButton,
    ) -> Option<glam::Vec2> {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_went_down_position[index]
        } else {
            None
        }
    }

    /// Returns the position the button went up at previously. This could have been some time ago.
    pub fn mouse_button_went_up_position(
        &self,
        mouse_button: MouseButton,
    ) -> Option<glam::Vec2> {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_button_went_up_position[index]
        } else {
            None
        }
    }

    /// Return true if the mouse is being dragged. (A drag means the button went down and mouse
    /// moved, but button hasn't come back up yet)
    pub fn is_mouse_drag_in_progress(
        &self,
        mouse_button: MouseButton,
    ) -> bool {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_drag_in_progress[index].is_some()
        } else {
            false
        }
    }

    /// Returns the mouse drag state if a drag is in process, otherwise None.
    pub fn mouse_drag_in_progress(
        &self,
        mouse_button: MouseButton,
    ) -> Option<MouseDragState> {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_drag_in_progress[index]
        } else {
            None
        }
    }

    /// Return true if a mouse drag completed in the previous frame, otherwise false
    pub fn is_mouse_drag_just_finished(
        &self,
        mouse_button: MouseButton,
    ) -> bool {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_drag_just_finished[index].is_some()
        } else {
            false
        }
    }

    /// Returns information about a mouse drag if it just completed, otherwise None
    pub fn mouse_drag_just_finished(
        &self,
        mouse_button: MouseButton,
    ) -> Option<MouseDragState> {
        if let Some(index) = Self::mouse_button_to_index(mouse_button) {
            self.mouse_drag_just_finished[index]
        } else {
            None
        }
    }

    //
    // Handlers for significant events
    //

    /// Call at the end of every frame. This clears events that were "just" completed.
    pub fn end_frame(&mut self) {
        self.mouse_wheel_delta = MouseScrollDelta { x: 0.0, y: 0.0 };

        for value in self.key_just_down.iter_mut() {
            *value = false;
        }

        for value in self.key_just_up.iter_mut() {
            *value = false;
        }

        for value in self.mouse_button_just_down.iter_mut() {
            *value = None;
        }

        for value in self.mouse_button_just_up.iter_mut() {
            *value = None;
        }

        for value in self.mouse_button_just_clicked.iter_mut() {
            *value = None;
        }

        for value in self.mouse_drag_just_finished.iter_mut() {
            *value = None;
        }

        self.mouse_motion = glam::Vec2::ZERO;

        for value in self.mouse_drag_in_progress.iter_mut() {
            if let Some(v) = value {
                v.previous_frame_delta = glam::Vec2::ZERO;
                //v.world_space_previous_frame_delta = glam::Vec2::zero()
            }
        }
    }

    /// Call when a key event occurs
    pub fn handle_keyboard_event(
        &mut self,
        keyboard_button: KeyboardKey,
        button_state: ButtonState,
    ) {
        if let Some(kc) = Self::keyboard_button_to_index(keyboard_button) {
            // Assign true if key is down, or false if key is up
            if button_state == ButtonState::Pressed {
                if !self.key_is_down[kc] {
                    self.key_just_down[kc] = true;
                }
                self.key_is_down[kc] = true
            } else {
                if self.key_is_down[kc] {
                    self.key_just_up[kc] = true;
                }
                self.key_is_down[kc] = false
            }
        }
    }

    /// Call when a mouse button event occurs
    pub fn handle_mouse_button_event(
        &mut self,
        button: MouseButton,
        button_event: ButtonState,
    ) {
        if let Some(button_index) = Self::mouse_button_to_index(button) {
            assert!(button_index < InputState::MOUSE_BUTTON_COUNT as usize);

            // Update is down/up, just down/up
            match button_event {
                ButtonState::Pressed => {
                    self.mouse_button_just_down[button_index] = Some(self.mouse_position);
                    self.mouse_button_is_down[button_index] = true;

                    self.mouse_button_went_down_position[button_index] = Some(self.mouse_position);
                }
                ButtonState::Released => {
                    self.mouse_button_just_up[button_index] = Some(self.mouse_position);
                    self.mouse_button_is_down[button_index] = false;

                    self.mouse_button_went_up_position[button_index] = Some(self.mouse_position);

                    match self.mouse_drag_in_progress[button_index] {
                        Some(in_progress) => {
                            let delta = self.mouse_position
                                - (in_progress.begin_position
                                    + in_progress.accumulated_frame_delta);

                            self.mouse_drag_just_finished[button_index] = Some(MouseDragState {
                                begin_position: in_progress.begin_position,
                                end_position: self.mouse_position,
                                previous_frame_delta: delta,
                                accumulated_frame_delta: in_progress.accumulated_frame_delta
                                    + delta,
                            });
                        }
                        None => {
                            self.mouse_button_just_clicked[button_index] = Some(self.mouse_position)
                        }
                    }

                    self.mouse_drag_in_progress[button_index] = None;
                }
            }
        }
    }

    /// Call when the cursor moves at all (even if it's locked or outside the window)
    pub fn handle_mouse_motion_event(
        &mut self,
        delta: glam::Vec2,
    ) {
        self.mouse_motion += delta
    }

    /// Call when a cursor moves within the window
    pub fn handle_mouse_update_position(
        &mut self,
        position: glam::Vec2,
    ) {
        // Update mouse position
        self.mouse_position = position;

        // Update drag in progress state
        for i in 0..Self::MOUSE_BUTTON_COUNT {
            let i = i as usize;
            if self.mouse_button_is_down[i] {
                self.mouse_drag_in_progress[i] = match self.mouse_drag_in_progress[i] {
                    None => {
                        match self.mouse_button_went_down_position[i] {
                            Some(went_down_position) => {
                                let min_drag_distance_met =
                                    glam::Vec2::length(went_down_position - self.mouse_position)
                                        > Self::MIN_DRAG_DISTANCE;
                                if min_drag_distance_met {
                                    let delta = self.mouse_position - went_down_position;

                                    // We dragged a non-trivial amount, start the drag
                                    Some(MouseDragState {
                                        begin_position: went_down_position,
                                        end_position: self.mouse_position,
                                        previous_frame_delta: delta,
                                        accumulated_frame_delta: delta,
                                    })
                                } else {
                                    // Mouse moved too small an amount to be considered a drag
                                    None
                                }
                            }

                            // We don't know where the mosue went down, so we can't start a drag
                            None => None,
                        }
                    }
                    Some(old_drag_state) => {
                        // We were already dragging, so just update the end position

                        let delta = self.mouse_position
                            - (old_drag_state.begin_position
                                + old_drag_state.accumulated_frame_delta);

                        Some(MouseDragState {
                            begin_position: old_drag_state.begin_position,
                            end_position: self.mouse_position,
                            previous_frame_delta: delta,
                            accumulated_frame_delta: old_drag_state.accumulated_frame_delta + delta,
                        })
                    }
                };
            }
        }
    }

    pub fn handle_mouse_wheel_event(
        &mut self,
        delta: MouseScrollDelta,
    ) {
        self.mouse_wheel_delta.x += delta.x;
        self.mouse_wheel_delta.y += delta.y;
    }

    /// Convert the winit mouse button enum into a numerical index
    pub fn mouse_button_to_index(button: MouseButton) -> Option<usize> {
        if button.0 >= Self::MOUSE_BUTTON_COUNT {
            None
        } else {
            Some(button.0 as usize)
        }
    }

    /// Convert to the winit mouse button enum from a numerical index
    pub fn mouse_index_to_button(index: usize) -> Option<MouseButton> {
        if index >= Self::MOUSE_BUTTON_COUNT as usize {
            None
        } else {
            Some(MouseButton(index as u16))
        }
    }

    /// Convert the winit virtual key code into a numerical index
    pub fn keyboard_button_to_index(button: KeyboardKey) -> Option<usize> {
        if button as usize >= Self::KEYBOARD_BUTTON_COUNT {
            None
        } else {
            Some(button as usize)
        }
    }
}
