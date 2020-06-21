use renderer_base::slab::RawSlabKey;
use renderer_nodes::GenericRenderNodeHandle;

////////////////// StaticAabb VisibilityNode //////////////////
pub struct StaticAabbVisibilityNode {
    pub handle: GenericRenderNodeHandle,
}

#[derive(Copy, Clone)]
pub struct StaticAabbVisibilityNodeHandle(pub RawSlabKey<StaticAabbVisibilityNode>);

////////////////// DynamicAabb VisibilityNode //////////////////
pub struct DynamicAabbVisibilityNode {
    pub handle: GenericRenderNodeHandle,
}

#[derive(Copy, Clone)]
pub struct DynamicAabbVisibilityNodeHandle(pub RawSlabKey<DynamicAabbVisibilityNode>);
