mod render_nodes;
pub use render_nodes::AllRenderNodes;
pub use render_nodes::GenericRenderNodeHandle;
pub use render_nodes::RenderNodeCount;
pub use render_nodes::RenderNodeIndex;
pub use render_nodes::RenderNodeSet;

mod submit_nodes;
pub use submit_nodes::FeatureSubmitNodes;
pub use submit_nodes::MergedFrameSubmitNodes;
pub use submit_nodes::SubmitNode;
pub use submit_nodes::SubmitNodeId;
pub use submit_nodes::SubmitNodeSortKey;
pub use submit_nodes::ViewSubmitNodes;

mod render_views;
pub use render_views::RenderPhaseMask;
pub use render_views::RenderPhaseMaskBuilder;
pub use render_views::RenderView;
pub use render_views::RenderViewCount;
pub use render_views::RenderViewDepthRange;
pub use render_views::RenderViewIndex;
pub use render_views::RenderViewSet;

mod frame_packet;
pub use frame_packet::FrameNodeCount;
pub use frame_packet::FrameNodeIndex;
pub use frame_packet::FramePacket;
pub use frame_packet::FramePacketBuilder;
pub use frame_packet::PerFrameNode;
pub use frame_packet::PerViewNode;
pub use frame_packet::ViewNodeCount;
pub use frame_packet::ViewNodeIndex;

mod jobs;
pub use jobs::*;

mod registry;
pub use registry::RenderFeature;
pub use registry::RenderFeatureIndex;
pub use registry::RenderPhase;
pub use registry::RenderPhaseIndex;
pub use registry::RenderRegistry;
pub use registry::RenderRegistryBuilder;
pub use registry::MAX_RENDER_PHASE_COUNT;

mod macro_render_feature;
mod macro_render_phase;

#[derive(Default)]
pub struct VisibilityResult {
    pub handles: Vec<GenericRenderNodeHandle>,
}
