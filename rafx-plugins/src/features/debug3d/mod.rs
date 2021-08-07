mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(Debug3DRenderFeature, DEBUG_3D_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

mod debug3d_resource;
pub use debug3d_resource::*;
