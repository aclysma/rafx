use crate::{RenderFeaturePlugin, RendererPipelinePlugin, RendererThreadPool};
use fnv::FnvBuildHasher;
use rafx_api::{RafxCommandBuffer, RafxDeviceContext, RafxQueue};
use rafx_api::{RafxPresentableFrame, RafxResult};
use rafx_framework::graph::PreparedRenderGraph;
use rafx_framework::render_features::render_features_prelude::*;
use rafx_framework::{DynCommandBuffer, RenderResources, ResourceContext};
use std::sync::Arc;

pub struct RenderFrameJobResult;

/// The `RenderFrameJob` is responsible for the `prepare` and `write` steps of the `Renderer` pipeline.
/// This is created by `Renderer::try_create_render_job` with the results of the `extract` step.
pub struct RenderFrameJob {
    pub thread_pool: Box<dyn RendererThreadPool>,
    pub render_resources: Arc<RenderResources>,
    pub prepared_render_graph: PreparedRenderGraph,
    pub resource_context: ResourceContext,
    pub frame_packets: Vec<Box<dyn RenderFeatureFramePacket>>,
    pub render_registry: RenderRegistry,
    pub device_context: RafxDeviceContext,
    pub graphics_queue: RafxQueue,
    pub render_views: Vec<RenderView>,
    pub feature_plugins: Arc<Vec<Arc<dyn RenderFeaturePlugin>>>,
    pub pipeline_plugin: Arc<dyn RendererPipelinePlugin>,
}

impl RenderFrameJob {
    pub fn render_async(
        mut self,
        presentable_frame: RafxPresentableFrame,
    ) -> RenderFrameJobResult {
        let t0 = rafx_base::Instant::now();

        let graphics_queue = self.graphics_queue.clone();
        let result = Self::do_render_async(
            self.prepared_render_graph,
            self.resource_context,
            self.frame_packets,
            self.render_registry,
            &*self.render_resources,
            self.graphics_queue,
            self.render_views,
            self.feature_plugins,
            self.pipeline_plugin,
            &mut *self.thread_pool,
        );

        let t1 = rafx_base::Instant::now();
        log::trace!(
            "[render thread] render took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        match result {
            Ok(command_buffers) => {
                // ignore the error, we will receive it when we try to acquire the next image
                let refs: Vec<&RafxCommandBuffer> = command_buffers.iter().map(|x| &**x).collect();
                //graphics_queue.wait_for_queue_idle().unwrap();
                let _ = presentable_frame.present(&graphics_queue, &refs);
                //graphics_queue.wait_for_queue_idle().unwrap();
            }
            Err(err) => {
                log::error!("Render thread failed with error {:?}", err);
                // Pass error on to the next swapchain image acquire call
                presentable_frame.present_with_error(&graphics_queue, err);
            }
        }

        let t2 = rafx_base::Instant::now();
        log::trace!(
            "[render thread] present took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        RenderFrameJobResult {}
    }

    #[allow(clippy::too_many_arguments)]
    fn do_render_async(
        prepared_render_graph: PreparedRenderGraph,
        resource_context: ResourceContext,
        frame_packets: Vec<Box<dyn RenderFeatureFramePacket>>,
        render_registry: RenderRegistry,
        render_resources: &RenderResources,
        graphics_queue: RafxQueue,
        render_views: Vec<RenderView>,
        feature_plugins: Arc<Vec<Arc<dyn RenderFeaturePlugin>>>,
        pipeline_plugin: Arc<dyn RendererPipelinePlugin>,
        thread_pool: &mut dyn RendererThreadPool,
    ) -> RafxResult<Vec<DynCommandBuffer>> {
        let t0 = rafx_base::Instant::now();

        //
        // Prepare Jobs - everything beyond this point could be done in parallel with the main thread
        //

        let (submit_node_blocks, frame_and_submit_packets) = {
            profiling::scope!("Renderer Prepare");

            let prepare_context =
                RenderJobPrepareContext::new(resource_context.clone(), &render_resources);

            let prepare_jobs = {
                profiling::scope!("Create Prepare Jobs");
                RenderFrameJob::create_prepare_jobs(
                    &feature_plugins,
                    &prepare_context,
                    frame_packets,
                )
            };

            {
                profiling::scope!("Run Prepare Jobs");
                thread_pool.run_prepare_jobs(&prepare_jobs);
            }

            let render_view_submit_nodes = {
                profiling::scope!("Count View/Phase Submit Nodes");
                RenderFrameJob::count_render_view_phase_submit_nodes(&render_views, &prepare_jobs)
            };

            let submit_node_blocks = {
                profiling::scope!("Create Submit Node Blocks");
                thread_pool.create_submit_node_blocks(
                    &render_registry,
                    &render_view_submit_nodes,
                    &prepare_jobs,
                )
            };

            let frame_and_submit_packets =
                RenderFrameJob::take_frame_and_submit_packets(prepare_jobs);

            (submit_node_blocks, frame_and_submit_packets)
        };

        let t1 = rafx_base::Instant::now();
        log::trace!(
            "[render thread] render prepare took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        let command_buffers = {
            profiling::scope!("Renderer Write");

            let write_context =
                RenderJobWriteContext::new(resource_context.clone(), &render_resources);

            let write_jobs = {
                profiling::scope!("Create Write Jobs");
                RenderFrameJob::create_write_jobs(
                    &write_context,
                    &feature_plugins,
                    frame_and_submit_packets,
                )
            };

            let prepared_render_data = PreparedRenderData::new(&submit_node_blocks, write_jobs);

            {
                profiling::scope!("Execute Render Graph");
                prepared_render_graph.execute_graph(
                    &write_context,
                    prepared_render_data,
                    &graphics_queue,
                )?
            }
        };

        pipeline_plugin.finish_frame(render_resources);

        let t2 = rafx_base::Instant::now();
        log::trace!(
            "[render thread] execute graph took {} ms",
            (t2 - t1).as_secs_f32() * 1000.0
        );

        Ok(command_buffers)
    }

    fn create_prepare_jobs<'prepare>(
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packets: Vec<Box<dyn RenderFeatureFramePacket>>,
    ) -> Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>> {
        frame_packets
            .into_iter()
            .map(|frame_packet| {
                RenderFrameJob::create_prepare_job(features, prepare_context, frame_packet)
            })
            .collect()
    }

