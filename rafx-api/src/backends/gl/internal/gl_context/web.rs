use raw_window_handle::HasRawWindowHandle;
use web_sys::WebGlRenderingContext;

pub struct GlContext {
    context: WebGlRenderingContext
}

impl GlContext {
    pub fn new(window: &dyn HasRawWindowHandle) -> Self {
        use wasm_bindgen::JsCast;
        let handle = if let raw_window_handle::RawWindowHandle::Web(handle) = window.raw_window_handle() {
            Some(handle.id)
        } else {
            None
        }.unwrap();

        let canvas: web_sys::HtmlCanvasElement = web_sys::window()
            .and_then(|win| win.document())
            .expect("Cannot get document")
            .query_selector(&format!("canvas[data-raw-handle=\"{}\"]", handle))
            .expect("Cannot query for canvas")
            .expect("Canvas is not found")
            .dyn_into()
            .expect("Failed to downcast to canvas type");

        let context = canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .unwrap();

        GlContext {
            context
        }
    }

    pub fn context(&self) -> &WebGlRenderingContext {
        &self.context
    }

    pub fn make_current(&self) {
        // Web does not support multiple threads so this is irrelevant
    }

    pub fn make_not_current(&self) {
        // Web does not support multiple threads so this is irrelevant
    }

    pub fn swap_buffers(&self) {
        // Web swaps the buffers for us so this is irrelevant
    }

    pub fn gl_viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        self.context.viewport(x, y, width, height)
    }

    pub fn gl_clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        self.context.clear_color(r, g, b, a);
    }

    pub fn gl_clear(&self, mask: u32) {
        self.context.clear(mask);
    }

    pub fn gl_get_integerv(&self, pname: u32) -> i32 {
        let value = self.context.get_parameter(pname).unwrap();
        value.as_f64() as i32
    }

    pub fn gl_get_string(&self, pname: u32) -> String {
        let value = self.context.get_parameter(pname).unwrap();
        value.as_string()
    }
}