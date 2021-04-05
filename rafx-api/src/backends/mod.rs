#[cfg(feature = "rafx-metal")]
pub mod metal;

#[cfg(feature = "rafx-vulkan")]
pub mod vulkan;

#[cfg(feature = "rafx-gl")]
pub mod gl;

#[cfg(any(
    feature = "rafx-empty",
    not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
))]
#[doc(hidden)]
#[rustfmt::skip]
pub mod empty;
