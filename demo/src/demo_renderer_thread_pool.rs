use crate::features::mesh::MeshRenderFeature;
use crate::features::sprite::SpriteRenderFeature;
use bevy_tasks::prelude::*;
use bevy_tasks::{TaskPool, TaskPoolBuilder};
use crossbeam_channel::{bounded, unbounded};
use rafx::framework::render_features::render_features_prelude::*;
use rafx::render_feature_renderer_prelude::RenderFeaturePlugin;
use rafx::renderer::{RenderFrameJob, Renderer, RendererThreadPool};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct DemoRendererThreadPool {
    task_pool: TaskPool,
    feature_parallelism: Arc<HashMap<RenderFeatureIndex, ParallelChunkSizes>>,
}

impl DemoRendererThreadPool {
    pub fn new() -> Self {
        // NOTE(dvd): This is just one way to control the degree of parallelism.
        // Other implementations of the `RendererThreadPool` could decide to assign
        // a `cost` to each element of a given feature and only create a new task
        // when the `cost` exceeds some threshold.
        let mut feature_parallelism = HashMap::new();
        feature_parallelism.insert(
            MeshRenderFeature::feature_index(),
            ParallelChunkSizes::default()
                .extract_chunk_size(256)
                .prepare_chunk_size(128)
                .prepare_per_view_chunk_size(256),
        );
        feature_parallelism.insert(
            SpriteRenderFeature::feature_index(),
            ParallelChunkSizes::default().extract_chunk_size(5000),
        );

        Self {
            task_pool: TaskPoolBuilder::new().build(),
            feature_parallelism: Arc::new(feature_parallelism),
        }
    }

    pub fn extract_render_object_instance_num_chunks<'extract>(
        &self,
        extract_job: &Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>,
    ) -> Option<(usize, usize)> {
        if let Some(chunk_size) = self
            .feature_parallelism
            .get(&extract_job.feature_index())
            .and_then(|params| params.extract_chunk_size)
        {
            let num_render_object_instances = extract_job.num_render_object_instances();
            let num_chunks =
                f32::ceil((num_render_object_instances as f32) / (chunk_size as f32)) as usize;
            Some((chunk_size, num_chunks))
        } else {
            None
        }
    }

    pub fn extract_render_object_instance_per_view_num_chunks<'extract>(
        &self,
        extract_job: &Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>,
        view_packet: &dyn RenderFeatureViewPacket,
    ) -> Option<(usize, usize)> {
        if let Some(chunk_size) = self
            .feature_parallelism
            .get(&extract_job.feature_index())
            .and_then(|params| params.extract_per_view_chunk_size)
        {
            let num_render_object_instances = view_packet.num_render_object_instances();
            let num_chunks =
                f32::ceil((num_render_object_instances as f32) / (chunk_size as f32)) as usize;
            Some((chunk_size, num_chunks))
        } else {
            None
        }
    }

    pub fn prepare_render_object_instance_num_chunks<'prepare>(
        &self,
        prepare_job: &Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>,
    ) -> Option<(usize, usize)> {
        if let Some(chunk_size) = self
            .feature_parallelism
            .get(&prepare_job.feature_index())
            .and_then(|params| params.prepare_chunk_size)
        {
            let num_render_object_instances = prepare_job.num_render_object_instances();
            let num_chunks =
                f32::ceil((num_render_object_instances as f32) / (chunk_size as f32)) as usize;
            Some((chunk_size, num_chunks))
        } else {
            None
        }
    }

    pub fn prepare_render_object_instance_per_view_num_chunks<'prepare>(
        &self,
        prepare_job: &Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>,
        view_packet: &dyn RenderFeatureViewPacket,
    ) -> Option<(usize, usize)> {
        if let Some(chunk_size) = self
            .feature_parallelism
            .get(&prepare_job.feature_index())
            .and_then(|params| params.prepare_per_view_chunk_size)
        {
            let num_render_object_instances = view_packet.num_render_object_instances();
            let num_chunks =
                f32::ceil((num_render_object_instances as f32) / (chunk_size as f32)) as usize;
            Some((chunk_size, num_chunks))
        } else {
            None
        }
    }
}

