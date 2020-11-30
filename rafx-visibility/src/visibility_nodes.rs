use rafx_base::slab::DropSlabKey;
use rafx_nodes::GenericRenderNodeHandle;

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
