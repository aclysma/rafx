use super::*;
use rafx_base::slab::DropSlab;
use crate::nodes::RenderView;

#[derive(Default)]
pub struct DynamicVisibilityNodeSet {
    dynamic_aabb: DropSlab<DynamicAabbVisibilityNode>,
}

impl DynamicVisibilityNodeSet {
    pub fn register_dynamic_aabb(
        &mut self,
        node: DynamicAabbVisibilityNode,
    ) -> DynamicAabbVisibilityNodeHandle {
        //TODO: Insert into spatial structure?
        DynamicAabbVisibilityNodeHandle(self.dynamic_aabb.allocate(node))
    }

    pub fn calculate_dynamic_visibility(
        &mut self,
        view: &RenderView,
    ) -> VisibilityResult {
        self.dynamic_aabb.process_drops();

        log::trace!("Calculate dynamic visibility for {}", view.debug_name());
        let mut result = VisibilityResult::default();

        for aabb in self.dynamic_aabb.iter_values() {
            log::trace!("push dynamic visibility object {:?}", aabb.handle);
            result.handles.push(aabb.handle);
        }

        //TODO: Could consider sorting lists of handles by type/key to get linear memory access
        result
    }
}
