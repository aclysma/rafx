#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(not(target_arch = "wasm32"))]
pub mod gles2_bindings;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(target_arch = "wasm32")]
pub mod gles2_bindings;

use fnv::FnvHasher;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::ffi::CString;
use std::hash::{Hash, Hasher};

pub fn calculate_window_hash(
    display: &dyn HasRawDisplayHandle,
    window: &dyn HasRawWindowHandle,
) -> WindowHash {
    let mut hasher = FnvHasher::default();
    display.raw_display_handle().hash(&mut hasher);
    window.raw_window_handle().hash(&mut hasher);
    WindowHash(hasher.finish())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WindowHash(u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub u32);
pub const NONE_BUFFER: BufferId = BufferId(gles2_bindings::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);
pub const NONE_TEXTURE: TextureId = TextureId(gles2_bindings::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FramebufferId(pub u32);
pub const NONE_FRAMEBUFFER: FramebufferId = FramebufferId(gles2_bindings::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderbufferId(pub u32);
pub const NONE_RENDERBUFFER: RenderbufferId = RenderbufferId(gles2_bindings::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderId(pub u32);
pub const NONE_SHADER: ShaderId = ShaderId(gles2_bindings::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProgramId(pub u32);
pub const NONE_PROGRAM: ProgramId = ProgramId(gles2_bindings::NONE);

pub struct ActiveUniformInfo {
    pub name: CString,
    pub size: u32,
    pub ty: u32,
}
