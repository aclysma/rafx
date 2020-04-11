
trait Resource {

}

// A feature can use this to request loading resources. The provider is responsible for uploading
// data to the GPU
trait ResourceProvider<T : Resource> {
    fn add_ref_count(&self, resource_id: u32);
    fn remove_ref_count(&self, resource_id: u32);

    fn get_resource(&self, resource_id: u32) -> Option<&T>;

    // Need interface that lets us send deltas and repopulate GPU
    // - functions to get added/removed resources?
    // - functions to get all resources?
    // - update call? maybe it produces its own GPU calls?
}

trait ResourceRegistry {
    fn get_resource_provider<T: Resource>(&self) -> Option<&ResourceProvider<T>>;
}

/*
// Everything we want to draw. Generally holds a reference back to entities
trait RenderNode {

}

// Holds all render nodes for a given feature
trait RenderNodeSet {

}

// Responsible for producing ExtractedData. Implements the extract calls
trait ExtractJob<F: Feature> {
    //fn run(self) -> F::ExtractedData;
    fn run(self) -> F::PrepareJob;
}

trait ExtractedData {

}

trait PrepareJob<F: Feature> {
    //fn run(self, extracted_data: F::ExtractedData) -> F::PreparedData;

}

trait PreparedData {

}

trait Feature: Sized {
    type RenderNode: RenderNode;
    type RenderNodeSet: RenderNodeSet;

    type ExtractJob: ExtractJob<Self>;
    //type ExtractedData: ExtractedData;

    type PrepareJob: PrepareJob<Self>;
    type PreparedData: PreparedData;

    // fn test() -> u32;
    //
    // fn test2(&self) {
    //
    // }

    fn create_extract_job(visible_nodes_per_view: Vec<Vec<u32>>) -> ExtractJob<Self>;
}

struct DefaultExtractedData<FrameNodeT, ViewNodeT> {
    frame_nodes: Vec<FrameNodeT>,
    view_nodes: Vec<ViewNodeT>
}

impl<FrameNodeT, ViewNodeT> ExtractedData for DefaultExtractedData<FrameNodeT, ViewNodeT> {

}


struct DefaultExtractJob<FrameNodeT, ViewNodeT> {
    frame_nodes: Vec<FrameNodeT>,
    view_nodes: Vec<ViewNodeT>
}

impl<F: Feature, FrameNodeT, ViewNodeT> ExtractJob<F> for DefaultExtractJob<FrameNodeT, ViewNodeT> {
    // fn run(self) -> F::ExtractedData {
    //     DefaultExtractedData::<FrameNodeT, ViewNodeT> {
    //         frame_nodes: self.frame_nodes,
    //         view_nodes: self.view_nodes
    //     }
    // }
    fn run(self) -> F::PrepareJob {
        F::P
    }
}

struct DefaultPrepareJob<FrameNodeT, ViewNodeT> {
    frame_nodes: Vec<FrameNodeT>,
    view_nodes: Vec<ViewNodeT>
}

impl<F: Feature, FrameNodeT, ViewNodeT> PrepareJob<F> for DefaultPrepareJob<FrameNodeT, ViewNodeT> {
    // fn run(self, extracted_data: F::ExtractedData) -> <F as Feature>::PreparedData {
    //     unimplemented!()
    // }
}
*/
//////////////////////////////////////

trait ExtractJob {
    fn extract(self) -> Box<dyn PrepareJob>;
}

trait PrepareJob {
    fn prepare(self);
}

trait SubmitNodeContainer {
    fn submit(view: u32, stage: u32, node_index: u32);
}

struct SpriteExtractJob<'a> {
    test_str: &'a String,
    vec_o_stuff: Vec<u32>
}

impl<'a> ExtractJob for SpriteExtractJob<'a> {
    fn extract(self) -> Box<PrepareJob> {
        Box::new(SpritePrepareJob {
            vec_o_stuff: self.vec_o_stuff
        })
    }
}

struct SpritePrepareJob {
    vec_o_stuff: Vec<u32>

}

impl PrepareJob for SpritePrepareJob {
    fn prepare(self) {

    }
}


trait ExtractJobImpl {
    fn begin(&self);
    fn extract_frame_node(&self, entity: u32);
    fn extract_view_node(&self, entity: u32, view: u32);
    fn finish_view(&self, view: u32);
    fn finish_frame(self) -> Box<PrepareJob>;
}

struct DefaultExtractJob<ExtractImplT: ExtractJobImpl> {
    job_impl: ExtractImplT
}

impl<ExtractImplT: ExtractJobImpl> ExtractJob for DefaultExtractJob<ExtractImplT> {
    fn extract(self) -> Box<PrepareJob> {
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

fn test_fn() {
    let mut test_str = "test".to_string();

    let sprite_job = SpriteExtractJob {
        test_str: &test_str,
        vec_o_stuff: Vec::new()
    };

    let prepare_job = sprite_job.extract();
    test_str.insert(0, '1');
}

struct SpriteExtractJobImpl<'a> {
    test_str: &'a String,
    vec_o_stuff: Vec<u32>
}

impl<'a> ExtractJobImpl for SpriteExtractJobImpl<'a> {
    fn begin(&self) {
    }

    fn extract_frame_node(&self, entity: u32) {
    }

    fn extract_view_node(&self, entity: u32, view: u32) {
    }

    fn finish_view(&self, view: u32) {
    }

    fn finish_frame(self) -> Box<PrepareJob> {
        Box::new(DefaultPrepareJob {
            job_impl: SpritePrepareJobImpl {
                vec_o_stuff: self.vec_o_stuff
            }
        })
    }
}

struct SpritePrepareJobImpl {
    vec_o_stuff: Vec<u32>
}

impl PrepareJobImpl for SpritePrepareJobImpl {
    fn begin(&self) {
    }

    fn prepare_frame_node(&self, entity: u32) {
    }

    fn prepare_view_node(&self, entity: u32, view: u32) {
    }

    fn finish_view(&self, view: u32) {
    }

    fn finish_frame(&self) {
        unimplemented!()
    }
}

fn test_fn2() {
    let mut test_str = "test".to_string();

    let sprite_job = SpriteExtractJob {
        test_str: &test_str,
        vec_o_stuff: Vec::new()
    };

    let prepare_job = sprite_job.extract();
    test_str.insert(0, '1');
}
