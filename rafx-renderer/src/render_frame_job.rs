use super::Renderer;
use super::RendererPlugin;
use rafx_api::{RafxCommandBuffer, RafxDeviceContext, RafxQueue};
use rafx_api::{RafxPresentableFrame, RafxResult};
use rafx_framework::graph::PreparedRenderGraph;
use rafx_framework::nodes::{
    FramePacket, PrepareJobSet, RenderJobPrepareContext, RenderRegistry, RenderView,
};
use rafx_framework::{DynCommandBuffer, RenderResources, ResourceContext};
use std::sync::Arc;

pub struct RenderFrameJobResult;

pub struct RenderFrameJob {
    pub renderer: Renderer,
    pub prepare_job_set: PrepareJobSet,
    pub prepared_render_graph: PreparedRenderGraph,
    pub resource_context: ResourceContext,
    pub frame_packet: FramePacket,
    pub render_registry: RenderRegistry,
    pub device_context: RafxDeviceContext,
    pub graphics_queue: RafxQueue,
    pub plugins: Arc<Vec<Box<dyn RendererPlugin>>>,
    pub render_views: Vec<RenderView>,
}

impl RenderFrameJob {
    pub fn render_async(
        self,
        presentable_frame: RafxPresentableFrame,
        render_resources: &RenderResources,
    ) -> RenderFrameJobResult {
        let t0 = std::time::Instant::now();
        let result = Self::do_render_async(
            self.prepare_job_set,
            self.prepared_render_graph,
            self.resource_context,
            self.frame_packet,
            self.render_registry,
            render_resources,
            self.graphics_queue,
            self.plugins,
            self.render_views,
        );

        let t1 = std::time::Instant::now();
        log::trace!(
            "[render thread] render took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        match result {
            Ok(command_buffers) => {
                // ignore the error, we will receive it when we try to acquire the next image
                let graphics_queue = self.renderer.graphics_queue();

                let refs: Vec<&RafxCommandBuffer> = command_buffers.iter().map(|x| &**x).collect();
                let _ = presentable_frame.present(graphics_queue, &refs);
            }
            Err(err) => {
                log::error!("Render thread failed with error {:?}", err);
                // Pass error on to the next swapchain image acquire call
                let graphics_queue = self.renderer.graphics_queue();
                presentable_frame.present_with_error(graphics_queue, err);
            }
        }

        let t2 = std::time::Instant::now();
        log::trace!(
            "[render thread] present took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        RenderFrameJobResult {}
    }

    #[allow(clippy::too_many_arguments)]
    fn do_render_async(
        prepare_job_set: PrepareJobSet,
        prepared_render_graph: PreparedRenderGraph,
        resource_context: ResourceContext,
        frame_packet: FramePacket,
        render_registry: RenderRegistry,
        render_resources: &RenderResources,
        graphics_queue: RafxQueue,
        _plugins: Arc<Vec<Box<dyn RendererPlugin>>>,
        render_views: Vec<RenderView>,
    ) -> RafxResult<Vec<DynCommandBuffer>> {
        let t0 = std::time::Instant::now();

        //
        // Prepare Jobs - everything beyond this point could be done in parallel with the main thread
        //
        let prepared_render_data = {
            profiling::scope!("Renderer Prepare");

            let prepare_context =
                RenderJobPrepareContext::new(resource_context.clone(), &render_resources);

            prepare_job_set.prepare(
                &prepare_context,
                &frame_packet,
                &render_views,
                &render_registry,
            )
        };
        let t1 = std::time::Instant::now();
        log::trace!(
            "[render thread] render prepare took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        let command_buffers = {
            profiling::scope!("Renderer Execute Graph");
            prepared_render_graph.execute_graph(prepared_render_data, &graphics_queue)?
        };
        let t2 = std::time::Instant::now();
        log::trace!(
            "[render thread] execute graph took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        Ok(command_buffers)
    }
}
