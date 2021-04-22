
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
use std::ffi::CString;

pub fn calculate_window_hash(window: &dyn HasRawWindowHandle) -> WindowHash {
    let mut hasher = FnvHasher::default();
    window.raw_window_handle().hash(&mut hasher);
    WindowHash(hasher.finish())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WindowHash(u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub u32);
pub const NONE_BUFFER: BufferId = BufferId(gles20::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VertexArrayObjectId(pub u32);
pub const NONE_VERTEX_ARRAY_OBJECT: VertexArrayObjectId = VertexArrayObjectId(gles20::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);
pub const NONE_TEXTURE: TextureId = TextureId(gles20::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderbufferId(pub u32);
pub const NONE_RENDERBUFFER: RenderbufferId = RenderbufferId(gles20::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderId(pub u32);
pub const NONE_SHADER: ShaderId = ShaderId(gles20::NONE);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProgramId(pub u32);
pub const NONE_PROGRAM: ProgramId = ProgramId(gles20::NONE);

pub struct ActiveUniformInfo {
    pub name: CString,
    pub size: u32,
    pub ty: u32,
}
