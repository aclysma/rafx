use crate::nodes::GenericRenderNodeHandle;
use rafx_base::slab::DropSlabKey;

////////////////// StaticAabb VisibilityNode //////////////////
pub struct StaticAabbVisibilityNode {
    pub handle: GenericRenderNodeHandle,
}

#[derive(Clone)]
pub struct StaticAabbVisibilityNodeHandle(pub DropSlabKey<StaticAabbVisibilityNode>);

////////////////// DynamicAabb VisibilityNode //////////////////
pub struct DynamicAabbVisibilityNode {
    pub handle: GenericRenderNodeHandle,
}

#[derive(Clone)]
pub struct DynamicAabbVisibilityNodeHandle(pub DropSlabKey<DynamicAabbVisibilityNode>);
