
pub mod renderpass;
pub mod imgui_support;
pub mod features;
pub mod phases;

mod features_init;
pub use features_init::init_renderer_features;
pub use features_init::destroy_renderer_features;

use legion::prelude::*;
use glam::Vec3;
use renderer_visibility::DynamicAabbVisibilityNodeHandle;

//
// Everything below this point is only being used by the api_design example for prototyping purposes
//
use features::sprite::SpriteRenderNodeHandle;
use crate::features::demo::DemoRenderNodeHandle;
use renderer_shell_vulkan::{VkResourceDropSink, VkBuffer, VkDeviceContext};
use renderer_shell_vulkan::cleanup::VkResourceDropSinkChannel;
use std::mem::ManuallyDrop;
use ash::vk;
use atelier_assets::loader::handle::Handle;
use crate::features::mesh::{MeshRenderNodeHandle};
use renderer_assets::assets::image::ImageAsset;
use renderer_assets::assets::gltf::MeshAsset;
use renderer_resources::resource_managers::{ResourceManager, DynResourceAllocatorSet};


#[derive(Copy, Clone)]
pub struct PositionComponent {
    pub position: Vec3,
}

#[derive(Clone)]
pub struct PointLightComponent {
    pub color: glam::Vec4,
    pub range: f32,
    pub intensity: f32,
}

#[derive(Clone)]
pub struct DirectionalLightComponent {
    pub direction: glam::Vec3,
    pub color: glam::Vec4,
    pub intensity: f32,
}

#[derive(Clone)]
pub struct SpotLightComponent {
    pub direction: glam::Vec3,
    pub color: glam::Vec4,
    pub spotlight_half_angle: f32,
    pub range: f32,
    pub intensity: f32,
}

#[derive(Clone)]
pub struct SpriteComponent {
    pub sprite_handle: SpriteRenderNodeHandle,
    pub visibility_handle: DynamicAabbVisibilityNodeHandle,
    pub alpha: f32,
    pub image: Handle<ImageAsset>,
    //pub texture_material: ResourceArc<>
}

#[derive(Clone)]
pub struct MeshComponent {
    pub mesh_handle: MeshRenderNodeHandle,
    pub visibility_handle: DynamicAabbVisibilityNodeHandle,
    pub mesh: Handle<MeshAsset>,
}

pub struct RenderJobExtractContext {
    world: &'static World,
    resources: &'static Resources,
    resource_manager: &'static ResourceManager,
}

impl RenderJobExtractContext {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
        resource_manager: &'a ResourceManager,
    ) -> Self {
        unsafe {
            RenderJobExtractContext {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
                resource_manager: force_to_static_lifetime(resource_manager)
            }
        }
    }
}

pub struct RenderJobPrepareContext {
    pub dyn_resource_lookups: DynResourceAllocatorSet
}

impl RenderJobPrepareContext {
    pub fn new(
        resource_allocators: DynResourceAllocatorSet,
    ) -> Self {
        RenderJobPrepareContext {
            dyn_resource_lookups: resource_allocators
        }
    }
}

// Used to produce RenderJobWriteContexts per each job
pub struct RenderJobWriteContextFactory {
    pub device_context: VkDeviceContext,
    pub dyn_resource_lookups: DynResourceAllocatorSet
}

impl RenderJobWriteContextFactory {
    pub fn new(
        device_context: VkDeviceContext,
        resource_allocators: DynResourceAllocatorSet,
    ) -> Self {
        RenderJobWriteContextFactory {
            device_context,
            dyn_resource_lookups: resource_allocators
        }
    }

    pub fn create_context(
        &self,
        command_buffer: vk::CommandBuffer
    ) -> RenderJobWriteContext {
        RenderJobWriteContext {
            device_context: self.device_context.clone(),
            dyn_resource_lookups: self.dyn_resource_lookups.clone(),
            command_buffer
        }
    }
}

pub struct RenderJobWriteContext {
    pub device_context: VkDeviceContext,
    pub dyn_resource_lookups: DynResourceAllocatorSet,
    pub command_buffer: vk::CommandBuffer,
}

impl RenderJobWriteContext {
    pub fn new(
        device_context: VkDeviceContext,
        resource_allocators: DynResourceAllocatorSet,
        command_buffer: vk::CommandBuffer,
    ) -> Self {
        RenderJobWriteContext {
            device_context,
            dyn_resource_lookups: resource_allocators,
            command_buffer
        }
    }
}


unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}
unsafe fn force_to_static_lifetime_mut<T>(value: &mut T) -> &'static mut T {
    std::mem::transmute(value)
}


//
// Just for demonstration of minimal API
//
pub struct DemoExtractContext {
    world: &'static World,
    resources: &'static Resources,
}

impl DemoExtractContext {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
    ) -> Self {
        unsafe {
            DemoExtractContext {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
            }
        }
    }
}

pub struct DemoPrepareContext;
pub struct DemoWriteContext;

#[derive(Clone)]
pub struct DemoComponent {
    pub render_node_handle: DemoRenderNodeHandle,
    pub visibility_handle: DynamicAabbVisibilityNodeHandle,
    pub alpha: f32,
}