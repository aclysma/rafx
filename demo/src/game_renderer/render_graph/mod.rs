use crate::features::mesh::ShadowMapRenderView;
use crate::render_contexts::RenderJobWriteContext;
use rafx::api::{
    RafxDeviceContext, RafxFormat, RafxPrimitiveTopology, RafxResourceState, RafxResourceType,
    RafxResult, RafxSampleCount,
};
use rafx::graph::*;
use rafx::nodes::{PreparedRenderData, RenderView};
use rafx::resources::VertexDataSetLayout;
use rafx::resources::{ComputePipelineResource, ResourceContext};
use rafx::resources::{ImageViewResource, MaterialPassResource, ResourceArc};

mod shadow_map_pass;
use shadow_map_pass::ShadowMapImageResources;

mod opaque_pass;
use opaque_pass::OpaquePass;

mod bloom_extract_pass;
use bloom_extract_pass::BloomExtractPass;

mod bloom_blur_pass;

mod bloom_combine_pass;

mod ui_pass;

mod compute_test;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

// Any data you want available within rendergraph execution callbacks should go here. This can
// include data that is not known until later after the extract/prepare phases have completed.
pub struct RenderGraphUserContext {
    pub prepared_render_data: Box<PreparedRenderData<RenderJobWriteContext>>,
}

// Everything produced by the graph. This includes resources that may be needed during the prepare
// phase
pub struct BuildRenderGraphResult {
    pub executor: RenderGraphExecutor<RenderGraphUserContext>,
    pub shadow_map_image_views: Vec<ResourceArc<ImageViewResource>>,
}

// All the data that can influence the rendergraph
pub struct RenderGraphConfig {
    pub color_format: RafxFormat,
    pub depth_format: RafxFormat,
    pub swapchain_format: RafxFormat,
    pub samples: RafxSampleCount,
    pub enable_hdr: bool,
    pub enable_bloom: bool,
    pub blur_pass_count: usize,
}

// This just wraps a bunch of values so they don't have to be passed individually to all the passes
struct RenderGraphContext<'a> {
    graph: &'a mut RenderGraphBuilder,
    graph_config: &'a RenderGraphConfig,
    graph_callbacks: &'a mut RenderGraphNodeCallbacks<RenderGraphUserContext>,
    main_view: &'a RenderView,
}

pub fn build_render_graph(
    device_context: &RafxDeviceContext,
    resource_context: &ResourceContext,
    graph_config: &RenderGraphConfig,
    swapchain_surface_info: &SwapchainSurfaceInfo,
    swapchain_image: ResourceArc<ImageViewResource>,
    main_view: RenderView,
    shadow_map_views: &[ShadowMapRenderView],
    bloom_extract_material_pass: ResourceArc<MaterialPassResource>,
    bloom_blur_material_pass: ResourceArc<MaterialPassResource>,
    bloom_combine_material_pass: ResourceArc<MaterialPassResource>,
    test_compute_pipeline: &ResourceArc<ComputePipelineResource>,
) -> RafxResult<BuildRenderGraphResult> {
    profiling::scope!("Build Render Graph");

    let mut graph = RenderGraphBuilder::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphUserContext>::default();

    let mut graph_context = RenderGraphContext {
        graph: &mut graph,
        graph_callbacks: &mut graph_callbacks,
        graph_config,
        main_view: &main_view,
    };

    let shadow_maps = shadow_map_pass::shadow_map_passes(&mut graph_context, shadow_map_views);

    let compute_test_pass =
        compute_test::compute_test_pass(&mut graph_context, test_compute_pipeline);

    let opaque_pass = opaque_pass::opaque_pass(&mut graph_context, &shadow_maps);
    {
        let _out = graph_context.graph.read_storage_buffer(
            opaque_pass.node,
            compute_test_pass.position_buffer,
            RenderGraphBufferConstraint {
                ..Default::default()
            },
        );
    }

    let previous_pass_color = if graph_config.enable_hdr {
        let bloom_extract_pass = bloom_extract_pass::bloom_extract_pass(
            &mut graph_context,
            bloom_extract_material_pass,
            &opaque_pass,
        );

        let blurred_color = if graph_config.enable_bloom && graph_config.blur_pass_count > 0 {
            let bloom_blur_pass = bloom_blur_pass::bloom_blur_pass(
                &mut graph_context,
                bloom_blur_material_pass,
                &bloom_extract_pass,
            );
            bloom_blur_pass.color
        } else {
            bloom_extract_pass.hdr_image
        };

        let bloom_combine_pass = bloom_combine_pass::bloom_combine_pass(
            &mut graph_context,
            bloom_combine_material_pass,
            &bloom_extract_pass,
            blurred_color,
        );

        bloom_combine_pass.color
    } else {
        opaque_pass.color
    };

    let ui_pass = ui_pass::ui_pass(&mut graph_context, previous_pass_color);

    let _swapchain_output_image_id = graph.set_output_image(
        ui_pass.color,
        swapchain_image,
        RenderGraphImageSpecification {
            samples: RafxSampleCount::SampleCount1,
            format: graph_config.swapchain_format,
            resource_type: RafxResourceType::TEXTURE,
            extents: RenderGraphImageExtents::MatchSurface,
            layer_count: 1,
            mip_count: 1,
        },
        Default::default(),
        RafxResourceState::PRESENT,
    );

    //
    // Create the executor, it needs to have access to the resource manager to add framebuffers
    // and renderpasses to the resource lookups
    //
    let executor = RenderGraphExecutor::new(
        &device_context,
        &resource_context,
        graph,
        swapchain_surface_info,
        graph_callbacks,
    )?;

    let shadow_map_image_views = opaque_pass
        .shadow_maps
        .iter()
        .map(|&x| executor.image_view_resource(x).unwrap())
        .collect();

    Ok(BuildRenderGraphResult {
        shadow_map_image_views,
        executor,
    })
}
