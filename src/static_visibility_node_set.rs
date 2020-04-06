
use renderer_base::slab::RawSlab;
use crate::render_view::RenderView;
use crate::visibility_nodes::*;
use crate::GenericRenderNodeHandle;

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

    pub fn calculate_static_visibility(&self, view: &RenderView) -> StaticVisibilityResult {
        let mut result = StaticVisibilityResult::default();

        for (_, aabb) in self.static_aabb.iter() {
            result.handles.push(aabb.handle);
        }

        //TODO: Could consider sorting lists of handles by type/key to get linear memory access
        result
    }
}

#[derive(Default)]
pub struct StaticVisibilityResult {
    pub handles: Vec<GenericRenderNodeHandle>
}