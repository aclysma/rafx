
use raw_window_handle::HasRawWindowHandle;
use raw_gl_context::GlConfig;
use super::gles20::Gles2;


pub struct GlContext {
    context: raw_gl_context::GlContext,
    gles2: Gles2
}

impl GlContext {
    pub fn new(window: &dyn HasRawWindowHandle) -> Self {
        let context = raw_gl_context::GlContext::create(window, GlConfig::default()).unwrap();
        context.make_current();
        let gles2 = Gles2::load_with(|symbol| context.get_proc_address(symbol) as *const _);
        context.make_not_current();

        GlContext {
            context,
            gles2
        }
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

            std::ffi::CString::from_raw(str as _).into_string().unwrap()
        }
    }
}
