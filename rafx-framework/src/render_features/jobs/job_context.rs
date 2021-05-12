use crate::render_features::render_features_prelude::*;

/// A `JobContext` is definable by each `RenderFeature`. It is used to bundle any expensive
/// work, like acquiring locks or other resources, into fewer call-sites for any work occurring on
/// the same thread. The `JobContext` will **not** be shared between threads.

/// Use `DefaultJobContext` if the `RenderFeature` does not need to acquire resources.
pub struct DefaultJobContext {}

impl DefaultJobContext {
    /// Returns `{}`.
    pub fn new() -> Self {
        Self {}
    }
}

/// Use `RenderObjectsJobContext` if the `RenderFeature` only needs to lock a `RenderObjectsMap`.
pub struct RenderObjectsJobContext<'job, RenderObjectStaticDataT> {
    pub render_objects: RwLockReadGuard<'job, RenderObjectsMap<RenderObjectStaticDataT>>,
}

impl<'job, RenderObjectStaticDataT> RenderObjectsJobContext<'job, RenderObjectStaticDataT> {
    pub fn new(
        render_objects: RwLockReadGuard<'job, RenderObjectsMap<RenderObjectStaticDataT>>
    ) -> Self {
        RenderObjectsJobContext { render_objects }
    }
}
