use super::*;
use rafx_base::slab::DropSlab;
use crate::nodes::RenderView;
use crate::nodes::VisibilityResult;

#[derive(Default)]
pub struct StaticVisibilityNodeSet {
    static_aabb: DropSlab<StaticAabbVisibilityNode>,
}

impl StaticVisibilityNodeSet {
    pub fn register_static_aabb(
        &mut self,
        node: StaticAabbVisibilityNode,
    ) -> StaticAabbVisibilityNodeHandle {
        //TODO: Insert into spatial structure?
        StaticAabbVisibilityNodeHandle(self.static_aabb.allocate(node))
    }

    pub fn calculate_static_visibility(
        &mut self,
        view: &RenderView,
    ) -> VisibilityResult {
        self.static_aabb.process_drops();

        log::trace!("Calculate static visibility for {}", view.debug_name());
        let mut result = VisibilityResult::default();

        for aabb in self.static_aabb.iter_values() {
            log::trace!("push static visibility object {:?}", aabb.handle);
            result.handles.push(aabb.handle);
        }

        //TODO: Could consider sorting lists of handles by type/key to get linear memory access
        result
    }
}
