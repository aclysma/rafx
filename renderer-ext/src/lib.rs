pub mod renderpass;

pub mod imgui_support;

mod game_renderer;
pub use game_renderer::GameRenderer;
pub use game_renderer::GameRendererWithContext;

pub mod time;

pub mod asset_resource;
pub mod asset_storage;
pub mod pipeline;
pub mod image_utils;
//pub mod upload;
pub mod resource_managers;
pub mod push_buffer;
pub mod pipeline_description;

use legion::prelude::*;
use glam::Vec3;
use renderer_base::visibility::DynamicAabbVisibilityNodeHandle;

//
// Everything below this point is only being used by the api_design example for prototyping purposes
//
pub mod features;
use features::sprite::SpriteRenderNodeHandle;
use crate::features::demo::DemoRenderNodeHandle;
use renderer_shell_vulkan::{VkResourceDropSink, VkBuffer, VkDeviceContext};
use renderer_shell_vulkan::cleanup::VkResourceDropSinkChannel;
use std::mem::ManuallyDrop;
use ash::vk;
use crate::resource_managers::DynResourceLookupSet;

pub mod phases;

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

pub struct RenderJobExtractContext {
    world: &'static World,
    resources: &'static Resources,
}

impl RenderJobExtractContext {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
    ) -> Self {
        unsafe {
            RenderJobExtractContext {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
            }
        }
    }
}

pub struct RenderJobPrepareContext {
    pub dyn_resource_lookups: DynResourceLookupSet
}

impl RenderJobPrepareContext {
    pub fn new(
        resource_allocators: DynResourceLookupSet,
    ) -> Self {
        RenderJobPrepareContext {
            dyn_resource_lookups: resource_allocators
        }
    }
}


pub struct RenderJobWriteContextFactory {
    pub device_context: VkDeviceContext,
    pub dyn_resource_lookups: DynResourceLookupSet
}

impl RenderJobWriteContextFactory {
    pub fn new(
        device_context: VkDeviceContext,
        resource_allocators: DynResourceLookupSet,
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
    pub dyn_resource_lookups: DynResourceLookupSet,
    pub command_buffer: vk::CommandBuffer,
}

impl RenderJobWriteContext {
    pub fn new(
        device_context: VkDeviceContext,
        resource_allocators: DynResourceLookupSet,
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