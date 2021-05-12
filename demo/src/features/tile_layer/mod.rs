mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(TileLayerRenderFeature, TILE_LAYER_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

pub use jobs::TileLayerVertex;

mod render_object;
pub use render_object::*;

mod tile_layer_resource;
pub use tile_layer_resource::*;
