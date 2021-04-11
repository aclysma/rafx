mod jobs;
use jobs::*;

use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(SkyboxRenderFeature, SKYBOX_FEATURE_INDEX);

// Public API

mod plugin;
pub use plugin::*;
