use std::ffi::c_void;
use std::str::FromStr;

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use cocoa::appkit::{
    NSOpenGLContext, NSOpenGLContextParameter, NSOpenGLPFAAccelerated, NSOpenGLPFAAlphaSize,
    NSOpenGLPFAColorSize, NSOpenGLPFADepthSize, NSOpenGLPFADoubleBuffer, NSOpenGLPFAMultisample,
    NSOpenGLPFAOpenGLProfile, NSOpenGLPFASampleBuffers, NSOpenGLPFASamples, NSOpenGLPFAStencilSize,
    NSOpenGLPixelFormat, NSOpenGLProfileVersion3_2Core, NSOpenGLProfileVersion4_1Core,
    NSOpenGLProfileVersionLegacy,
};
use cocoa::base::{id, nil};

use core_foundation::base::TCFType;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};
use core_foundation::string::CFString;

use objc::{msg_send, sel, sel_impl};

use super::{GlConfig, GlError, Profile};
use cocoa::foundation::NSAutoreleasePool;
use objc::runtime::Object;

pub struct GlContext {
    context: id,
}

impl GlContext {
    pub fn create(
        parent: &dyn HasRawWindowHandle,
        config: GlConfig,
        shared_context: Option<&GlContext>,
    ) -> Result<GlContext, GlError> {
        let handle = if let RawWindowHandle::MacOS(handle) = parent.raw_window_handle() {
            handle
        } else {
            return Err(GlError::InvalidWindowHandle);
        };

        let ns_view = if !handle.ns_view.is_null() {
            Ok(handle.ns_view as id)
        } else if !handle.ns_window.is_null() {
            let ns_window = handle.ns_window as *mut Object;
            let ns_view: *mut c_void = unsafe { msg_send![ns_window, contentView] };

            assert!(!ns_view.is_null());
            Ok(ns_view as id)
        } else {
            return Err(GlError::InvalidWindowHandle);
        }?;

        let parent_view = ns_view as id;

        unsafe {
            let version = if config.version < (3, 2) && config.profile == Profile::Compatibility {
                NSOpenGLProfileVersionLegacy
            } else if config.version == (3, 2) && config.profile == Profile::Core {
                NSOpenGLProfileVersion3_2Core
            } else if config.version > (3, 2) && config.profile == Profile::Core {
                NSOpenGLProfileVersion4_1Core
            } else {
                return Err(GlError::VersionNotSupported);
            };

            #[rustfmt::skip]
            let mut attrs = vec![
                NSOpenGLPFAOpenGLProfile as u32, version as u32,
                NSOpenGLPFAColorSize as u32, (config.red_bits + config.blue_bits + config.green_bits) as u32,
                NSOpenGLPFAAlphaSize as u32, config.alpha_bits as u32,
                NSOpenGLPFADepthSize as u32, config.depth_bits as u32,
                NSOpenGLPFAStencilSize as u32, config.stencil_bits as u32,
                NSOpenGLPFAAccelerated as u32,
            ];

            if config.samples.is_some() {
                #[rustfmt::skip]
                attrs.extend_from_slice(&[
                    NSOpenGLPFAMultisample as u32,
                    NSOpenGLPFASampleBuffers as u32, 1,
                    NSOpenGLPFASamples as u32, config.samples.unwrap() as u32,
                ]);
            }

            if config.double_buffer {
                attrs.push(NSOpenGLPFADoubleBuffer as u32);
            }

            attrs.push(0);

            let pixel_format = NSOpenGLPixelFormat::alloc(nil).initWithAttributes_(&attrs);

            if pixel_format == nil {
                return Err(GlError::CreationFailed);
            }

            let shared_context = shared_context.map(|x| x.context).unwrap_or(nil);

            let gl_context = NSOpenGLContext::alloc(nil)
                .initWithFormat_shareContext_(pixel_format, shared_context);

            if gl_context == nil {
                return Err(GlError::CreationFailed);
            }

            gl_context.setView_(parent_view);

            gl_context.setValues_forParameter_(
                &(config.vsync as i32),
                NSOpenGLContextParameter::NSOpenGLCPSwapInterval,
            );

            let () = msg_send![pixel_format, release];

            Ok(GlContext {
                context: gl_context,
            })
        }
    }

    pub fn make_current(&self) {
        unsafe {
            self.context.makeCurrentContext();
        }
    }

    pub fn make_not_current(&self) {
        unsafe {
            NSOpenGLContext::clearCurrentContext(self.context);
        }
    }

    pub fn get_proc_address(
        &self,
        symbol: &str,
    ) -> *const c_void {
        let symbol_name = CFString::from_str(symbol).unwrap();
        let framework_name = CFString::from_str("com.apple.opengl").unwrap();
        let framework =
            unsafe { CFBundleGetBundleWithIdentifier(framework_name.as_concrete_TypeRef()) };
        let addr = unsafe {
            CFBundleGetFunctionPointerForName(framework, symbol_name.as_concrete_TypeRef())
        };
        addr as *const c_void
    }

    pub fn swap_buffers(&self) {
        unsafe {
            let pool = NSAutoreleasePool::new(nil);
            self.context.flushBuffer();
            self.context.update();
            let _: () = msg_send![pool, release];
        }
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        unsafe {
            let () = msg_send![self.context, release];
        }
    }
}
