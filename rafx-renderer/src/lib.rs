mod render_feature_plugin;
pub use render_feature_plugin::RenderFeaturePlugin;

mod renderer_asset_plugin;
pub use renderer_asset_plugin::RendererAssetPlugin;

mod render_graph_generator;
pub use render_graph_generator::RenderGraphGenerator;

mod viewports_resource;
pub use viewports_resource::RenderViewMeta;
pub use viewports_resource::ViewportsResource;

mod render_thread;
use render_thread::RenderThread;

mod time_render_resource;
pub use time_render_resource::TimeRenderResource;

mod swapchain_render_resource;
pub use swapchain_render_resource::SwapchainRenderResource;

mod render_frame_job;
pub use render_frame_job::RenderFrameJob;

mod renderer_builder;
pub use renderer_builder::AssetSource;
pub use renderer_builder::RendererBuilder;
pub use renderer_builder::RendererBuilderResult;

mod renderer_thread_pool_none;

//TODO: Find a way to not expose this
mod swapchain_handling;
pub use swapchain_handling::SwapchainHandler;

mod renderer;
pub use renderer::*;

mod renderer_thread_pool;
pub use renderer_thread_pool::*;

pub mod daemon;
