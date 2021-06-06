mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(EguiRenderFeature, EGUI_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

mod sdl2_egui_manager;
pub use sdl2_egui_manager::*;

mod winit_egui_manager;
pub use winit_egui_manager::*;

mod egui_context_resource;
pub use egui_context_resource::*;
