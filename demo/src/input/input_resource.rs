use std::ops::{Deref, DerefMut};

use crate::input::InputState;

// For now just wrap the input helper that skulpin provides
pub struct InputResource {
    input_state: InputState,
}

impl InputResource {
    pub fn new() -> Self {
        InputResource {
            input_state: InputState::new(),
        }
    }

    pub fn input_state(&self) -> &InputState {
        &self.input_state
    }

    pub fn input_state_mut(&mut self) -> &mut InputState {
        &mut self.input_state
    }
}

impl Deref for InputResource {
    type Target = InputState;

    fn deref(&self) -> &Self::Target {
        self.input_state()
    }
}

impl DerefMut for InputResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.input_state_mut()
    }
}
