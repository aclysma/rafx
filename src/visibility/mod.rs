mod visibility_nodes;
pub use visibility_nodes::*;

mod static_visibility_node_set;
pub use static_visibility_node_set::StaticVisibilityNodeSet;

mod dynamic_visibility_node_set;
pub use dynamic_visibility_node_set::DynamicVisibilityNodeSet;
use crate::GenericRenderNodeHandle;

#[derive(Default)]
pub struct VisibilityResult {
    pub handles: Vec<GenericRenderNodeHandle>,
}
