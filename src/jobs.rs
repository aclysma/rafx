use crate::render_view::RenderView;
use crate::visibility::VisibilityResult;

////////////////// ViewExtractJob //////////////////
enum ViewExtractJobState {
    WaitingForStaticVisibility,
    WaitingForDynamicVisibility,
    WaitingForSimulationFinish,
    Finished,
}

/// This wraps the process of calculating visibility and extracting data from the world into
struct ViewExtractJob {
    state: ViewExtractJobState,
    view: RenderView,
    //static_visibility_future: std::future::Future<StaticVisibilityNodeSet>::Output,
    //dynamic_visibility_future: std::future::Future<DynamicVisibilityNodeSet>
    //world_future: std::future::Future<&World>
}

impl ViewExtractJob {
    fn new(view: RenderView) -> Self {
        ViewExtractJob {
            state: ViewExtractJobState::WaitingForStaticVisibility,
            view,
            //static_visibility_future
            //dynamic_visibility_future
            //world_future
        }
    }

    fn on_static_visibility_ready(
        &mut self,
        nodes: &VisibilityResult,
    ) {
        // This should set some
        //self.static_visibility_future = result::ok(nodes);
    }

    fn on_dynamic_visibility_ready(
        &mut self,
        nodes: &VisibilityResult,
    ) {
        //self.dynamic_visibility_future = result::ok(nodes);
    }

    fn on_simulation_finish(&mut self /*, world: &World*/) {
        //self.world_future = result::ok(nodes);
    }

    fn poll(&self) -> std::task::Poll<()> {
        // spawn task..
        // Await static_visibility_future
        // create static visibility result
        // Await dynamic_visibility_future
        // create dynamic visibility result
        // create nodes
        // Await world_future
        // extract data

        std::task::Poll::Ready(())
    }
}
