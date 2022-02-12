mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(MeshAdvRenderFeature, MESH_ADV_FEATURE_INDEX);

// Generates submit nodes for drawing wireframes into the `WireframeRenderPhase` if the view is
// registered for both the `WireframeRenderPhase` and the optional `MeshAdvWireframeRenderFeatureFlag`.
rafx::declare_render_feature_flag!(
    MeshAdvWireframeRenderFeatureFlag,
    MESH_ADV_WIREFRAME_FLAG_INDEX
);

// Generates untextured submit nodes if the `MeshAdvUntexturedRenderFeatureFlag` is registered on the view.
rafx::declare_render_feature_flag!(
    MeshAdvUntexturedRenderFeatureFlag,
    MESH_ADV_UNTEXTURED_FLAG_INDEX
);

// Ignores lighting when the `MeshAdvUnlitRenderFeatureFlag` is registered on the view.
rafx::declare_render_feature_flag!(MeshAdvUnlitRenderFeatureFlag, MESH_ADV_UNLIT_FLAG_INDEX);

// Ignores shadows when the `MeshAdvNoShadowsRenderFeatureFlag` is registered on the view.
rafx::declare_render_feature_flag!(
    MeshAdvNoShadowsRenderFeatureFlag,
    MESH_ADV_NO_SHADOWS_FLAG_INDEX
);

// Public API

mod plugin;
pub use plugin::*;

pub use jobs::MeshVertexFull;
pub use jobs::MeshVertexPosition;
pub use jobs::ShadowMapAtlasClearTileVertex;
pub use jobs::SHADOW_MAP_ATLAS_CLEAR_TILE_LAYOUT;

mod render_object;
pub use render_object::*;

mod shadow_map_resource;
pub use shadow_map_resource::*;

mod render_options;
pub use render_options::*;

pub mod light_binning;
mod shadow_map_atlas;

mod gpu_occlusion_cull;
pub use gpu_occlusion_cull::*;

pub use shadow_map_atlas::*;
