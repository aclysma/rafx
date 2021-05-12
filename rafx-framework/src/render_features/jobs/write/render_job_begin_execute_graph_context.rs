use crate::graph::OnBeginExecuteGraphArgs;
use crate::{DynCommandBuffer, ResourceContext};
use rafx_api::RafxDeviceContext;

pub struct RenderJobBeginExecuteGraphContext {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub command_buffer: DynCommandBuffer,
}

impl RenderJobBeginExecuteGraphContext {
    pub fn new(
        resource_context: ResourceContext,
        command_buffer: DynCommandBuffer,
    ) -> Self {
        RenderJobBeginExecuteGraphContext {
            device_context: resource_context.device_context().clone(),
            resource_context,
            command_buffer,
        }
    }

    pub fn from_on_begin_execute_graph_args(
        args: &OnBeginExecuteGraphArgs
    ) -> RenderJobBeginExecuteGraphContext {
        RenderJobBeginExecuteGraphContext::new(
            args.graph_context.resource_context().clone(),
            args.command_buffer.clone(),
        )
    }
}
