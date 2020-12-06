use ash::vk;

mod graph_builder;
pub use graph_builder::RenderGraphBuilder;
pub use graph_builder::RenderGraphQueue;

mod graph_image;
pub use graph_image::RenderGraphImageConstraint;
pub use graph_image::RenderGraphImageExtents;
pub use graph_image::RenderGraphImageSpecification;
pub use graph_image::RenderGraphImageSubresourceRange;
pub use graph_image::RenderGraphImageUsageId;
use graph_image::*;

mod graph_node;
pub use graph_node::RenderGraphNodeId;
use graph_node::*;

mod graph_pass;
use graph_pass::*;

mod graph_plan;
pub use graph_plan::RenderGraphPlan;

mod prepared_graph;
pub use prepared_graph::PreparedRenderGraph;
pub use prepared_graph::RenderGraphCache;
pub use prepared_graph::RenderGraphExecutor;
pub use prepared_graph::RenderGraphNodeCallbacks;
pub use prepared_graph::VisitRenderpassArgs;

// Test doesn't function because the graph now takes ResourceArc<ImageViewResource>
#[cfg(test)]
mod graph_tests;
