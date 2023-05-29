use crate::gles3::GlContext;
use crate::RafxResult;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::sync::{Arc, Mutex};

pub struct GlContextManager {
    main_context: Arc<GlContext>,
    current_context: Mutex<Option<Arc<GlContext>>>,
}

impl GlContextManager {
    pub fn new(
        display: &dyn HasRawDisplayHandle,
        window: &dyn HasRawWindowHandle
    ) -> RafxResult<GlContextManager> {
        let main_context = Arc::new(GlContext::new(display, window, None)?);
        main_context.make_current();

        Ok(GlContextManager {
            main_context: main_context.clone(),
            current_context: Mutex::new(Some(main_context)),
        })
    }

    pub fn main_context(&self) -> &Arc<GlContext> {
        &self.main_context
    }

    pub fn set_current_context(
        &self,
        new_context: Option<&Arc<GlContext>>,
    ) {
        let mut current_context = self.current_context.lock().unwrap();

        // If the context is already current, or no context is passed or set, return
        if new_context == current_context.as_ref() {
            return;
        }

        // Take the old context and make it not current
        let old_context = current_context.take();
        if let Some(old_context) = old_context {
            old_context.make_not_current();
        }

        if let Some(new_context) = new_context {
            new_context.make_current();
            *current_context = Some(new_context.clone());
        }
    }

    // Either creates a new context for the surface, or returns the main context if the given
    // surface matches the main surface.
    //
    // Caveats:
    // - The main context must never change or be invalidated
    // - Calling create_surface_context on the same window is only allowed if the previously
    //   returned context was torn down completely
    pub fn create_surface_context(
        &self,
        display: &dyn HasRawDisplayHandle,
        window: &dyn HasRawWindowHandle,
    ) -> RafxResult<Arc<GlContext>> {
        if self.main_context.window_hash() == super::gl_context::calculate_window_hash(display, window) {
            Ok(self.main_context.clone())
        } else {
            Ok(Arc::new(GlContext::new(display, window, Some(&*self.main_context))?))
        }
    }
}
