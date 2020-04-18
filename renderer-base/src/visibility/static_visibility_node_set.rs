use crate::slab::RawSlab;
use crate::render_views::RenderView;
use crate::visibility::*;

#[derive(Default)]
pub struct StaticVisibilityNodeSet {
    static_aabb: RawSlab<StaticAabbVisibilityNode>,
}

impl StaticVisibilityNodeSet {
    pub fn register_static_aabb(
        &mut self,
        node: StaticAabbVisibilityNode,
    ) -> StaticAabbVisibilityNodeHandle {
        //TODO: Insert into spatial structure?
        StaticAabbVisibilityNodeHandle(self.static_aabb.allocate(node))
    }

    pub fn unregister_static_aabb(
        &mut self,
        handle: StaticAabbVisibilityNodeHandle,
    ) {
        //TODO: Remove from spatial structure?
        self.static_aabb.free(handle.0);
    }

    pub fn calculate_static_visibility(
        &self,
        view: &RenderView,
    ) -> VisibilityResult {
        log::debug!("Calculate static visibility for {}", view.debug_name());
        let mut result = VisibilityResult::default();

        for (_, aabb) in self.static_aabb.iter() {
            log::trace!("push static visibility object {:?}", aabb.handle);
            result.handles.push(aabb.handle);
        }

        //TODO: Could consider sorting lists of handles by type/key to get linear memory access
        result
    }
}
