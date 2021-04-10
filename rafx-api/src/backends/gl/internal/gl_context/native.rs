
use raw_window_handle::HasRawWindowHandle;
use raw_gl_context::GlConfig;
use super::gles20::Gles2;
use fnv::FnvHasher;
use std::hash::{Hasher, Hash};
use super::WindowHash;

pub struct GlContext {
    context: raw_gl_context::GlContext,
    gles2: Gles2,
    window_hash: WindowHash,
}

impl PartialEq for GlContext {
    fn eq(&self, other: &Self) -> bool {
        self.window_hash == other.window_hash
    }
}

impl GlContext {
    pub fn new(window: &dyn HasRawWindowHandle, share: Option<&GlContext>) -> Self {
        let window_hash = super::calculate_window_hash(window);

        let context = raw_gl_context::GlContext::create(window, GlConfig::default(), share.map(|x| x.context())).unwrap();
        context.make_current();
        let gles2 = Gles2::load_with(|symbol| context.get_proc_address(symbol) as *const _);
        context.make_not_current();

        GlContext {
            context,
            gles2,
            window_hash
        }
    }

    pub fn window_hash(&self) -> WindowHash {
        self.window_hash
    }

    pub fn context(&self) -> &raw_gl_context::GlContext {
        &self.context
    }

    pub fn gles2(&self) -> &Gles2 {
        &self.gles2
    }

    pub fn make_current(&self) {
        self.context.make_current();
    }

    pub fn make_not_current(&self) {
        self.context.make_not_current();
    }

    pub fn swap_buffers(&self) {
        self.context.swap_buffers();
    }

    pub fn gl_viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            self.gles2.Viewport(x, y, width, height)
        }
    }

    pub fn gl_clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        unsafe {
            self.gles2.ClearColor(r, g, b, a);
        }
    }

    pub fn gl_clear(&self, mask: u32) {
        unsafe {
            self.gles2.Clear(mask);
        }
    }

    pub fn gl_get_integerv(&self, pname: u32) -> i32 {
        unsafe {
            let mut value = 0;
            self.gles2.GetIntegerv(pname, &mut value);
            value
        }
    }

    pub fn gl_get_string(&self, pname: u32) -> String {
        unsafe {
            let str = self.gles2.GetString(pname);
            if str.is_null() {
                return "".to_string();
            }

            std::ffi::CStr::from_ptr(str as _).to_str().unwrap().to_string()
        }
    }

    pub fn gl_finish(&self) {
        unsafe {
            self.gles2.Finish();
        }
    }
}
