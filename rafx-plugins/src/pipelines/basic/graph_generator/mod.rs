use rafx::api::{
    RafxFormat, RafxPrimitiveTopology, RafxResourceState, RafxResult, RafxSampleCount,
};
use rafx::framework::VertexDataSetLayout;
use rafx::framework::{ImageViewResource, ResourceArc};
use rafx::framework::{RenderResources, ResourceContext};
use rafx::graph::*;
use rafx::render_features::{ExtractResources, RenderView};

mod shadow_map_pass;
use shadow_map_pass::ShadowMapImageResources;

mod opaque_pass;
use opaque_pass::OpaquePass;

mod depth_prepass;

mod bloom_extract_pass;
use super::BasicPipelineRenderOptions;
use super::BasicPipelineStaticResources;
use crate::features::debug_pip::DebugPipRenderResource;
use crate::features::mesh_basic::MeshBasicShadowMapResource;
use crate::pipelines::basic::AntiAliasMethodBasic;
use bloom_extract_pass::BloomExtractPass;
use rafx::assets::AssetManager;
use rafx::renderer::SwapchainRenderResource;

mod bloom_blur_pass;

mod bloom_combine_pass;

mod debug_pip_pass;

mod ui_pass;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

// All the data that can influence the rendergraph
pub struct BasicPipelineRenderGraphConfig {
    pub color_format: RafxFormat,
    pub depth_format: RafxFormat,
    pub swapchain_format: RafxFormat,
    pub samples: RafxSampleCount,
    pub enable_hdr: bool,
    pub enable_bloom: bool,
    pub show_surfaces: bool,
    pub blur_pass_count: usize,
}

// This just wraps a bunch of values so they don't have to be passed individually to all the passes
struct BasicPipelineContext<'a> {
    graph: &'a mut RenderGraphBuilder,
    #[allow(dead_code)]
    resource_context: &'a ResourceContext,
    graph_config: &'a BasicPipelineRenderGraphConfig,
    main_view: &'a RenderView,
    extract_resources: &'a ExtractResources<'a>,
    render_resources: &'a RenderResources,
}

pub(super) fn generate_render_graph(
    asset_manager: &AssetManager,
    swapchain_image: ResourceArc<ImageViewResource>,
    _rotating_frame_index: usize,
    main_view: RenderView,
    extract_resources: &ExtractResources,
    render_resources: &RenderResources,
) -> RafxResult<PreparedRenderGraph> {
    profiling::scope!("Build Render Graph");

    let device_context = asset_manager.device_context();
    let resource_context = asset_manager.resource_manager().resource_context();
    let swapchain_render_resource = render_resources.fetch::<SwapchainRenderResource>();
    let swapchain_info = swapchain_render_resource.surface_info().unwrap();
    let static_resources = render_resources.fetch::<BasicPipelineStaticResources>();

    render_resources
        .fetch_mut::<DebugPipRenderResource>()
        .clear();

    let graph_config = {
        let render_options = extract_resources
            .fetch::<BasicPipelineRenderOptions>()
            .clone();
        let swapchain_format = swapchain_info.swapchain_surface_info.format;
        let sample_count = if render_options.anti_alias_method == AntiAliasMethodBasic::Msaa4x {
            RafxSampleCount::SampleCount4
        } else {
            RafxSampleCount::SampleCount1
        };

        let color_format = if render_options.enable_hdr {
            swapchain_info.default_color_format_hdr
        } else {
            swapchain_info.default_color_format_sdr
        };

        BasicPipelineRenderGraphConfig {
            color_format,
            depth_format: swapchain_info.default_depth_format,
            samples: sample_count,
            enable_hdr: render_options.enable_hdr,
            swapchain_format,
            enable_bloom: render_options.enable_bloom,
            show_surfaces: render_options.show_surfaces,
            blur_pass_count: render_options.blur_pass_count,
        }
    };

    let mut graph = RenderGraphBuilder::default();

    let mut graph_context = BasicPipelineContext {
        graph: &mut graph,
        resource_context: &resource_context,
        graph_config: &graph_config,
        main_view: &main_view,
        render_resources,
        extract_resources,
    };

    let swapchain_image_id = graph_context.graph.add_external_image(
        swapchain_image,
        Default::default(),
        RafxResourceState::PRESENT,
        RafxResourceState::PRESENT,
    );

    let depth_prepass = depth_prepass::depth_prepass(&mut graph_context);

    let shadow_maps = shadow_map_pass::shadow_map_passes(&mut graph_context);

    let opaque_pass = opaque_pass::opaque_pass(&mut graph_context, depth_prepass, &shadow_maps);

    let mut previous_pass_color = if graph_config.enable_hdr {
        let bloom_extract_material_pass = asset_manager
            .committed_asset(&static_resources.bloom_extract_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        let bloom_blur_material_pass = asset_manager
            .committed_asset(&static_resources.bloom_blur_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        let bloom_combine_material_pass = asset_manager
            .committed_asset(&static_resources.bloom_combine_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

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
            &*swapchain_render_resource,
        );

        bloom_combine_pass.color
    } else {
        opaque_pass.color
    };

    previous_pass_color =
        debug_pip_pass::debug_pip_pass(&mut graph_context, previous_pass_color).color;

    previous_pass_color = ui_pass::ui_pass(&mut graph_context, previous_pass_color).color;

    graph.write_external_image(swapchain_image_id, previous_pass_color);

    let prepared_render_graph = PreparedRenderGraph::new(
        &device_context,
        &resource_context,
        graph,
        &swapchain_info.swapchain_surface_info,
    )?;

    render_resources
        .fetch_mut::<MeshBasicShadowMapResource>()
        .set_shadow_map_image_views(&prepared_render_graph);

    Ok(prepared_render_graph)
}
