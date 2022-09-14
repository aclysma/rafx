mod graph_builder;
pub use graph_builder::RenderGraphBuilder;
pub use graph_builder::RenderGraphQueue;

mod graph_image;
pub use graph_image::RenderGraphExternalImageId;
pub use graph_image::RenderGraphImageConstraint;
pub use graph_image::RenderGraphImageExtents;
pub use graph_image::RenderGraphImageSpecification;
pub use graph_image::RenderGraphImageUsageId;
pub use graph_image::RenderGraphImageViewOptions;
use graph_image::*;

mod graph_buffer;
pub use graph_buffer::RenderGraphBufferConstraint;
pub use graph_buffer::RenderGraphBufferSpecification;
pub use graph_buffer::RenderGraphBufferUsageId;
pub use graph_buffer::RenderGraphExternalBufferId;
use graph_buffer::*;

mod graph_node;
pub use graph_node::RenderGraphNodeId;
use graph_node::*;

mod graph_pass;
use graph_pass::*;

mod graph_plan;
pub use graph_plan::RenderGraphPlan;

mod graph_resource_cache;
pub use graph_resource_cache::RenderGraphCache;

mod prepared_graph;
pub use prepared_graph::OnBeginExecuteGraphArgs;
pub use prepared_graph::PreparedRenderGraph;
pub use prepared_graph::RenderGraphContext;
pub use prepared_graph::SwapchainSurfaceInfo;
pub use prepared_graph::VisitComputeNodeArgs;
pub use prepared_graph::VisitRenderpassNodeArgs;

use rafx_api::RafxResult;

pub type RenderGraphResourceName = &'static str;

enum RenderGraphNodeVisitNodeCallback {
    Render(Box<RenderGraphNodeVisitRenderpassNodeCallback>),
    Callback(Box<RenderGraphNodeVisitCallbackNodeCallback>),
}

type RenderGraphNodeVisitRenderpassNodeCallback =
    dyn Fn(VisitRenderpassNodeArgs) -> RafxResult<()> + Send;

type RenderGraphNodeVisitCallbackNodeCallback =
    dyn Fn(VisitComputeNodeArgs) -> RafxResult<()> + Send;
