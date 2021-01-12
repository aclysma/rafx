use std::sync::Arc;

pub use ash;
pub use vk_mem;

use crate::vulkan::device::VkCreateDeviceError;
use crate::vulkan::VkCreateInstanceError;
pub use api::*;
use ash::vk;
pub use buffer::*;
pub use command_buffer::*;
pub use command_pool::*;
pub use descriptor_set_array::*;
pub use device_context::*;
pub use extra::swapchain_helper::*;
pub use fence::*;
pub use pipeline::*;
pub use queue::*;
pub use render_target::*;
pub use root_signature::*;
pub use sampler::*;
pub use semaphore::*;
pub use shader::*;
pub use shader_module::*;
pub use swapchain::*;
pub use texture::*;
pub use types::*;
#[cfg(feature = "rafx-vulkan")]
pub use vulkan::RafxApiDefVulkan;

#[cfg(feature = "rafx-metal")]
pub mod metal;

pub mod extra;
mod types;
#[cfg(feature = "rafx-vulkan")]
pub mod vulkan;

mod api;
mod buffer;
mod command_buffer;
mod command_pool;
mod descriptor_set_array;
mod device_context;
mod fence;
mod pipeline;
mod queue;
mod reflection;
mod render_target;
mod root_signature;
mod sampler;
mod semaphore;
mod shader;
mod shader_module;
mod swapchain;
mod texture;

pub type RafxResult<T> = Result<T, RafxError>;

#[derive(Debug, Clone)]
pub enum RafxError {
    StringError(String),
    VkError(vk::Result),
    VkLoadingError(Arc<ash::LoadingError>),
    VkCreateInstanceError(Arc<VkCreateInstanceError>),
    VkCreateDeviceError(Arc<VkCreateDeviceError>),
    VkMemError(Arc<vk_mem::Error>),
    IoError(Arc<std::io::Error>),
}

impl std::error::Error for RafxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            RafxError::StringError(_) => None,
            RafxError::VkError(ref e) => Some(e),
            RafxError::VkLoadingError(ref e) => Some(&**e),
            RafxError::VkCreateInstanceError(ref e) => Some(&**e),
            RafxError::VkCreateDeviceError(ref e) => Some(&**e),
            RafxError::VkMemError(ref e) => Some(&**e),
            RafxError::IoError(ref e) => Some(&**e),
        }
    }
}

impl core::fmt::Display for RafxError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            RafxError::StringError(ref e) => e.fmt(fmt),
            RafxError::VkError(ref e) => e.fmt(fmt),
            RafxError::VkLoadingError(ref e) => e.fmt(fmt),
            RafxError::VkCreateInstanceError(ref e) => e.fmt(fmt),
            RafxError::VkCreateDeviceError(ref e) => e.fmt(fmt),
            RafxError::VkMemError(ref e) => e.fmt(fmt),
            RafxError::IoError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<&str> for RafxError {
    fn from(str: &str) -> Self {
        RafxError::StringError(str.to_string())
    }
}

impl From<String> for RafxError {
    fn from(string: String) -> Self {
        RafxError::StringError(string)
    }
}

impl From<vk::Result> for RafxError {
    fn from(result: vk::Result) -> Self {
        RafxError::VkError(result)
    }
}

impl From<ash::LoadingError> for RafxError {
    fn from(result: ash::LoadingError) -> Self {
        RafxError::VkLoadingError(Arc::new(result))
    }
}

impl From<VkCreateInstanceError> for RafxError {
    fn from(result: VkCreateInstanceError) -> Self {
        RafxError::VkCreateInstanceError(Arc::new(result))
    }
}

impl From<VkCreateDeviceError> for RafxError {
    fn from(result: VkCreateDeviceError) -> Self {
        RafxError::VkCreateDeviceError(Arc::new(result))
    }
}

impl From<std::io::Error> for RafxError {
    fn from(error: std::io::Error) -> Self {
        RafxError::IoError(Arc::new(error))
    }
}

impl From<vk_mem::Error> for RafxError {
    fn from(error: vk_mem::Error) -> Self {
        RafxError::VkMemError(Arc::new(error))
    }
}