impl RendererThreadPool for DemoRendererThreadPool {
    fn run_view_visibility_jobs<'extract>(
        &mut self,
        view_visibility_jobs: &[Arc<ViewVisibilityJob>],
        extract_context: &RenderJobExtractContext<'extract>,
    ) -> Vec<RenderViewVisibilityQuery> {
        view_visibility_jobs.par_chunk_map(&self.task_pool, 1, |visibility_job| {
            Renderer::run_view_visibility_job(&visibility_job[0], extract_context)
        })
    }

    fn count_render_features_render_objects<'extract>(
        &mut self,
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
    ) {
        features.par_chunk_map(&self.task_pool, 1, |feature| {
            Renderer::count_render_feature_render_objects(
                &feature[0],
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
        let (sender, receiver) = bounded(features.len());

        self.task_pool.scope(|scope| {
            for feature in features {
                let visibility_results = &visibility_results;
                let sender = sender.clone();
                scope.spawn(async move {
                    if let Some(extract_job) =
                        Renderer::create_extract_job(&feature, extract_context, visibility_results)
                    {
                        sender.send(extract_job).unwrap()
                    }
                });
            }
        });

        std::mem::drop(sender);
        receiver.iter().collect()
    }

    fn run_extract_jobs<'extract>(
        &mut self,
        extract_jobs: &Vec<Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>>,
    ) {
        extract_jobs.par_chunk_map(&self.task_pool, 1, |extract_job| {
            let extract_job = &extract_job[0];
            profiling::scope!(extract_job.feature_debug_constants().feature_name);

            extract_job.begin_per_frame_extract();

            if let Some((chunk_size, num_chunks)) =
                self.extract_render_object_instance_num_chunks(extract_job)
            {
                self.task_pool.scope(|scope| {
                    for chunk_index in 0..num_chunks {
                        scope.spawn(async move {
                            Renderer::extract_render_object_instance_chunk(
                                extract_job,
                                chunk_index,
                                chunk_size,
                            );
                        });
                    }
                });
            } else {
                Renderer::extract_render_object_instance_all(extract_job);
            }

            self.task_pool.scope(|scope| {
                let thread_pool = &self;
                let task_pool = &self.task_pool;
                for view_index in 0..extract_job.num_views() {
                    scope.spawn(async move {
                        let view_packet = extract_job.view_packet(view_index as ViewFrameIndex);

                        if let Some((chunk_size, num_chunks)) = thread_pool
                            .extract_render_object_instance_per_view_num_chunks(
                                extract_job,
                                view_packet,
                            )
                        {
                            task_pool.scope(|scope| {
                                for chunk_index in 0..num_chunks {
                                    scope.spawn(async move {
                                        Renderer::extract_render_object_instance_per_view_chunk(
                                            extract_job,
                                            view_packet,
                                            chunk_index,
                                            chunk_size,
                                        );
                                    });
                                }
                            });
                        } else {
                            Renderer::extract_render_object_instance_per_view_all(
                                extract_job,
                                view_packet,
                            );
                        }

                        extract_job.end_per_view_extract(view_packet);
                    })
                }
            });

            extract_job.end_per_frame_extract();
        });
    }

    fn run_prepare_jobs<'prepare>(
        &mut self,
        prepare_jobs: &Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>,
    ) {
        prepare_jobs.par_chunk_map(&self.task_pool, 1, |prepare_job| {
            let prepare_job = &prepare_job[0];
            profiling::scope!(prepare_job.feature_debug_constants().feature_name);

            prepare_job.begin_per_frame_prepare();

            if let Some((chunk_size, num_chunks)) = self.prepare_render_object_instance_num_chunks(prepare_job)
            {
                self.task_pool.scope(|scope| {
                    for chunk_index in 0..num_chunks {
                        scope.spawn(async move {
                            RenderFrameJob::prepare_render_object_instance_chunk(
                                prepare_job,
                                chunk_index,
                                chunk_size,
                            );
                        });
                    }
                });
            } else {
                RenderFrameJob::prepare_render_object_instance_all(prepare_job);
            }

            self.task_pool.scope(|scope| {
                let thread_pool = &self;
                let task_pool = &self.task_pool;
                for view_index in 0..prepare_job.num_views() {
                    scope.spawn(async move {
                        let view_packet = prepare_job.view_packet(view_index as ViewFrameIndex);
                        let view_submit_packet =
                            prepare_job.view_submit_packet(view_index as ViewFrameIndex);

                        if let Some((chunk_size, num_chunks)) = thread_pool
                            .prepare_render_object_instance_per_view_num_chunks(
                                prepare_job,
                                view_packet,
                            )
                        {
                            task_pool.scope(|scope| {
                                for chunk_index in 0..num_chunks {
                                    scope.spawn(async move {
                                        RenderFrameJob::prepare_render_object_instance_per_view_chunk(
                                            prepare_job,
                                            view_packet,
                                            view_submit_packet,
                                            chunk_index,
                                            chunk_size,
                                        );
                                    });
                                }
                            });
                        } else {
                            RenderFrameJob::prepare_render_object_instance_per_view_all(
                                prepare_job,
                                view_packet,
                                view_submit_packet,
                            );
                        }

                        prepare_job.end_per_view_prepare(view_packet, view_submit_packet);
                    })
                }
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
        let (sender, receiver) = unbounded();

        self.task_pool.scope(|scope| {
            for (render_view_index, num_view_submit_nodes) in render_view_submit_nodes.iter() {
                let sender = sender.clone();
                scope.spawn(async move {
                    let submit_node_blocks = RenderFrameJob::create_submit_node_blocks_for_view(
                        render_registry,
                        render_view_index,
                        num_view_submit_nodes,
                        finished_prepare_jobs,
                    );

                    for submit_node_block in submit_node_blocks {
                        sender.send(submit_node_block).unwrap()
                    }
                });
            }
        });

        std::mem::drop(sender);
        receiver
            .iter()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|submit_node_block| (submit_node_block.view_phase().clone(), submit_node_block))
            .collect()
    }

    fn clone_to_box(&mut self) -> Box<dyn RendererThreadPool> {
        Box::new(self.clone())
    }
}

#[derive(Default)]
struct ParallelChunkSizes {
    extract_chunk_size: Option<usize>,
    extract_per_view_chunk_size: Option<usize>,
    prepare_chunk_size: Option<usize>,
    prepare_per_view_chunk_size: Option<usize>,
}

impl ParallelChunkSizes {
    pub fn extract_chunk_size(
        self,
        num: usize,
    ) -> Self {
        Self {
            extract_chunk_size: Some(num),
            ..self
        }
    }

    pub fn extract_per_view_chunk_size(
        self,
        num: usize,
    ) -> Self {
        Self {
            extract_per_view_chunk_size: Some(num),
            ..self
        }
    }

    pub fn prepare_chunk_size(
        self,
        num: usize,
    ) -> Self {
        Self {
            prepare_chunk_size: Some(num),
            ..self
        }
    }

    pub fn prepare_per_view_chunk_size(
        self,
        num: usize,
    ) -> Self {
        Self {
            prepare_per_view_chunk_size: Some(num),
            ..self
        }
    }
}
