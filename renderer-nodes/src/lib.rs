mod render_nodes;
pub use render_nodes::GenericRenderNodeHandle;
pub use render_nodes::RenderNodeSet;
pub use render_nodes::AllRenderNodes;
pub use render_nodes::RenderNodeIndex;
pub use render_nodes::RenderNodeCount;

mod submit_nodes;
pub use submit_nodes::SubmitNode;
pub use submit_nodes::FeatureSubmitNodes;
pub use submit_nodes::ViewSubmitNodes;
pub use submit_nodes::MergedFrameSubmitNodes;
pub use submit_nodes::SubmitNodeId;
pub use submit_nodes::SubmitNodeSortKey;

mod render_views;
pub use render_views::RenderViewSet;
pub use render_views::RenderView;
pub use render_views::RenderPhaseMaskBuilder;
pub use render_views::RenderPhaseMask;
pub use render_views::RenderViewIndex;
pub use render_views::RenderViewCount;

mod frame_packet;
pub use frame_packet::FramePacket;
pub use frame_packet::FramePacketBuilder;
pub use frame_packet::PerFrameNode;
pub use frame_packet::PerViewNode;
pub use frame_packet::FrameNodeIndex;
pub use frame_packet::FrameNodeCount;
pub use frame_packet::ViewNodeIndex;
pub use frame_packet::ViewNodeCount;

mod jobs;
pub use jobs::*;

mod registry;
pub use registry::RenderRegistry;
pub use registry::RenderRegistryBuilder;
pub use registry::RenderFeature;
pub use registry::RenderPhase;
pub use registry::RenderFeatureIndex;
pub use registry::RenderPhaseIndex;

mod macro_render_feature;
mod macro_render_phase;

#[derive(Default)]
pub struct VisibilityResult {
    pub handles: Vec<GenericRenderNodeHandle>,
}
