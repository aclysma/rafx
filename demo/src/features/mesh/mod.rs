mod jobs;
use jobs::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(MeshRenderFeature, MESH_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

mod mesh_vertex;
pub use mesh_vertex::*;

mod mesh_render_node_set;
pub use mesh_render_node_set::*;

mod shadow_map_resource;
pub use shadow_map_resource::*;
