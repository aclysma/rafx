mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(TextRenderFeature, TEXT_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

mod text_resource;
pub use text_resource::*;
