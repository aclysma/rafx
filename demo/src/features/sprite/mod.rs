mod jobs;
use jobs::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(SpriteRenderFeature, SPRITE_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

mod sprite_render_node_set;
pub use sprite_render_node_set::*;
