pub mod features;
pub mod phases;
pub mod renderpass;

pub mod imgui_support;

mod game_renderer;
pub use game_renderer::GameRenderer;
pub use game_renderer::GameRendererWithShell;

mod resource_manager;
pub use resource_manager::ResourceManager;

pub mod time;

use legion::prelude::*;
use glam::Vec3;
use features::sprite::SpriteRenderNodeHandle;
use renderer_base::visibility::DynamicAabbVisibilityNodeHandle;

pub mod asset_resource;
pub mod asset_storage;
pub mod image_importer;

pub mod asset_uploader;

#[derive(Copy, Clone)]
pub struct PositionComponent {
    pub position: Vec3,
}

#[derive(Clone)]
pub struct SpriteComponent {
    pub sprite_handle: SpriteRenderNodeHandle,
    pub visibility_handle: DynamicAabbVisibilityNodeHandle,
    pub alpha: f32,
}

pub struct ExtractSource {
    world: &'static World,
    resources: &'static Resources,
}

impl ExtractSource {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
    ) -> Self {
        unsafe {
            ExtractSource {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
            }
        }
    }
}

pub struct CommandWriter {}

impl CommandWriter {}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}
