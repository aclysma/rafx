use crate::{
    RenderFeatureIndex, RenderPhase, RenderView, MergedFrameSubmitNodes, RenderRegistry,
    SubmitNodeId, RenderPhaseIndex,
};

pub trait FeatureCommandWriter<WriteContextT> {
    fn apply_setup(
        &self,
        write_context: &mut WriteContextT,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    );
    fn render_element(
        &self,
        write_context: &mut WriteContextT,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    );
    fn revert_setup(
        &self,
        write_context: &mut WriteContextT,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    );

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

// pub struct FeatureCommandWriterSet<WriteContextT> {
//     prepare_jobs: Vec<Box<dyn FeatureCommandWriter<WriteContextT>>>,
// }

pub struct PreparedRenderData<WriteContextT> {
    feature_writers: Vec<Option<Box<dyn FeatureCommandWriter<WriteContextT>>>>,
    submit_nodes: MergedFrameSubmitNodes,
}

impl<WriteContextT> PreparedRenderData<WriteContextT> {
    pub fn new(
        feature_writers: Vec<Box<dyn FeatureCommandWriter<WriteContextT>>>,
        submit_nodes: MergedFrameSubmitNodes,
    ) -> Self {
        let mut writers: Vec<_> = (0..RenderRegistry::registered_feature_count())
            .map(|_| None)
            .collect();

        for writer in feature_writers {
            let feature_index = writer.feature_index();
            writers[feature_index as usize] = Some(writer);
        }

        PreparedRenderData {
            feature_writers: writers,
            submit_nodes,
        }
    }

    pub fn write_view_phase<PhaseT: RenderPhase>(
        &self,
        view: &RenderView,
        write_context: &mut WriteContextT,
    ) {
        let submit_nodes = self.submit_nodes.submit_nodes::<PhaseT>(view);
        let render_phase_index = PhaseT::render_phase_index();

        let mut previous_node_feature_index: i32 = -1;
        for submit_node in submit_nodes {
            if submit_node.feature_index() as i32 != previous_node_feature_index {
                if previous_node_feature_index != -1 {
                    // call revert setup
                    log::trace!("revert setup for feature {}", previous_node_feature_index);
                    self.feature_writers[previous_node_feature_index as usize]
                        .as_ref()
                        .unwrap()
                        .revert_setup(write_context, view, render_phase_index);
                }

                // call apply setup
                log::trace!("apply setup for feature {}", submit_node.feature_index());
                self.feature_writers[submit_node.feature_index() as usize]
                    .as_ref()
                    .unwrap()
                    .apply_setup(write_context, view, render_phase_index);
            }

            log::trace!(
                "draw render node feature: {} node id: {}",
                submit_node.feature_index(),
                submit_node.submit_node_id(),
            );
            self.feature_writers[submit_node.feature_index() as usize]
                .as_ref()
                .unwrap()
                .render_element(
                    write_context,
                    view,
                    render_phase_index,
                    submit_node.submit_node_id(),
                );
            previous_node_feature_index = submit_node.feature_index() as i32;
        }

        if previous_node_feature_index != -1 {
            // call revert setup
            log::trace!("revert setup for feature: {}", previous_node_feature_index);
        }
    }
}
