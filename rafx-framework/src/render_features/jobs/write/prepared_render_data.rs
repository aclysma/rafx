use crate::render_features::render_features_prelude::*;
use crate::render_features::{BeginSubmitNodeBatchArgs, RenderSubmitNodeArgs};
use fnv::FnvHashMap;
use rafx_api::RafxResult;
use std::sync::Arc;

/// The total number of submit nodes needed by each `RenderView`. The `RenderView` is represented by
/// the `RenderViewIndex` key and the count of submit nodes are contained by a `Vec<usize>` where
/// the count at each index `I` corresponds to the `RenderPhase` with `RenderPhaseIndex` equal to `I`.
pub type RenderViewSubmitNodeCount = FnvHashMap<RenderViewIndex, Vec<usize>>;

/// A collection containing all `SubmitNode`s across all `RenderFeature`s matching a `RenderView`
/// and `RenderPhase` where the `SubmitNode`s are sorted by the `RenderPhase`'s sort function. The
/// `RenderView` and `RenderPhase` pair is represented by the `ViewPhase` key and the `SubmitNode`s
/// are contained by the `ViewPhaseSubmitNodeBlock`.
pub type SubmitNodeBlocks = FnvHashMap<ViewPhase, ViewPhaseSubmitNodeBlock>;

/// The sorted `SubmitNode`s for each `RenderView` and `RenderPhase` with all relevant `RenderFeature`'s
/// `RenderFeatureWriteJob`s.
pub struct PreparedRenderData<'write> {
    submit_node_blocks: &'write SubmitNodeBlocks,
    write_jobs: Vec<Option<Arc<dyn RenderFeatureWriteJob<'write> + 'write>>>,
}

impl<'write> PreparedRenderData<'write> {
    pub fn new(
        submit_node_blocks: &'write SubmitNodeBlocks,
        write_jobs: Vec<Option<Arc<dyn RenderFeatureWriteJob<'write> + 'write>>>,
    ) -> Self {
        Self {
            submit_node_blocks,
            write_jobs,
        }
    }

    pub fn on_begin_execute_graph(
        &self,
        write_context: &mut RenderJobBeginExecuteGraphContext,
    ) -> RafxResult<()> {
        for writer in &self.write_jobs {
            if let Some(writer) = writer {
                writer.on_begin_execute_graph(write_context)?;
            }
        }

        Ok(())
    }

    pub fn write_view_phase<PhaseT: RenderPhase>(
        &self,
        view: &RenderView,
        write_context: &mut RenderJobCommandBufferContext,
    ) -> RafxResult<()> {
        profiling::scope!({
            use rafx_base::memory::force_to_static_lifetime;
            unsafe { force_to_static_lifetime(view).debug_name() }
        });

        let render_phase_index = PhaseT::render_phase_index();

        let view_phase = ViewPhase {
            view_index: view.view_index(),
            phase_index: render_phase_index,
        };

        let submit_node_block = self.submit_node_blocks.get(&view_phase);
        let submit_nodes: &[RenderFeatureSubmitNode] =
            if let Some(submit_node_block) = submit_node_block {
                submit_node_block.submit_nodes()
            } else {
                &[]
            };

        let mut previous_node_feature_index: Option<RenderFeatureIndex> = None;
        let mut previous_node_sort_key: Option<SubmitNodeSortKey> = None;
        let mut view_frame_index: Option<ViewFrameIndex> = None;

        for submit_node in submit_nodes {
            let feature_index = submit_node.feature_index();
            let sort_key = submit_node.sort_key();
            let feature_changed = Some(feature_index) != previous_node_feature_index;
            let sort_key_changed = Some(sort_key) != previous_node_sort_key;

            if feature_changed {
                view_frame_index = Some(
                    self.write_jobs[submit_node.feature_index() as usize]
                        .as_ref()
                        .unwrap()
                        .view_frame_index(view),
                );
            }

            if feature_changed || sort_key_changed {
                self.write_jobs[submit_node.feature_index() as usize]
                    .as_ref()
                    .unwrap()
                    .begin_submit_node_batch(
                        write_context,
                        BeginSubmitNodeBatchArgs {
                            view_frame_index: view_frame_index.unwrap(),
                            render_phase_index,
                            feature_changed,
                            // previous_node_sort_key
                            sort_key: submit_node.sort_key(),
                        },
                    )?;
            }

            log::trace!(
                "draw render node feature: {} node id: {}",
                submit_node.feature_index(),
                submit_node.submit_node_id(),
            );

            self.write_jobs[submit_node.feature_index() as usize]
                .as_ref()
                .unwrap()
                .render_submit_node(
                    write_context,
                    RenderSubmitNodeArgs {
                        view_frame_index: view_frame_index.unwrap(),
                        render_phase_index,
                        submit_node_id: submit_node.submit_node_id(),
                    },
                )?;

            previous_node_feature_index = Some(feature_index);
            previous_node_sort_key = Some(sort_key);
        }

        Ok(())
    }
}
