mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(EguiRenderFeature, EGUI_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

#[cfg(feature = "egui-sdl2")]
mod sdl2_egui_manager;
#[cfg(feature = "egui-sdl2")]
pub use sdl2_egui_manager::*;

#[cfg(feature = "egui-winit")]
mod winit_egui_manager;
#[cfg(feature = "egui-winit")]
pub use winit_egui_manager::*;

mod egui_context_resource;
pub use egui_context_resource::*;
