use crate::{RenderRegistry, FramePacket, RenderView, RenderPhase, RenderFeatureIndex, PrepareJob, PrepareJobSet};
use crate::registry::RenderPhaseIndex;

use std::sync::Arc;
use std::marker::PhantomData;

pub trait ExtractJob<SourceT> {
    fn extract(self: Box<Self>, source: &SourceT, frame_packet: &FramePacket, views: &[&RenderView]) -> Box<dyn PrepareJob>;

    fn feature_debug_name(&self) -> &'static str;
}

pub struct ExtractJobSet<SourceT> {
    extract_jobs: Vec<Box<ExtractJob<SourceT>>>
}

impl<SourceT> Default for ExtractJobSet<SourceT> {
    fn default() -> Self {
        ExtractJobSet {
            extract_jobs: Default::default()
        }
    }
}

impl<SourceT> ExtractJobSet<SourceT> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_job(
        &mut self,
        extract_job: Box<ExtractJob<SourceT>>,
    ) {
        self.extract_jobs.push(extract_job)
    }

    pub fn extract(mut self, source: &SourceT, frame_packet: &FramePacket, views: &[&RenderView]) -> PrepareJobSet {
        log::debug!("Start extract job set");

        let mut prepare_jobs = vec![];
        for mut extract_job in self.extract_jobs {
            log::debug!("Start job {}", extract_job.feature_debug_name());

            let prepare_job = extract_job.extract(source, frame_packet, views);
            prepare_jobs.push(prepare_job);
        }

        PrepareJobSet::new(prepare_jobs)
    }
}


pub trait DefaultExtractJobImpl<SourceT> {
    fn extract_begin(&self, source: &SourceT);
    fn extract_frame_node(&self, source: &SourceT, entity: u32);
    fn extract_view_node(&self, source: &SourceT, entity: u32, view: u32);
    fn extract_view_finalize(&self, source: &SourceT, view: u32);
    fn extract_frame_finalize(self, source: &SourceT) -> Box<PrepareJob>;

    fn feature_debug_name(&self) -> &'static str;
}

struct FrameNode {

}

pub(crate) struct DefaultExtractJob<SourceT, ExtractImplT: DefaultExtractJobImpl<SourceT>> {
    extract_impl: ExtractImplT,
    phantom_data: PhantomData<SourceT>

    //frame_nodes: Vec<FrameNode>
}

impl<SourceT, ExtractImplT: DefaultExtractJobImpl<SourceT>> DefaultExtractJob<SourceT, ExtractImplT> {
    pub fn new(extract_impl: ExtractImplT) -> Self {
        DefaultExtractJob {
            extract_impl,
            phantom_data: Default::default()
        }
    }
}

impl<SourceT, ExtractImplT: DefaultExtractJobImpl<SourceT>> ExtractJob<SourceT> for DefaultExtractJob<SourceT, ExtractImplT> {
    fn extract(self: Box<Self>, source: &SourceT, frame_packet: &FramePacket, views: &[&RenderView]) -> Box<PrepareJob> {
        log::debug!("DefaultExtractJob::extract");
        // // Responsible for iterating across frame packet to call these callbacks
        // self.extract_impl.extract_begin(source);
        // self.extract_impl.extract_frame_node(source, 0);
        // self.extract_impl.extract_view_node(source, 0, 0);
        // self.extract_impl.extract_view_finalize(source, 0);
        // self.extract_impl.extract_frame_finalize(source)

        // In the future, make features run in parallel
        log::debug!("extract_begin {}", self.extract_impl.feature_debug_name());
        self.extract_impl.extract_begin(source); //TODO: Pass node counts

        // foreach frame node, call extract
        //for frame_node in frame_packet.fram
        log::debug!("extract_frame_node {}", self.extract_impl.feature_debug_name());
        self.extract_impl.extract_frame_node(source, 0); //TODO: Call once per frame node

        //TODO: Views can run in parallel
        for view in views {
            // foreach view node, call extract
            log::debug!(
                "extract_frame_node {} {}",
                self.extract_impl.feature_debug_name(),
                view.debug_name()
            );
            self.extract_impl.extract_view_node(source, 0, 0); //TODO: Call once per view node

            // call once after all view nodes extracted
            log::debug!(
                "extract_view_finalize {} {}",
                self.extract_impl.feature_debug_name(),
                view.debug_name()
            );
            self.extract_impl.extract_view_finalize(source, 0); //TODO: Pass the view?
        }

        // call once after all nodes extracted
        log::debug!(
            "extract_frame_finalize {}",
            self.extract_impl.feature_debug_name()
        );
        self.extract_impl.extract_frame_finalize(source)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.extract_impl.feature_debug_name()
    }
}
















// struct SpriteExtractJob {
//     world: Arc<String>,
//     vec_o_stuff: Vec<u32>
// }
//
// impl ExtractJob for SpriteExtractJob {
//     fn extract(self: Box<Self>) -> Box<PrepareJob> {
//         Box::new(SpritePrepareJob {
//             vec_o_stuff: self.vec_o_stuff
//         })
//     }
// }
//
// struct SpritePrepareJob {
//     vec_o_stuff: Vec<u32>
// }
//
// impl PrepareJob for SpritePrepareJob {
//     fn prepare(self) {
//         unimplemented!()
//     }
// }

// fn test_stuff() {
//     let mut world = Arc::new("test".to_string());
//
//     let prepare_job_set = {
//         let mut extract_job_set = ExtractJobSet::default();
//         extract_job_set.add_job(Box::new(SpriteExtractJob {
//             world: world.clone(),
//             vec_o_stuff: vec![]
//         }));
//
//         extract_job_set.extract()
//     };
//
//     let world_unwrapped = Arc::try_unwrap(world).unwrap();
//
//     prepare_job_set.prepare();
// }
//
// fn run_extract_jobs(extract_jobs: Vec<Box<ExtractJob>>) -> Vec<Box<PrepareJob>> {
//     let mut prepare_jobs = vec![];
//     for extract_job in extract_jobs {
//         prepare_jobs.push(extract_job.extract())
//     }
//     prepare_jobs
// }





pub trait RenderFeatureExtractImpl {
    fn feature_index(&self) -> RenderFeatureIndex;
    fn feature_debug_name(&self) -> &str;

    fn extract_begin(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_frame_node(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_view_nodes(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_view_finalize(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_frame_finalize(
        &self,
        frame_packet: &FramePacket,
    );
}

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
