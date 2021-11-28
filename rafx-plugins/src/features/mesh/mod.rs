mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(MeshRenderFeature, MESH_FEATURE_INDEX);

// Generates submit nodes for drawing wireframes into the `WireframeRenderPhase` if the view is
// registered for both the `WireframeRenderPhase` and the optional `MeshWireframeRenderFeatureFlag`.
rafx::declare_render_feature_flag!(MeshWireframeRenderFeatureFlag, MESH_WIREFRAME_FLAG_INDEX);

// Generates untextured submit nodes if the `MeshUntexturedRenderFeatureFlag` is registered on the view.
rafx::declare_render_feature_flag!(MeshUntexturedRenderFeatureFlag, MESH_UNTEXTURED_FLAG_INDEX);

// Ignores lighting when the `MeshUnlitRenderFeatureFlag` is registered on the view.
rafx::declare_render_feature_flag!(MeshUnlitRenderFeatureFlag, MESH_UNLIT_FLAG_INDEX);

// Ignores shadows when the `MeshNoShadowsRenderFeatureFlag` is registered on the view.
rafx::declare_render_feature_flag!(MeshNoShadowsRenderFeatureFlag, MESH_NO_SHADOWS_FLAG_INDEX);

// Public API

mod plugin;
pub use plugin::*;

pub use jobs::MeshVertexFull;
pub use jobs::MeshVertexPosition;

mod render_object;
pub use render_object::*;

mod shadow_map_resource;
pub use shadow_map_resource::*;

mod render_options;
pub use render_options::*;
