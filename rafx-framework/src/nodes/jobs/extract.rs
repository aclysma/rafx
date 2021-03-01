use crate::nodes::{FramePacket, PrepareJob, PrepareJobSet, RenderFeatureIndex, RenderView, RenderJobExtractContext};

pub trait ExtractJob {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob>;

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct ExtractJobSet {
    extract_jobs: Vec<Box<dyn ExtractJob>>,
}

impl Default
    for ExtractJobSet
{
    fn default() -> Self {
        ExtractJobSet {
            extract_jobs: Default::default(),
        }
    }
}

impl ExtractJobSet
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_job(
        &mut self,
        extract_job: Box<dyn ExtractJob>,
    ) {
        self.extract_jobs.push(extract_job)
    }

    pub fn extract(
        self,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> PrepareJobSet {
        log::trace!("Start extract job set");

        let mut prepare_jobs = vec![];
        for extract_job in self.extract_jobs {
            log::trace!("Start job {}", extract_job.feature_debug_name());

            let prepare_job = extract_job.extract(extract_context, frame_packet, views);
            prepare_jobs.push(prepare_job);
        }

        PrepareJobSet::new(prepare_jobs)
    }
}
