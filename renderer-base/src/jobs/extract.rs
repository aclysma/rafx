use crate::{FramePacket, RenderView, PrepareJob, PrepareJobSet};

pub trait ExtractJob<SourceT> {
    fn extract(self: Box<Self>, source: &SourceT, frame_packet: &FramePacket, views: &[&RenderView]) -> Box<dyn PrepareJob>;

    fn feature_debug_name(&self) -> &'static str;
}

pub struct ExtractJobSet<SourceT> {
    extract_jobs: Vec<Box<dyn ExtractJob<SourceT>>>
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
        extract_job: Box<dyn ExtractJob<SourceT>>,
    ) {
        self.extract_jobs.push(extract_job)
    }

    pub fn extract(self, source: &SourceT, frame_packet: &FramePacket, views: &[&RenderView]) -> PrepareJobSet {
        log::debug!("Start extract job set");

        let mut prepare_jobs = vec![];
        for extract_job in self.extract_jobs {
            log::debug!("Start job {}", extract_job.feature_debug_name());

            let prepare_job = extract_job.extract(source, frame_packet, views);
            prepare_jobs.push(prepare_job);
        }

        PrepareJobSet::new(prepare_jobs)
    }
}

