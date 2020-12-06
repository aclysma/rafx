use crate::features::mesh::ShadowMapRenderView;
use crate::render_contexts::RenderJobWriteContext;
use crate::VkDeviceContext;
use ash::prelude::VkResult;
use ash::vk;
use rafx::graph::*;
use rafx::nodes::{PreparedRenderData, RenderView};
use rafx::resources::ResourceContext;
use rafx::resources::{vk_description as dsc, VertexDataSetLayout};
use rafx::resources::{ImageViewResource, MaterialPassResource, ResourceArc};
use rafx::vulkan::SwapchainInfo;

mod shadow_map_pass;
use shadow_map_pass::ShadowMapImageResources;

mod opaque_pass;
use opaque_pass::OpaquePass;

mod bloom_extract_pass;
use bloom_extract_pass::BloomExtractPass;

mod bloom_blur_pass;

mod bloom_combine_pass;

mod ui_pass;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![])
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
struct RenderGraphConfig {
    color_format: vk::Format,
    depth_format: vk::Format,
    swapchain_format: vk::Format,
    samples: vk::SampleCountFlags,
    enable_hdr: bool,
    blur_pass_count: usize,
}

// This just wraps a bunch of values so they don't have to be passed individually to all the passes
struct RenderGraphContext<'a> {
    graph: &'a mut RenderGraphBuilder,
    graph_config: &'a RenderGraphConfig,
    graph_callbacks: &'a mut RenderGraphNodeCallbacks<RenderGraphUserContext>,
    main_view: &'a RenderView,
}

pub fn build_render_graph(
    device_context: &VkDeviceContext,
    resource_context: &ResourceContext,
    swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    swapchain_info: &SwapchainInfo,
    swapchain_image: ResourceArc<ImageViewResource>,
    main_view: RenderView,
    shadow_map_views: &[ShadowMapRenderView],
    bloom_extract_material_pass: ResourceArc<MaterialPassResource>,
    bloom_blur_material_pass: ResourceArc<MaterialPassResource>,
    bloom_combine_material_pass: ResourceArc<MaterialPassResource>,
) -> VkResult<BuildRenderGraphResult> {
    profiling::scope!("Build Render Graph");

    let enable_hdr = true;

    //TODO: Fix this back to be color format - need to happen in the combine pass
    let color_format = if enable_hdr {
        swapchain_surface_info.color_format
    } else {
        swapchain_surface_info.surface_format.format
    };

    let depth_format = swapchain_surface_info.depth_format;
    let swapchain_format = swapchain_surface_info.surface_format.format;
    let samples = swapchain_surface_info.msaa_level.into();

    let graph_config = RenderGraphConfig {
        color_format,
        depth_format,
        samples,
        enable_hdr,
        swapchain_format,
        blur_pass_count: 5,
    };

    let mut graph = RenderGraphBuilder::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphUserContext>::default();

    let mut graph_context = RenderGraphContext {
        graph: &mut graph,
        graph_callbacks: &mut graph_callbacks,
        graph_config: &graph_config,
        main_view: &main_view,
    };

    let shadow_maps = shadow_map_pass::shadow_map_passes(&mut graph_context, shadow_map_views);
    let opaque_pass = opaque_pass::opaque_pass(&mut graph_context, &shadow_maps);

    let previous_pass_color = if enable_hdr {
        let bloom_extract_pass = bloom_extract_pass::bloom_extract_pass(
            &mut graph_context,
            bloom_extract_material_pass,
            &opaque_pass,
        );
        let bloom_blur_pass = bloom_blur_pass::bloom_blur_pass(
            &mut graph_context,
            bloom_blur_material_pass,
            &bloom_extract_pass,
        );
        let bloom_combine_pass = bloom_combine_pass::bloom_combine_pass(
            &mut graph_context,
            bloom_combine_material_pass,
            &bloom_extract_pass,
            &bloom_blur_pass,
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
            samples: vk::SampleCountFlags::TYPE_1,
            format: swapchain_format,
            aspect_flags: vk::ImageAspectFlags::COLOR,
            usage_flags: swapchain_info.image_usage_flags,
            create_flags: Default::default(),
            extents: RenderGraphImageExtents::MatchSurface,
            layer_count: 1,
            mip_count: 1,
        },
        Default::default(),
        Default::default(),
        dsc::ImageLayout::PresentSrcKhr,
        vk::AccessFlags::empty(),
        vk::PipelineStageFlags::empty(),
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
