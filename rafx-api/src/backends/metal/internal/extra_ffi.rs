use core_foundation::string::CFStringRef;
use core_graphics_types::base::CGFloat;
use metal_rs::MetalLayerRef;
use objc::rc::StrongPtr;
use objc::runtime::{Object, BOOL, YES};
use objc::{class, msg_send, sel, sel_impl};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub static kCGColorSpaceSRGB: CFStringRef;
    pub static kCGColorSpaceExtendedLinearSRGB: CFStringRef;
    pub static kCGColorSpaceExtendedLinearDisplayP3: CFStringRef;
}

pub fn set_colorspace(
    layer: &MetalLayerRef,
    colorspace: &core_graphics::color_space::CGColorSpaceRef,
) {
    unsafe { msg_send![layer, setColorspace: colorspace] }
}

pub struct NSWindowWrapper(StrongPtr);

impl NSWindowWrapper {
    pub fn new(window: *mut Object) -> Self {
        unsafe {
            assert!(!window.is_null());
            let class = class!(NSWindow);
            let is_actually_window: BOOL = msg_send![window, isKindOfClass: class];
            assert_eq!(is_actually_window, YES);

            let ptr = StrongPtr::retain(window);
            NSWindowWrapper(ptr)
        }
    }

    pub fn max_potential_edr_color_component_value(&self) -> f32 {
        unsafe {
            let screen_id: cocoa::base::id = msg_send![*self.0, screen];
            let max_edr: CGFloat = msg_send![
                screen_id,
                maximumPotentialExtendedDynamicRangeColorComponentValue
            ];
            max_edr as f32
        }
    }

    pub fn max_edr_color_component_value(&self) -> f32 {
        unsafe {
            let screen_id: cocoa::base::id = msg_send![*self.0, screen];
            let max_edr: CGFloat =
                msg_send![screen_id, maximumExtendedDynamicRangeColorComponentValue];
            max_edr as f32
        }
    }

    pub fn max_reference_edr_color_component_value(&self) -> f32 {
        unsafe {
            let screen_id: cocoa::base::id = msg_send![*self.0, screen];
            let max_edr: CGFloat = msg_send![
                screen_id,
                maximumReferenceExtendedDynamicRangeColorComponentValue
            ];
            max_edr as f32
        }
    }
}
