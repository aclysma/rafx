pub mod features;
pub mod phases;

use legion::prelude::*;
use glam::Vec3;
use features::sprite::SpriteRenderNodeHandle;
use renderer_base::visibility::DynamicAabbVisibilityNodeHandle;

#[derive(Copy, Clone)]
pub struct PositionComponent {
    pub position: Vec3,
}

#[derive(Clone)]
pub struct SpriteComponent {
    pub sprite_handle: SpriteRenderNodeHandle,
    pub visibility_handle: DynamicAabbVisibilityNodeHandle,
}

//type ExtractSource = (World, Resources);

// pub struct ExtractSource<'a> {
//     world: &'a World,
//     resources: &'a Resources
// }
//
// impl<'a> ExtractSource<'a> {
//     pub fn new(world: &'a World, resources: &'a Resources) -> Self {
//         ExtractSource {
//             world,
//             resources
//         }
//     }
// }

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

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}

//
//
// struct ExtractSource<'a> {
//     position_components: &'a Read<PositionComponent>,
//     sprite_components: &'a Read<SpriteComponent>
// }
//
// impl<'a> ExtractSource<'a> {
//     pub fn new(world: &'a World) -> Self {
//
//         position
//
//         ExtractSource {
//             position_components
//         }
//
//     }
// }
