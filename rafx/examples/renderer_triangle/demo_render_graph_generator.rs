use crate::phases::OpaqueRenderPhase;
use rafx::api::*;
use rafx::assets::AssetManager;
use rafx::framework::render_features::{ExtractResources, RenderView};
use rafx::framework::{ImageViewResource, RenderResources, ResourceArc};
use rafx::graph::{
    PreparedRenderGraph, RenderGraphBuilder, RenderGraphImageConstraint, RenderGraphImageExtents,
    RenderGraphImageSpecification, RenderGraphQueue,
};
use rafx::render_features::RenderJobCommandBufferContext;
use rafx::renderer::{RenderGraphGenerator, SwapchainRenderResource};

pub struct DemoRenderGraphGenerator;

impl RenderGraphGenerator for DemoRenderGraphGenerator {
    fn generate_render_graph(
        &self,
        asset_manager: &AssetManager,
        swapchain_image: ResourceArc<ImageViewResource>,
        main_view: RenderView,
        _extract_resources: &ExtractResources,
        render_resources: &RenderResources,
    ) -> RafxResult<PreparedRenderGraph> {
        profiling::scope!("Build Render Graph");

        let device_context = asset_manager.device_context();
        let resource_context = asset_manager.resource_manager().resource_context();
        let swapchain_render_resource = render_resources.fetch::<SwapchainRenderResource>();
        let swapchain_info = swapchain_render_resource.get().unwrap();

        //
        // Create a graph to describe how we will draw the frame. Here we just have a single
        // renderpass with a color attachment. See the demo for more complex example usage.
        //
        let mut graph_builder = RenderGraphBuilder::default();

        let node = graph_builder.add_node("opaque", RenderGraphQueue::DefaultGraphics);
        let color_attachment = graph_builder.create_color_attachment(
            node,
            0,
            Some(RafxColorClearValue([0.0, 0.0, 0.0, 0.0])),
            RenderGraphImageConstraint {
                samples: Some(RafxSampleCount::SampleCount1),
                format: Some(swapchain_info.default_color_format_sdr),
                ..Default::default()
            },
            Default::default(),
        );
        graph_builder.set_image_name(color_attachment, "color");

        //
        // Set a callback to be run when the graph is executed.
        //
        graph_builder.set_renderpass_callback(node, move |args| {
            profiling::scope!("Opaque Pass");

            let mut write_context =
                RenderJobCommandBufferContext::from_graph_visit_render_pass_args(&args);

            args.graph_context
                .prepared_render_data()
                .write_view_phase::<OpaqueRenderPhase>(&main_view, &mut write_context)?;

            Ok(())
        });

        //
        // Flag the color attachment as needing to output to the swapchain image. This is not a
        // copy - the graph walks backwards from outputs so that it operates directly on the
        // intended output image where possible. It only creates additional resources if
        // necessary.
        //
        let _ = graph_builder.set_output_image(
            color_attachment,
            swapchain_image,
            RenderGraphImageSpecification {
                samples: RafxSampleCount::SampleCount1,
                format: swapchain_info.swapchain_surface_info.format,
                resource_type: RafxResourceType::TEXTURE | RafxResourceType::RENDER_TARGET_COLOR,
                extents: RenderGraphImageExtents::MatchSurface,
                layer_count: 1,
                mip_count: 1,
            },
            Default::default(),
            RafxResourceState::PRESENT,
        );

        let prepared_render_graph = PreparedRenderGraph::new(
            &device_context,
            &resource_context,
            graph_builder,
            &swapchain_info.swapchain_surface_info,
        )?;

        Ok(prepared_render_graph)
    }
}
