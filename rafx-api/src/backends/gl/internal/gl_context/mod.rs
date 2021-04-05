
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