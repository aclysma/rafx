
use crate::slab::SlabIndexT;
use crate::slab::RawSlab;
use crate::render_view::RenderView;
use crate::visibility::*;
use crate::GenericRenderNodeHandle;
use crate::RenderRegistry;
use std::any::Any;

// Keep a list of generic handles grouped by type
// - Requires less storage space since we store the type id once instead of per handle
// - Binning is essentially an O(n) sort
// - Lets us jobify on the type
struct HandleSet {
    handles: Vec<Vec<SlabIndexT>>
}

impl HandleSet {
    fn new() -> Self {
        let feature_count = RenderRegistry::registered_feature_count();

        let handles = (0..feature_count).map(|_| Vec::new()).collect();
        HandleSet {
            handles
        }
    }

    fn insert(&mut self, handle: GenericRenderNodeHandle) {
        self.handles[handle.render_feature_index() as usize].push(handle.slab_index());
    }
}

////////////////// StaticVisibilityNodeSet //////////////////
#[derive(Default)]
pub struct StaticVisibilityNodeSet {
    static_aabb: RawSlab<StaticAabbVisibilityNode>
}

impl StaticVisibilityNodeSet {
    pub fn register_static_aabb(&mut self, node: StaticAabbVisibilityNode) -> StaticAabbVisibilityNodeHandle {
        //TODO: Insert into spatial structure?
        StaticAabbVisibilityNodeHandle(self.static_aabb.allocate(node))
    }

    pub fn unregister_static_aabb(&mut self, handle: StaticAabbVisibilityNodeHandle) {
        //TODO: Remove from spatial structure?
        self.static_aabb.free(&handle.0);
    }

    pub fn calculate_static_visibility(&self, view: &RenderView) -> VisibilityResult {
        log::debug!("Calculate static visibility for {}", view.debug_name());
        let mut result = VisibilityResult::default();

        for (_, aabb) in self.static_aabb.iter() {
            result.handles.push(aabb.handle);
        }

        //TODO: Could consider sorting lists of handles by type/key to get linear memory access
        result
    }
}
