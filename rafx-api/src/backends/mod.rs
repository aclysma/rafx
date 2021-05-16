#[cfg(feature = "rafx-metal")]
pub mod metal;

#[cfg(feature = "rafx-vulkan")]
pub mod vulkan;

#[cfg(feature = "rafx-gles2")]
pub mod gles2;

#[cfg(feature = "rafx-gles3")]
pub mod gles3;

#[cfg(any(
    feature = "rafx-empty",
    not(any(feature = "rafx-metal", feature = "rafx-vulkan", feature = "rafx-gles2", feature = "rafx-gles3"))
))]
#[doc(hidden)]
#[rustfmt::skip]
pub mod empty;
