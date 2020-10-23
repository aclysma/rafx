use crate::game_renderer::{GameRenderer, GameRendererInner};
use renderer::nodes::{PrepareJobSet, FramePacket, RenderView, RenderRegistry};
use crate::render_contexts::{
    RenderJobPrepareContext, RenderJobWriteContext, RenderJobWriteContextFactory,
};
use renderer::assets::graph::RenderGraphExecutor;
use renderer::assets::resources::{DynResourceAllocatorSet, DynCommandWriterAllocator};
use renderer::vulkan::{VkDeviceContext, FrameInFlight};
use std::sync::MutexGuard;
use ash::prelude::VkResult;
use ash::vk;
use crate::game_renderer::render_graph::RenderGraphExecuteContext;

pub struct RenderFrameJob {
    pub game_renderer: GameRenderer,
    pub prepare_job_set: PrepareJobSet<RenderJobPrepareContext, RenderJobWriteContext>,
    pub render_graph: RenderGraphExecutor<RenderGraphExecuteContext>,
    pub dyn_resource_allocator_set: DynResourceAllocatorSet,
    pub dyn_command_writer_allocator: DynCommandWriterAllocator,
    pub frame_packet: FramePacket,
    pub main_view: RenderView,
    pub render_registry: RenderRegistry,
    pub device_context: VkDeviceContext,
    pub frame_in_flight: FrameInFlight,
}

impl RenderFrameJob {
    pub fn render_async(self) {
        // let t0 = std::time::Instant::now();
        let guard = self.game_renderer.inner.lock().unwrap();

        let result = Self::do_render_async(
            guard,
            self.prepare_job_set,
            self.render_graph,
            self.dyn_resource_allocator_set,
            self.dyn_command_writer_allocator,
            self.frame_packet,
            self.main_view,
            self.render_registry,
            self.device_context,
            // self.opaque_pipeline_info,
            // self.imgui_pipeline_info,
            self.frame_in_flight.present_index() as usize,
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
        mut guard: MutexGuard<GameRendererInner>,
        prepare_job_set: PrepareJobSet<RenderJobPrepareContext, RenderJobWriteContext>,
        render_graph: RenderGraphExecutor<RenderGraphExecuteContext>,
        dyn_resource_allocator_set: DynResourceAllocatorSet,
        dyn_command_writer_allocator: DynCommandWriterAllocator,
        frame_packet: FramePacket,
        main_view: RenderView,
        render_registry: RenderRegistry,
        device_context: VkDeviceContext,
        present_index: usize,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let t0 = std::time::Instant::now();
        //let mut guard = self.inner.lock().unwrap();
        let swapchain_resources = guard.swapchain_resources.as_mut().unwrap();

        let mut command_writer = dyn_command_writer_allocator.allocate_writer(
            device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            vk::CommandPoolCreateFlags::TRANSIENT,
            0,
        )?;

        //
        // Prepare Jobs - everything beyond this point could be done in parallel with the main thread
        //
        let prepare_context = RenderJobPrepareContext::new(dyn_resource_allocator_set);
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
        // Write Jobs - called from within renderpasses for now
        //
        let write_context_factory =
            RenderJobWriteContextFactory::new(device_context, prepare_context.dyn_resource_lookups);

        let mut graph_context = RenderGraphExecuteContext {
            prepared_render_data,
            view: main_view,
            write_context_factory,
            command_writer,
        };

        let mut command_buffers =
            render_graph.execute_graph(&dyn_command_writer_allocator, &mut graph_context)?;

        let prepared_render_data = graph_context.prepared_render_data;
        let main_view = graph_context.view;
        let write_context_factory = graph_context.write_context_factory;
        let mut command_writer = graph_context.command_writer;

/*
                let mut command_buffers = vec![];
                //
                // Opaque renderpass
                //
                log::trace!("opaque_renderpass update");
                let command_buffer = swapchain_resources.opaque_renderpass.update(
                    &*prepared_render_data,
                    &main_view,
                    &write_context_factory,
                    &mut command_writer,
                )?;
                command_buffers.push(command_buffer);

                //
                // Debug Renderpass
                //
                let descriptor_set_per_pass = swapchain_resources
                    .debug_material_per_frame_data
                    .descriptor_set()
                    .get();
                log::trace!("msaa_renderpass update");

                let command_buffer = swapchain_resources
                    .msaa_renderpass
                    .update(&mut command_writer)?;
                command_buffers.push(command_buffer);

                //
                // bloom extract
                //
                let descriptor_set_per_pass = swapchain_resources
                    .bloom_extract_material_dyn_set
                    .descriptor_set()
                    .get();
                log::trace!("bloom_extract_renderpass update");

                let command_buffer = swapchain_resources
                    .bloom_extract_renderpass
                    .update(descriptor_set_per_pass, &mut command_writer)?;
                command_buffers.push(command_buffer);

                //
                // bloom blur
                //
                log::trace!("bloom_blur_renderpass update");
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[0]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[1]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[0]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[1]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[0]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[1]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[0]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[1]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[0]);
                command_buffers.push(swapchain_resources.bloom_blur_renderpass.command_buffers[1]);

                //
                // bloom combine
                //
                let descriptor_set_per_pass = swapchain_resources
                    .bloom_combine_material_dyn_set
                    .descriptor_set()
                    .get();
                log::trace!("bloom_combine_renderpass update");

                let command_buffer = swapchain_resources.bloom_combine_renderpass.update(
                    present_index,
                    descriptor_set_per_pass,
                    &mut command_writer,
                )?;
                command_buffers.push(command_buffer);

                //
                // imgui
                //
                let command_buffer = swapchain_resources.ui_renderpass.update(
                    present_index,
                    &*prepared_render_data,
                    &main_view,
                    &write_context_factory,
                    &mut command_writer,
                )?;
                command_buffers.push(command_buffer);

                let t2 = std::time::Instant::now();
                log::trace!(
                    "[async] render write took {} ms",
                    (t2 - t1).as_secs_f32() * 1000.0
                );
*/
        Ok(command_buffers)
    }
}
