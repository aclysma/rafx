use crate::game_renderer::render_graph::RenderGraphUserContext;
use crate::game_renderer::GameRenderer;
use crate::render_contexts::{RenderJobPrepareContext, RenderJobWriteContext};
use ash::prelude::VkResult;
use ash::vk;
use renderer::graph::RenderGraphExecutor;
use renderer::nodes::{FramePacket, PrepareJobSet, RenderRegistry, RenderView};
use renderer::resources::ResourceContext;
use renderer::vulkan::{FrameInFlight, VkDeviceContext};

pub struct RenderFrameJob {
    pub game_renderer: GameRenderer,
    pub prepare_job_set: PrepareJobSet<RenderJobPrepareContext, RenderJobWriteContext>,
    pub render_graph: RenderGraphExecutor<RenderGraphUserContext>,
    pub resource_context: ResourceContext,
    pub frame_packet: FramePacket,
    pub main_view: RenderView,
    pub shadow_map_render_views: Vec<RenderView>,
    pub render_registry: RenderRegistry,
    pub device_context: VkDeviceContext,
}

impl RenderFrameJob {
    pub fn render_async(
        self,
        frame_in_flight: FrameInFlight,
    ) {
        let t0 = std::time::Instant::now();
        let result = Self::do_render_async(
            self.prepare_job_set,
            self.render_graph,
            self.resource_context,
            self.frame_packet,
            self.main_view,
            self.shadow_map_render_views,
            self.render_registry,
            self.device_context,
        );

        let t1 = std::time::Instant::now();
        log::trace!(
            "[render thread] render took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        match result {
            Ok(command_buffers) => {
                // ignore the error, we will receive it when we try to acquire the next image
                let _ = frame_in_flight.present(command_buffers.as_slice());
            }
            Err(err) => {
                log::error!("Render thread failed with error {:?}", err);
                // Pass error on to the next swapchain image acquire call
                frame_in_flight.cancel_present(Err(err));
            }
        }

        let t2 = std::time::Instant::now();
        log::trace!(
            "[render thread] present took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn do_render_async(
        prepare_job_set: PrepareJobSet<RenderJobPrepareContext, RenderJobWriteContext>,
        render_graph: RenderGraphExecutor<RenderGraphUserContext>,
        resource_context: ResourceContext,
        frame_packet: FramePacket,
        main_view: RenderView,
        shadow_map_render_views: Vec<RenderView>,
        render_registry: RenderRegistry,
        device_context: VkDeviceContext,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let t0 = std::time::Instant::now();

        //
        // Prepare Jobs - everything beyond this point could be done in parallel with the main thread
        //
        let prepared_render_data = {
            profiling::scope!("Renderer Prepare");

            let mut views = Vec::default();
            views.push(&main_view);
            for shadow_map_view in &shadow_map_render_views {
                views.push(shadow_map_view);
            }

            let prepare_context =
                RenderJobPrepareContext::new(device_context.clone(), resource_context.clone());
            prepare_job_set.prepare(&prepare_context, &frame_packet, &views, &render_registry)
        };
        let t1 = std::time::Instant::now();
        log::trace!(
            "[render thread] render prepare took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        //
        // Write Jobs - triggered by the render graph
        //
        let graph_context = RenderGraphUserContext {
            prepared_render_data,
        };

        let command_buffers = {
            profiling::scope!("Renderer Execute Graph");
            render_graph.execute_graph(&graph_context)?
        };
        let t2 = std::time::Instant::now();
        log::trace!(
            "[render thread] execute graph took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        Ok(command_buffers)
    }
}
