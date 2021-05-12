mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(MeshRenderFeature, MESH_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

pub use jobs::MeshVertex;

mod render_object;
pub use render_object::*;

mod shadow_map_resource;
pub use shadow_map_resource::*;
