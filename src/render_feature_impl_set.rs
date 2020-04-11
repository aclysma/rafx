use crate::{RenderRegistry, FramePacket, RenderView, RenderPhase};
use crate::registry::{RenderFeatureExtractImpl, RenderPhaseIndex};

pub struct RenderFeatureExtractImplSet {
    feature_impls: Vec<Option<Box<RenderFeatureExtractImpl>>>,
}

impl RenderFeatureExtractImplSet {
    pub fn new() -> Self {
        let feature_count = RenderRegistry::registered_feature_count();
        //let feature_impls = Vec::with_capacity(feature_count).resize_with(feature_count, None);
        let feature_impls: Vec<_> = (0..feature_count).map(|_| None).collect();

        RenderFeatureExtractImplSet { feature_impls }
    }

    pub fn add_impl(
        &mut self,
        render_feature_impl: Box<RenderFeatureExtractImpl>,
    ) {
        let feature_index = render_feature_impl.feature_index() as usize;
        self.feature_impls[feature_index] = Some(render_feature_impl);
    }

    pub fn extract(
        &self,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) {
        log::debug!("RenderFeatureExtractImplSet::extract");
        // In the future, make features run in parallel
        for feature_impl in &self.feature_impls {
            if let Some(feature_impl) = feature_impl {
                log::debug!("extract_begin {}", feature_impl.feature_debug_name());
                feature_impl.extract_begin(frame_packet);

                // foreach frame node, call extract
                //for frame_node in frame_packet.fram
                log::debug!("extract_frame_node {}", feature_impl.feature_debug_name());
                feature_impl.extract_frame_node(frame_packet);

                for view in views {
                    // foreach view node, call extract
                    log::debug!(
                        "extract_frame_node {} {}",
                        feature_impl.feature_debug_name(),
                        view.debug_name()
                    );
                    feature_impl.extract_view_nodes(frame_packet);

                    // call once after all view nodes extracted
                    log::debug!(
                        "extract_view_finalize {} {}",
                        feature_impl.feature_debug_name(),
                        view.debug_name()
                    );
                    feature_impl.extract_view_finalize(frame_packet);
                }

                // call once after all nodes extracted
                log::debug!(
                    "extract_frame_finalize {}",
                    feature_impl.feature_debug_name()
                );
                feature_impl.extract_frame_finalize(frame_packet);
            }
        }
    }

    // pub fn prepare(
    //     &self,
    //     frame_packet: &FramePacket,
    //     views: &[&RenderView],
    // ) {
    //     log::debug!("RenderFeatureExtractImplSet::prepare");
    //     for feature_impl in &self.feature_impls {
    //         if let Some(feature_impl) = feature_impl {
    //             log::debug!("prepare_begin {}", feature_impl.feature_debug_name());
    //             //feature_impl.prepare_begin(frame_packet);
    //
    //             // foreach frame node, call extract
    //             log::debug!("prepare_frame_nodes {}", feature_impl.feature_debug_name());
    //             //feature_impl.prepare_frame_nodes(frame_packet);
    //
    //             for view in views {
    //                 // foreach view node, call extract
    //                 log::debug!(
    //                     "prepare_view_nodes {} {}",
    //                     feature_impl.feature_debug_name(),
    //                     view.debug_name()
    //                 );
    //                 //feature_impl.prepare_view_nodes(frame_packet);
    //
    //                 // call once after all view nodes extracted
    //                 log::debug!(
    //                     "prepare_view_finalize {} {}",
    //                     feature_impl.feature_debug_name(),
    //                     view.debug_name()
    //                 );
    //                 //feature_impl.prepare_view_finalize(frame_packet);
    //             }
    //
    //             // call once after all nodes extracted
    //             log::debug!(
    //                 "prepare_frame_finalize {}",
    //                 feature_impl.feature_debug_name()
    //             );
    //             //feature_impl.prepare_frame_finalize(frame_packet);
    //         }
    //     }
    // }
    //
    // pub fn submit<T: RenderPhase>(
    //     &self,
    //     frame_packet: &FramePacket,
    //     view: &RenderView,
    //     render_phase: T
    // ) {
    //     log::debug!("RenderFeatureExtractImplSet::submit {} {}", core::any::type_name::<T>(), view.debug_name());
    // }
}
