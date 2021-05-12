use crate::{RenderResources, ResourceContext};
use rafx_api::RafxDeviceContext;

/// Holds references to resources valid for the entirety of the `write` step as
/// represented by the `'write` lifetime. `RenderFeatureWriteJob`s should cache
/// any resources needed from the `RenderJobWriteContext` during their `new` function.
pub struct RenderJobWriteContext<'write> {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub render_resources: &'write RenderResources,
}

impl<'write> RenderJobWriteContext<'write> {
    pub fn new(
        resource_context: ResourceContext,
        render_resources: &'write RenderResources,
    ) -> Self {
        RenderJobWriteContext {
            device_context: resource_context.device_context().clone(),
            resource_context,
            render_resources,
        }
    }
}
