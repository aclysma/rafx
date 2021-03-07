mod extract;
pub use extract::*;

mod prepare;
pub use prepare::*;

mod write;
pub use write::*;

use crate::graph::{OnBeginExecuteGraphArgs, VisitRenderpassNodeArgs};
use crate::{DynCommandBuffer, GraphicsPipelineRenderTargetMeta, RenderResources, ResourceContext};
use rafx_api::RafxDeviceContext;
use rafx_base::memory::force_to_static_lifetime;

pub type ExtractResources<'a> = rafx_base::resource_ref_map::ResourceRefMap<'a>;

pub struct RenderJobExtractContext<'a> {
    pub extract_resources: &'a ExtractResources<'a>,
    pub render_resources: &'a RenderResources,
}

impl<'a> RenderJobExtractContext<'a> {
    pub fn new(
        extract_resources: &'a ExtractResources<'a>,
        render_resources: &'a RenderResources,
    ) -> Self {
        RenderJobExtractContext {
            extract_resources,
            render_resources,
        }
    }
}

pub struct RenderJobPrepareContext {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub render_resources: &'static RenderResources,
}

impl RenderJobPrepareContext {
    pub fn new<'a>(
        resource_context: ResourceContext,
        render_resources: &'a RenderResources,
    ) -> Self {
        RenderJobPrepareContext {
            device_context: resource_context.device_context().clone(),
            resource_context,
            render_resources: unsafe { force_to_static_lifetime(render_resources) },
        }
    }
}

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

pub struct RenderJobWriteContext {
    pub device_context: RafxDeviceContext,
    pub resource_context: ResourceContext,
    pub command_buffer: DynCommandBuffer,
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
}

impl RenderJobWriteContext {
    pub fn new(
        resource_context: ResourceContext,
        command_buffer: DynCommandBuffer,
        render_target_meta: GraphicsPipelineRenderTargetMeta,
    ) -> Self {
        RenderJobWriteContext {
            device_context: resource_context.device_context().clone(),
            resource_context,
            command_buffer,
            render_target_meta,
        }
    }

    pub fn from_graph_visit_render_pass_args(
        args: &VisitRenderpassNodeArgs
    ) -> RenderJobWriteContext {
        RenderJobWriteContext::new(
            args.graph_context.resource_context().clone(),
            args.command_buffer.clone(),
            args.render_target_meta.clone(),
        )
    }
}
