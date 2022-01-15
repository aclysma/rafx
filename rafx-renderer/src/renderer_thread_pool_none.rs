use crate::{RenderFeaturePlugin, RenderFrameJob, Renderer, RendererThreadPool};
use rafx_framework::render_features::render_features_prelude::*;
use std::sync::Arc;

/// This is a single-threaded implementation that will be used if the application does not
/// provide a `RendererThreadPool` implementation in `RendererBuilder::build`.
#[derive(Clone)]
pub(crate) struct RendererThreadPoolNone {}

impl RendererThreadPoolNone {
    pub fn new() -> Self {
        Self {}
    }
}

impl RendererThreadPool for RendererThreadPoolNone {
    fn run_view_visibility_jobs<'extract>(
        &mut self,
        view_visibility_jobs: &[Arc<ViewVisibilityJob>],
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_resource: &VisibilityResource,
    ) -> Vec<RenderViewVisibilityQuery> {
        view_visibility_jobs
            .iter()
            .map(|visibility_job: &Arc<ViewVisibilityJob>| {
                Renderer::run_view_visibility_job(
                    visibility_job,
                    extract_context,
                    visibility_resource,
                )
            })
            .collect()
    }

    fn count_render_features_render_objects<'extract>(
        &mut self,
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
    ) {
        features
            .iter()
            .for_each(|feature: &Arc<dyn RenderFeaturePlugin>| {
                Renderer::count_render_feature_render_objects(
                    feature,
                    extract_context,
                    visibility_results,
                )
            });
    }

    fn create_extract_jobs<'extract>(
        &mut self,
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: Vec<RenderViewVisibilityQuery>,
    ) -> Vec<Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>> {
        features
            .iter()
            .filter_map(|feature: &Arc<dyn RenderFeaturePlugin>| {
                Renderer::create_extract_job(feature, extract_context, &visibility_results)
            })
            .collect()
    }

    fn run_extract_jobs<'extract>(
        &mut self,
        extract_jobs: &Vec<Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>>,
        visibility_resource: &VisibilityResource,
    ) {
        extract_jobs
            .iter()
            .for_each(|extract_job: &Arc<dyn RenderFeatureExtractJob>| {
                profiling::scope!(extract_job.feature_debug_constants().feature_name);

                extract_job.begin_per_frame_extract();

                Renderer::extract_render_object_instance_all(extract_job, visibility_resource);

                (0..extract_job.num_views())
                    .into_iter()
                    .for_each(|view_index| {
                        let view_packet = extract_job.view_packet(view_index as ViewFrameIndex);

                        Renderer::extract_render_object_instance_per_view_all(
                            extract_job,
                            visibility_resource,
                            view_packet,
                        );

                        extract_job.end_per_view_extract(view_packet);
                    });

                extract_job.end_per_frame_extract();
            });
    }

    fn run_prepare_jobs<'prepare>(
        &mut self,
        prepare_jobs: &Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>,
    ) {
        prepare_jobs
            .iter()
            .for_each(|prepare_job: &Arc<dyn RenderFeaturePrepareJob>| {
                profiling::scope!(prepare_job.feature_debug_constants().feature_name);

                prepare_job.begin_per_frame_prepare();

                RenderFrameJob::prepare_render_object_instance_all(prepare_job);

                (0..prepare_job.num_views())
                    .into_iter()
                    .for_each(|view_index| {
                        let view_packet = prepare_job.view_packet(view_index as ViewFrameIndex);
                        let view_submit_packet =
                            prepare_job.view_submit_packet(view_index as ViewFrameIndex);

                        RenderFrameJob::prepare_render_object_instance_per_view_all(
                            prepare_job,
                            view_packet,
                            view_submit_packet,
                        );

                        prepare_job.end_per_view_prepare(view_packet, view_submit_packet);
                    });

                prepare_job.end_per_frame_prepare();
            });
    }

    fn create_submit_node_blocks<'prepare>(
        &mut self,
        render_registry: &RenderRegistry,
        render_view_submit_nodes: &RenderViewSubmitNodeCount,
        finished_prepare_jobs: &Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>,
    ) -> SubmitNodeBlocks {
        render_view_submit_nodes
            .iter()
            .flat_map(
                |(render_view_index, num_view_submit_nodes): (&RenderViewIndex, &Vec<usize>)| {
                    RenderFrameJob::create_submit_node_blocks_for_view(
                        render_registry,
                        render_view_index,
                        num_view_submit_nodes,
                        finished_prepare_jobs,
                    )
                },
            )
            .collect::<Vec<_>>()
            .into_iter()
            .map(|submit_node_block| (submit_node_block.view_phase().clone(), submit_node_block))
            .collect()
    }

    fn clone_to_box(&mut self) -> Box<dyn RendererThreadPool> {
        Box::new(self.clone())
    }
}
