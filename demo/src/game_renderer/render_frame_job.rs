use crate::game_renderer::GameRenderer;
use renderer::nodes::{PrepareJobSet, FramePacket, RenderView, RenderRegistry};
use crate::render_contexts::{
    RenderJobPrepareContext, RenderJobWriteContext, RenderJobWriteContextFactory,
};
use renderer::assets::graph::RenderGraphExecutor;
use renderer::vulkan::{VkDeviceContext, FrameInFlight};
use ash::prelude::VkResult;
use ash::vk;
use crate::game_renderer::render_graph::RenderGraphUserContext;
use renderer::assets::{ResourceContext};

pub struct RenderFrameJob {
    pub game_renderer: GameRenderer,
    pub prepare_job_set: PrepareJobSet<RenderJobPrepareContext, RenderJobWriteContext>,
    pub render_graph: RenderGraphExecutor<RenderGraphUserContext>,
    pub resource_context: ResourceContext,
    pub frame_packet: FramePacket,
    pub main_view: RenderView,
    pub render_registry: RenderRegistry,
    pub device_context: VkDeviceContext,
    pub frame_in_flight: FrameInFlight,
}

impl RenderFrameJob {
    pub fn render_async(self) {
        // let t0 = std::time::Instant::now();
        //let guard = self.game_renderer.inner.lock().unwrap();

        let result = Self::do_render_async(
            //guard,
            self.prepare_job_set,
            self.render_graph,
            self.resource_context,
            self.frame_packet,
            self.main_view,
            self.render_registry,
            self.device_context,
            //self.frame_in_flight.present_index() as usize,
        );

        let t1 = std::time::Instant::now();
        //log::info!("[async] render took {} ms", (t1 - t0).as_secs_f32() * 1000.0);

        match result {
            Ok(command_buffers) => {
                // ignore the error, we will receive it when we try to acquire the next image
                let _ = self.frame_in_flight.present(command_buffers.as_slice());
            }
            Err(err) => {
                log::error!("Render thread failed with error {:?}", err);
                // Pass error on to the next swapchain image acquire call
                self.frame_in_flight.cancel_present(Err(err));
            }
        }

        let t2 = std::time::Instant::now();
        log::trace!(
            "[async] present took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn do_render_async(
        //mut guard: MutexGuard<GameRendererInner>,
        prepare_job_set: PrepareJobSet<RenderJobPrepareContext, RenderJobWriteContext>,
        render_graph: RenderGraphExecutor<RenderGraphUserContext>,
        // dyn_resource_allocator_set_provider: DynResourceAllocatorSetProvider,
        // dyn_command_writer_allocator: DynCommandWriterAllocator,
        resource_context: ResourceContext,
        frame_packet: FramePacket,
        main_view: RenderView,
        render_registry: RenderRegistry,
        device_context: VkDeviceContext,
        //present_index: usize,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let t0 = std::time::Instant::now();

        //
        // Prepare Jobs - everything beyond this point could be done in parallel with the main thread
        //
        let prepare_context = RenderJobPrepareContext::new(resource_context.clone());
        let prepared_render_data = prepare_job_set.prepare(
            &prepare_context,
            &frame_packet,
            &[&main_view],
            &render_registry,
        );
        let t1 = std::time::Instant::now();
        log::trace!(
            "[async] render prepare took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        //
        // Write Jobs - triggered by the render graph
        //
        let write_context_factory =
            RenderJobWriteContextFactory::new(device_context, resource_context.clone());

        let graph_context = RenderGraphUserContext {
            prepared_render_data,
            write_context_factory,
        };

        let command_buffers = render_graph.execute_graph(&graph_context)?;

        Ok(command_buffers)
    }
}
