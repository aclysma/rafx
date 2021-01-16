#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::VkCreateInstanceError;
#[cfg(feature = "rafx-vulkan")]
use ash::vk;
use std::sync::Arc;

pub type RafxResult<T> = Result<T, RafxError>;

/// Generic error that contains all the different kinds of errors that may occur when using the API
#[derive(Debug, Clone)]
pub enum RafxError {
    StringError(String),
    IoError(Arc<std::io::Error>),
    #[cfg(feature = "rafx-vulkan")]
    VkError(vk::Result),
    #[cfg(feature = "rafx-vulkan")]
    VkLoadingError(Arc<ash::LoadingError>),
    #[cfg(feature = "rafx-vulkan")]
    VkCreateInstanceError(Arc<VkCreateInstanceError>),
    #[cfg(feature = "rafx-vulkan")]
    VkMemError(Arc<vk_mem::Error>),
}

impl std::error::Error for RafxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            RafxError::StringError(_) => None,
            RafxError::IoError(ref e) => Some(&**e),

            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkError(ref e) => Some(e),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkLoadingError(ref e) => Some(&**e),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkCreateInstanceError(ref e) => Some(&**e),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkMemError(ref e) => Some(&**e),
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
            RafxError::IoError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkLoadingError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkCreateInstanceError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkMemError(ref e) => e.fmt(fmt),
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

impl From<std::io::Error> for RafxError {
    fn from(error: std::io::Error) -> Self {
        RafxError::IoError(Arc::new(error))
    }
}

#[cfg(feature = "rafx-vulkan")]
impl From<vk::Result> for RafxError {
    fn from(result: vk::Result) -> Self {
        RafxError::VkError(result)
    }
}

#[cfg(feature = "rafx-vulkan")]
impl From<ash::LoadingError> for RafxError {
    fn from(result: ash::LoadingError) -> Self {
        RafxError::VkLoadingError(Arc::new(result))
    }
}

#[cfg(feature = "rafx-vulkan")]
impl From<VkCreateInstanceError> for RafxError {
    fn from(result: VkCreateInstanceError) -> Self {
        RafxError::VkCreateInstanceError(Arc::new(result))
    }
}

#[cfg(feature = "rafx-vulkan")]
impl From<vk_mem::Error> for RafxError {
    fn from(error: vk_mem::Error) -> Self {
        RafxError::VkMemError(Arc::new(error))
    }
}
