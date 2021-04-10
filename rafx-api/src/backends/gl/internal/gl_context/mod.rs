
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(not(target_arch = "wasm32"))]
pub mod gles20;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(target_arch = "wasm32")]
pub mod gles20;

use raw_window_handle::HasRawWindowHandle;
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

pub fn calculate_window_hash(window: &dyn HasRawWindowHandle) -> WindowHash {
    let mut hasher = FnvHasher::default();
    window.raw_window_handle().hash(&mut hasher);
    WindowHash(hasher.finish())
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct WindowHash(u64);