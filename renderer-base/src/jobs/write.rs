use crate::{
    RenderFeatureIndex, RenderPhase, RenderView, MergedFrameSubmitNodes, RenderRegistry,
    SubmitNodeId,
};

pub trait FeatureCommandWriter<WriteT> {
    fn apply_setup(
        &self,
        write_context: &mut WriteT,
    );
    fn render_element(
        &self,
        write_context: &mut WriteT,
        index: SubmitNodeId,
    );
    fn revert_setup(
        &self,
        write_context: &mut WriteT,
    );

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

// pub struct FeatureCommandWriterSet<WriteT> {
//     prepare_jobs: Vec<Box<dyn FeatureCommandWriter<WriteT>>>,
// }

pub struct PreparedRenderData<WriteT> {
    feature_writers: Vec<Option<Box<dyn FeatureCommandWriter<WriteT>>>>,
    submit_nodes: MergedFrameSubmitNodes,
}

impl<WriteT> PreparedRenderData<WriteT> {
    pub fn new(
        feature_writers: Vec<Box<dyn FeatureCommandWriter<WriteT>>>,
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
        write_context: &mut WriteT,
    ) {
        let submit_nodes = self.submit_nodes.submit_nodes::<PhaseT>(view);

        let mut previous_node_feature_index: i32 = -1;
        for submit_node in submit_nodes {
            if submit_node.feature_index() as i32 != previous_node_feature_index {
                if previous_node_feature_index != -1 {
                    // call revert setup
                    log::debug!("revert setup for feature {}", previous_node_feature_index);
                    self.feature_writers[previous_node_feature_index as usize]
                        .as_ref()
                        .unwrap()
                        .revert_setup(write_context);
                }

                // call apply setup
                log::debug!("apply setup for feature {}", submit_node.feature_index());
                self.feature_writers[submit_node.feature_index() as usize]
                    .as_ref()
                    .unwrap()
                    .apply_setup(write_context);
            }

            log::debug!(
                "draw render node feature: {} node id: {}",
                submit_node.feature_index(),
                submit_node.submit_node_id()
            );
            self.feature_writers[submit_node.feature_index() as usize]
                .as_ref()
                .unwrap()
                .render_element(write_context, submit_node.submit_node_id());
            previous_node_feature_index = submit_node.feature_index() as i32;
        }

        if previous_node_feature_index != -1 {
            // call revert setup
            log::debug!("revert setup for feature: {}", previous_node_feature_index);
        }
    }
}
