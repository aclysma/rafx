use crate::{RenderResources, ResourceContext};
use rafx_api::RafxDeviceContext;

/// Holds references to resources valid for the entirety of the `prepare` step as
/// represented by the `'prepare` lifetime. `RenderFeaturePrepareJob`s should cache
/// any resources needed from the `RenderJobPrepareContext` during their `new` function.
#[derive(Clone)]
pub struct RenderJobPrepareContext<'prepare> {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub render_resources: &'prepare RenderResources,
}

impl<'prepare> RenderJobPrepareContext<'prepare> {
    pub fn new(
        resource_context: ResourceContext,
        render_resources: &'prepare RenderResources,
    ) -> Self {
        RenderJobPrepareContext {
            device_context: resource_context.device_context().clone(),
            resource_context,
            render_resources,
        }
    }
}
