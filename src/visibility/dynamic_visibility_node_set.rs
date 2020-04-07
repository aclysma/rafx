
use crate::slab::RawSlab;
use crate::render_view::RenderView;
use crate::visibility::*;
use crate::GenericRenderNodeHandle;

////////////////// DynamicVisibilityNodeSet //////////////////
#[derive(Default)]
pub struct DynamicVisibilityNodeSet {
    dynamic_aabb: RawSlab<DynamicAabbVisibilityNode>
}

impl DynamicVisibilityNodeSet {
    pub fn register_dynamic_aabb(&mut self, node: DynamicAabbVisibilityNode) -> DynamicAabbVisibilityNodeHandle {
        //TODO: Insert into spatial structure?
        DynamicAabbVisibilityNodeHandle(self.dynamic_aabb.allocate(node))
    }

    pub fn unregister_dynamic_aabb(&mut self, handle: DynamicAabbVisibilityNodeHandle) {
        //TODO: Remove from spatial structure?
        self.dynamic_aabb.free(&handle.0);
    }

    pub fn calculate_dynamic_visibility(&self, view: &RenderView) -> DynamicVisibilityResult {
        let mut result = DynamicVisibilityResult::default();

        for (_, aabb) in self.dynamic_aabb.iter() {
            result.handles.push(aabb.handle);
        }

        //TODO: Could consider sorting lists of handles by type/key to get linear memory access
        result
    }
}

#[derive(Default)]
pub struct DynamicVisibilityResult {
    pub handles: Vec<GenericRenderNodeHandle>
}