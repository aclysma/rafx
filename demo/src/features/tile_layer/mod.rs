mod jobs;
use jobs::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(TileLayerRenderFeature, TILE_LAYER_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

pub use jobs::TileLayerVertex;

mod tile_layer_render_node_set;
pub use tile_layer_render_node_set::*;

mod tile_layer_resource;
pub use tile_layer_resource::*;