    fn create_prepare_job<'prepare>(
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        let feature = features.get(frame_packet.feature_index() as usize).unwrap();
        profiling::scope!(feature.feature_debug_constants().feature_name);

        assert_eq!(feature.feature_index(), frame_packet.feature_index());

        let submit_packet = {
            profiling::scope!("Allocate Submit Packet");
            feature.new_submit_packet(&frame_packet)
        };

        feature.new_prepare_job(prepare_context, frame_packet, submit_packet)
    }

    pub fn prepare_render_object_instance_chunk<'prepare>(
        prepare_job: &Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>,
        chunk_index: usize,
        chunk_size: usize,
    ) {
        let num_render_object_instances = prepare_job.num_render_object_instances();
        let start_id = chunk_index * chunk_size;
        let end_id = usize::min(start_id + chunk_size, num_render_object_instances);
        prepare_job.prepare_render_object_instance(start_id..end_id);
    }

    pub fn prepare_render_object_instance_all<'prepare>(
        prepare_job: &Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>
    ) {
        RenderFrameJob::prepare_render_object_instance_chunk(
            prepare_job,
            0,
            prepare_job.num_render_object_instances(),
        );
    }

    pub fn prepare_render_object_instance_per_view_chunk<'prepare>(
        prepare_job: &Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>,
        view_packet: &dyn RenderFeatureViewPacket,
        view_submit_packet: &dyn RenderFeatureViewSubmitPacket,
        chunk_index: usize,
        chunk_size: usize,
    ) {
        let num_render_object_instances = view_packet.num_render_object_instances();
        let start_id = chunk_index * chunk_size;
        let end_id = usize::min(start_id + chunk_size, num_render_object_instances);
        prepare_job.prepare_render_object_instance_per_view(
            view_packet,
            view_submit_packet,
            start_id..end_id,
        );
    }

    pub fn prepare_render_object_instance_per_view_all<'prepare>(
        prepare_job: &Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>,
        view_packet: &dyn RenderFeatureViewPacket,
        view_submit_packet: &dyn RenderFeatureViewSubmitPacket,
    ) {
        RenderFrameJob::prepare_render_object_instance_per_view_chunk(
            prepare_job,
            view_packet,
            view_submit_packet,
            0,
            view_packet.num_render_object_instances(),
        );
    }

    fn count_render_view_phase_submit_nodes<'prepare>(
        views: &[RenderView],
        finished_prepare_jobs: &Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>,
    ) -> RenderViewSubmitNodeCount {
        let mut render_view_submit_nodes = RenderViewSubmitNodeCount::with_capacity_and_hasher(
            views.len(),
            FnvBuildHasher::default(),
        );

        for view in views {
            render_view_submit_nodes.insert(
                view.view_index(),
                vec![0; RenderRegistry::registered_render_phase_count() as usize],
            );
        }

        for prepare_job in finished_prepare_jobs.iter() {
            profiling::scope!(prepare_job.feature_debug_constants().feature_name);

            let num_views = prepare_job.num_views();
            for view_index in 0..num_views {
                let render_view_index = prepare_job
                    .view_packet(view_index as ViewFrameIndex)
                    .view()
                    .view_index();

                let view_submit_packet =
                    prepare_job.view_submit_packet(view_index as ViewFrameIndex);

                let num_view_submit_nodes = render_view_submit_nodes
                    .get_mut(&render_view_index)
                    .unwrap();

                for render_phase_index in 0..RenderRegistry::registered_render_phase_count() {
                    num_view_submit_nodes[render_phase_index as usize] +=
                        view_submit_packet.num_submit_nodes(render_phase_index as RenderPhaseIndex);
                }
            }
        }

        render_view_submit_nodes
    }

    pub fn create_submit_node_blocks_for_view<'prepare>(
        render_registry: &RenderRegistry,
        render_view_index: &RenderViewIndex,
        num_view_submit_nodes: &Vec<usize>,
        finished_prepare_jobs: &Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>,
    ) -> Vec<ViewPhaseSubmitNodeBlock> {
        let mut submit_node_blocks = Vec::new();

        for render_phase_index in 0..RenderRegistry::registered_render_phase_count() {
            let num_submit_nodes = num_view_submit_nodes[render_phase_index as usize];
            if num_submit_nodes == 0 {
                continue;
            }

            let mut submit_node_block = ViewPhaseSubmitNodeBlock::new(
                ViewPhase {
                    view_index: *render_view_index,
                    phase_index: render_phase_index as RenderPhaseIndex,
                },
                num_submit_nodes,
            );

            for prepare_job in finished_prepare_jobs.iter() {
                profiling::scope!(prepare_job.feature_debug_constants().feature_name);

                let num_views = prepare_job.num_views();
                for view_index in 0..num_views {
                    if prepare_job
                        .view_packet(view_index as ViewFrameIndex)
                        .view()
                        .view_index()
                        != *render_view_index
                    {
                        continue;
                    }

                    let view_submit_packet =
                        prepare_job.view_submit_packet(view_index as ViewFrameIndex);

                    if let Some(feature_submit_node_block) = view_submit_packet
                        .get_submit_node_block(render_phase_index as RenderPhaseIndex)
                    {
                        for submit_node_id in 0..feature_submit_node_block.num_submit_nodes() {
                            submit_node_block.push_submit_node(
                                feature_submit_node_block
                                    .get_submit_node(submit_node_id as SubmitNodeId),
                            );
                        }
                    }
                }
            }

            submit_node_block.sort_submit_nodes(
                render_registry.submit_node_sort_function(render_phase_index as RenderPhaseIndex),
            );

            submit_node_blocks.push(submit_node_block)
        }

        submit_node_blocks
    }

    fn take_frame_and_submit_packet<'prepare>(
        prepare_job: &mut Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>
    ) -> (
        Box<dyn RenderFeatureFramePacket>,
        Box<dyn RenderFeatureSubmitPacket>,
    ) {
        let prepare_job = Arc::get_mut(prepare_job).unwrap();
        (
            prepare_job.take_frame_packet(),
            prepare_job.take_submit_packet(),
        )
    }

    fn take_frame_and_submit_packets<'prepare>(
        mut finished_prepare_jobs: Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>
    ) -> Vec<(
        Box<dyn RenderFeatureFramePacket>,
        Box<dyn RenderFeatureSubmitPacket>,
    )> {
        finished_prepare_jobs
            .iter_mut()
            .map(|prepare_job| RenderFrameJob::take_frame_and_submit_packet(prepare_job))
            .collect()
    }

    fn create_write_jobs<'write>(
        write_context: &RenderJobWriteContext<'write>,
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        frame_and_submit_packets: Vec<(
            Box<dyn RenderFeatureFramePacket>,
            Box<dyn RenderFeatureSubmitPacket>,
        )>,
    ) -> Vec<Option<Arc<dyn RenderFeatureWriteJob<'write> + 'write>>> {
        let mut write_jobs = vec![None; RenderRegistry::registered_feature_count() as usize];

        for write_job in frame_and_submit_packets.into_iter().map(|frame_packet| {
            RenderFrameJob::create_write_job(features, write_context, frame_packet)
        }) {
            let feature = write_jobs
                .get_mut(write_job.feature_index() as usize)
                .unwrap();

            assert!(feature.is_none());
            *feature = Some(write_job);
        }

        write_jobs
    }

    fn create_write_job<'write>(
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        write_context: &RenderJobWriteContext<'write>,
        frame_and_submit_packets: (
            Box<dyn RenderFeatureFramePacket>,
            Box<dyn RenderFeatureSubmitPacket>,
        ),
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        let frame_packet = frame_and_submit_packets.0;
        let submit_packet = frame_and_submit_packets.1;

        let feature = features.get(frame_packet.feature_index() as usize).unwrap();
        profiling::scope!(feature.feature_debug_constants().feature_name);

        assert_eq!(feature.feature_index(), frame_packet.feature_index());
        assert_eq!(frame_packet.feature_index(), submit_packet.feature_index());

        feature.new_write_job(write_context, frame_packet, submit_packet)
    }
}
