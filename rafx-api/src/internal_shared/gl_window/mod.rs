use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use std::ffi::c_void;
use std::marker::PhantomData;

#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
use win as platform;

#[cfg(target_os = "linux")]
mod x11;
#[cfg(target_os = "linux")]
use self::x11 as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

pub struct GlConfig {
    pub version: (u8, u8),
    pub profile: Profile,
    pub red_bits: u8,
    pub blue_bits: u8,
    pub green_bits: u8,
    pub alpha_bits: u8,
    pub depth_bits: u8,
    pub stencil_bits: u8,
    pub samples: Option<u8>,
    pub srgb: bool,
    pub double_buffer: bool,
    pub vsync: bool,

    // Not available on macOS
    pub use_debug_context: bool,
}

impl Default for GlConfig {
    fn default() -> Self {
        GlConfig {
            version: (3, 2),
            profile: Profile::Core,
            red_bits: 8,
            blue_bits: 8,
            green_bits: 8,
            alpha_bits: 8,
            depth_bits: 24,
            stencil_bits: 8,
            samples: None,
            srgb: true,
            double_buffer: true,
            vsync: false,
            use_debug_context: false,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Profile {
    Compatibility,
    Core,
}

#[derive(Debug)]
pub enum GlError {
    InvalidWindowHandle,
    VersionNotSupported,
    CreationFailed,
}

pub struct GlContext {
    context: platform::GlContext,
    phantom: PhantomData<*mut ()>,
}

impl GlContext {
    pub fn create(
        display: &dyn HasRawDisplayHandle,
        window: &dyn HasRawWindowHandle,
        config: GlConfig,
        share: Option<&GlContext>,
    ) -> Result<GlContext, GlError> {
        platform::GlContext::create(display, window, config, share.map(|x| &x.context)).map(
            |context| GlContext {
                context,
                phantom: PhantomData,
            },
        )
    }

    pub fn make_current(&self) {
        self.context.make_current();
    }

    pub fn make_not_current(&self) {
        self.context.make_not_current();
    }

    pub fn get_proc_address(
        &self,
        symbol: &str,
    ) -> *const c_void {
        self.context.get_proc_address(symbol)
    }

    pub fn swap_buffers(&self) {
        self.context.swap_buffers();
    }
}
