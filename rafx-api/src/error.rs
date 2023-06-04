#[cfg(feature = "rafx-vulkan")]
use ash::vk;
use std::sync::Arc;

pub type RafxResult<T> = Result<T, RafxError>;

/// Generic error that contains all the different kinds of errors that may occur when using the API
#[derive(Debug, Clone)]
pub enum RafxError {
    StringError(String),
    ValidationRequiredButUnavailable,
    IoError(Arc<std::io::Error>),
    #[cfg(feature = "rafx-dx12")]
    WindowsApiError(windows::core::Error),
    #[cfg(feature = "rafx-dx12")]
    HResult(windows::core::HRESULT),
    #[cfg(feature = "rafx-dx12")]
    HassleError(Arc<hassle_rs::HassleError>),
    #[cfg(feature = "rafx-vulkan")]
    VkError(vk::Result),
    #[cfg(feature = "rafx-vulkan")]
    VkLoadingError(Arc<ash::LoadingError>),
    #[cfg(any(feature = "rafx-dx12", feature = "rafx-vulkan",))]
    AllocationError(Arc<gpu_allocator::AllocationError>),
    #[cfg(any(feature = "rafx-gles2", feature = "rafx-gles3"))]
    GlError(u32),
}

impl std::error::Error for RafxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            RafxError::StringError(_) => None,
            RafxError::ValidationRequiredButUnavailable => None,
            RafxError::IoError(ref e) => Some(&**e),

            #[cfg(feature = "rafx-dx12")]
            RafxError::WindowsApiError(ref e) => Some(e),
            #[cfg(feature = "rafx-dx12")]
            RafxError::HResult(ref e) => None,
            #[cfg(feature = "rafx-dx12")]
            RafxError::HassleError(ref e) => Some(e),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkError(ref e) => Some(e),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkLoadingError(ref e) => Some(&**e),
            #[cfg(any(feature = "rafx-dx12", feature = "rafx-vulkan",))]
            RafxError::AllocationError(ref e) => Some(&**e),
            #[cfg(any(feature = "rafx-gles2", feature = "rafx-gles3"))]
            RafxError::GlError(_) => None,
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
            RafxError::ValidationRequiredButUnavailable => {
                "ValidationRequiredButUnavailable".fmt(fmt)
            }
            RafxError::IoError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-dx12")]
            RafxError::WindowsApiError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-dx12")]
            RafxError::HResult(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-dx12")]
            RafxError::HassleError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkError(ref e) => e.fmt(fmt),
            #[cfg(feature = "rafx-vulkan")]
            RafxError::VkLoadingError(ref e) => e.fmt(fmt),
            #[cfg(any(feature = "rafx-dx12", feature = "rafx-vulkan",))]
            RafxError::AllocationError(ref e) => e.fmt(fmt),
            #[cfg(any(feature = "rafx-gles2", feature = "rafx-gles3"))]
            RafxError::GlError(ref e) => e.fmt(fmt),
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

#[cfg(feature = "rafx-dx12")]
impl From<windows::core::Error> for RafxError {
    fn from(result: windows::core::Error) -> Self {
        RafxError::WindowsApiError(result)
    }
}

#[cfg(feature = "rafx-dx12")]
impl From<windows::core::HRESULT> for RafxError {
    fn from(result: windows::core::HRESULT) -> Self {
        RafxError::HResult(result)
    }
}

#[cfg(feature = "rafx-dx12")]
impl From<hassle_rs::HassleError> for RafxError {
    fn from(result: hassle_rs::HassleError) -> Self {
        RafxError::HassleError(Arc::new(result))
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

#[cfg(any(feature = "rafx-dx12", feature = "rafx-vulkan",))]
impl From<gpu_allocator::AllocationError> for RafxError {
    fn from(error: gpu_allocator::AllocationError) -> Self {
        RafxError::AllocationError(Arc::new(error))
    }
}
