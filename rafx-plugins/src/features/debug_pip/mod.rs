mod jobs;
use jobs::*;
mod internal;
use internal::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(DebugPipRenderFeature, DEBUG_PIP_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;

mod debug_pip_resource;
pub use debug_pip_resource::*;
