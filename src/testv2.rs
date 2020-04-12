
trait ExtractJobImpl {
    fn begin(&self);
    fn extract_frame_node(&self, entity: u32);
    fn extract_view_node(&self, entity: u32, view: u32);
    fn finish_view(&self, view: u32);
    fn finish_frame(self) -> Box<dyn PrepareJob>;
}

struct DefaultExtractJob<ExtractImplT: ExtractJobImpl> {
    job_impl: ExtractImplT
}

impl<ExtractImplT: ExtractJobImpl> ExtractJob for DefaultExtractJob<ExtractImplT> {
    fn extract(self) -> Box<dyn PrepareJob> {
        // Responsible for iterating across frame packet to call these callbacks
        self.job_impl.begin();
        self.job_impl.extract_frame_node(0);
        self.job_impl.extract_view_node(0, 0);
        self.job_impl.finish_view(0);
        self.job_impl.finish_frame()
    }
}

trait PrepareJobImpl {
    fn begin(&self);
    fn prepare_frame_node(&self, entity: u32);
    fn prepare_view_node(&self, entity: u32, view: u32);
    fn finish_view(&self, view: u32);
    fn finish_frame(&self);
}


struct DefaultPrepareJob<PrepareImplT: PrepareJobImpl> {
    job_impl: PrepareImplT
}

impl<PrepareImplT: PrepareJobImpl> PrepareJob for DefaultPrepareJob<PrepareImplT> {
    fn prepare(self) {

    }
}

trait SubmitKernel {
    fn apply_setup();
    fn render_element(index: u32);
    fn revert_setup();
}
