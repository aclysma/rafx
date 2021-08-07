mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(SpriteRenderFeature, SPRITE_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

mod render_object;
pub use render_object::*;
