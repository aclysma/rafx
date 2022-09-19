use rafx::api::{
    RafxExtents3D, RafxFormat, RafxPrimitiveTopology, RafxResourceState, RafxResourceType,
    RafxResult, RafxSampleCount, RafxTextureDef, RafxTextureDimensions,
};
use rafx::framework::VertexDataSetLayout;
use rafx::framework::{ImageViewResource, ResourceArc};
use rafx::framework::{RenderResources, ResourceContext};
use rafx::graph::*;
use rafx::render_features::{ExtractResources, RenderView};

mod shadow_map_pass;

mod opaque_pass;

mod depth_prepass;

mod ssao_pass;

mod bloom_extract_pass;
use super::ModernPipelineRenderOptions;
use super::ModernPipelineStaticResources;
use crate::features::debug_pip::DebugPipRenderResource;
use crate::features::mesh_adv::{MeshAdvRenderPipelineState, ShadowMapAtlas};
use crate::pipelines::modern::{
    AntiAliasMethodAdv, ModernPipelineMeshCullingDebugData, ModernPipelineTonemapDebugData,
};
use rafx::assets::AssetManager;
use rafx::renderer::SwapchainRenderResource;
use rafx::renderer::TimeRenderResource;

mod bloom_blur_pass;

mod bloom_combine_pass;

mod light_binning;

mod luma_pass;

mod debug_pip_pass;

mod ui_pass;

mod taa_pass;

mod cas_pass;

mod mesh_culling;

mod depth_pyramid;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

// All the data that can influence the rendergraph
pub struct ModernPipelineRenderGraphConfig {
    pub color_format: RafxFormat,
    pub depth_format: RafxFormat,
    pub swapchain_format: RafxFormat,
    pub samples: RafxSampleCount,
    pub enable_hdr: bool,
    pub enable_ssao: bool,
    pub enable_bloom: bool,
    pub show_surfaces: bool,
    pub blur_pass_count: usize,
    pub jitter_amount: glam::Vec2,
    pub sharpening_amount: f32,
}

// This just wraps a bunch of values so they don't have to be passed individually to all the passes
struct ModernPipelineContext<'a> {
    graph: &'a mut RenderGraphBuilder,
    #[allow(dead_code)]
    resource_context: &'a ResourceContext,
    asset_manager: &'a AssetManager,
    graph_config: &'a ModernPipelineRenderGraphConfig,
    main_view: &'a RenderView,
    extract_resources: &'a ExtractResources<'a>,
    render_resources: &'a RenderResources,
}

pub(super) fn generate_render_graph(
    asset_manager: &AssetManager,
    swapchain_image: ResourceArc<ImageViewResource>,
    rotating_frame_index: usize,
    main_view: RenderView,
    extract_resources: &ExtractResources,
    render_resources: &RenderResources,
) -> RafxResult<PreparedRenderGraph> {
    profiling::scope!("Build Render Graph");
    let device_context = asset_manager.device_context();
    let resource_context = asset_manager.resource_manager().resource_context();
    let swapchain_render_resource = render_resources.fetch::<SwapchainRenderResource>();
    let swapchain_info = swapchain_render_resource.surface_info().unwrap();
    let swapchain_extents = swapchain_info.swapchain_surface_info.extents;
    let mut static_resources = render_resources.fetch_mut::<ModernPipelineStaticResources>();
    let mut shadow_atlas = render_resources.fetch_mut::<ShadowMapAtlas>();
    let previous_update_dt = render_resources
        .fetch::<TimeRenderResource>()
        .previous_update_dt();

    if let Some(taa_history_rt) = &static_resources.taa_history_rt {
        let taa_history_rt_extents = taa_history_rt
            .get_raw()
            .image
            .get_raw()
            .image
            .texture_def()
            .extents;
        if swapchain_extents.width != taa_history_rt_extents.width
            || swapchain_extents.height != taa_history_rt_extents.height
        {
            static_resources.taa_history_rt = None;
        }
    }

    render_resources
        .fetch_mut::<DebugPipRenderResource>()
        .clear();

    let render_options = extract_resources
        .fetch::<ModernPipelineRenderOptions>()
        .clone();

    // Configure options for this frame
    let graph_config = {
        let swapchain_format = swapchain_info.swapchain_surface_info.format;
        let sample_count = if render_options.anti_alias_method == AntiAliasMethodAdv::Msaa4x {
            RafxSampleCount::SampleCount4
        } else {
            RafxSampleCount::SampleCount1
        };

        let color_format = if render_options.enable_hdr {
            RafxFormat::R16G16B16A16_SFLOAT
        } else {
            swapchain_info.default_color_format_sdr
        };

        let jitter_amount = if render_options.anti_alias_method == AntiAliasMethodAdv::Taa {
            super::internal::jitter::jitter_amount(
                main_view.frame_index(),
                render_options.taa_options.jitter_pattern,
                main_view.extents_vec2(),
            ) * render_options.taa_options.jitter_multiplier
        } else {
            glam::Vec2::ZERO
        };

        ModernPipelineRenderGraphConfig {
            color_format,
            depth_format: swapchain_info.default_depth_format,
            samples: sample_count,
            enable_hdr: render_options.enable_hdr,
            swapchain_format,
            enable_ssao: render_options.enable_ssao
                && sample_count == RafxSampleCount::SampleCount1,
            enable_bloom: render_options.enable_bloom,
            show_surfaces: render_options.show_surfaces,
            blur_pass_count: render_options.blur_pass_count,
            jitter_amount,
            sharpening_amount: render_options.sharpening_amount,
        }
    };

    // Push pipeline options into the mesh feature
    {
        let mut mesh_render_pipeline_state =
            render_resources.fetch_mut::<MeshAdvRenderPipelineState>();
        mesh_render_pipeline_state.jitter_amount = graph_config.jitter_amount;
        mesh_render_pipeline_state.forward_pass_mip_bias =
            render_options.taa_options.forward_pass_mip_bias;
    }

    let mut taa_history_rt_has_data = false;
    let taa_history_rt = if render_options.anti_alias_method == AntiAliasMethodAdv::Taa {
        let required_extents = RafxExtents3D {
            width: swapchain_extents.width,
            height: swapchain_extents.height,
            depth: 1,
        };

        let mut history_texture_compatible = false;
        if let Some(taa_history) = &static_resources.taa_history_rt {
            let history_image = taa_history.get_raw().image.get_raw().image.clone();
            let history_def = history_image.texture_def();
            history_texture_compatible = history_def.sample_count == graph_config.samples
                && history_def.format == graph_config.color_format
                && history_def.extents == required_extents;
        }

        if !history_texture_compatible {
            let taa_history_rt =
                asset_manager
                    .device_context()
                    .create_texture(&RafxTextureDef {
                        resource_type: RafxResourceType::RENDER_TARGET_COLOR
                            | RafxResourceType::TEXTURE,
                        sample_count: graph_config.samples,
                        format: graph_config.color_format,
                        extents: RafxExtents3D {
                            width: swapchain_extents.width,
                            height: swapchain_extents.height,
                            depth: 1,
                        },
                        dimensions: RafxTextureDimensions::Dim2D,
                        ..Default::default()
                    })?;
            taa_history_rt.set_debug_name("TAA History RT");
            let taa_history_rt = asset_manager.resources().insert_image(taa_history_rt);
            let taa_history_rt = asset_manager
                .resources()
                .get_or_create_image_view(&taa_history_rt, None)?;
            static_resources.taa_history_rt = Some(taa_history_rt.clone());
            Some(taa_history_rt)
        } else {
            taa_history_rt_has_data = true;
            Some(static_resources.taa_history_rt.clone().unwrap())
        }
    } else {
        static_resources.taa_history_rt = None;
        None
    };

    let tonemap_debug_data = extract_resources
        .try_fetch::<ModernPipelineTonemapDebugData>()
        .map(|x| x.clone());

    let mesh_culling_debug_data = extract_resources
        .try_fetch::<ModernPipelineMeshCullingDebugData>()
        .map(|x| x.clone());

    let mut graph = RenderGraphBuilder::default();

    let mut graph_context = ModernPipelineContext {
        graph: &mut graph,
        resource_context: &resource_context,
        asset_manager,
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

    let shadow_atlas_image = shadow_atlas.add_to_render_graph(graph_context.graph);
    let shadow_atlas_needs_full_clear = shadow_atlas.take_requires_full_clear();
    drop(shadow_atlas);

    let tonemap_histogram_result = graph_context.graph.add_external_buffer(
        static_resources.tonemap_histogram_result.clone(),
        RafxResourceState::UNORDERED_ACCESS,
        RafxResourceState::UNORDERED_ACCESS,
    );

    let tonemap_debug_output = graph_context.graph.add_external_buffer(
        static_resources.tonemap_debug_output[rotating_frame_index].clone(),
        RafxResourceState::UNORDERED_ACCESS,
        RafxResourceState::UNORDERED_ACCESS,
    );

    let mesh_culling_debug_output = graph_context.graph.add_external_buffer(
        static_resources.mesh_culling_debug_output[rotating_frame_index].clone(),
        RafxResourceState::UNORDERED_ACCESS,
        RafxResourceState::UNORDERED_ACCESS,
    );

    let depth_prepass = depth_prepass::depth_prepass(&mut graph_context);

    let depth_pyramid_pipeline = asset_manager
        .committed_asset(&static_resources.depth_pyramid_pipeline)
        .unwrap()
        .compute_pipeline
        .clone();

    let depth_pyramid_pass = depth_pyramid::depth_pyramid_pass(
        &mut graph_context,
        &depth_pyramid_pipeline,
        depth_prepass.depth,
        &swapchain_info.swapchain_surface_info,
    );

    let ssao_material_pass = asset_manager
        .committed_asset(&static_resources.ssao_material)
        .unwrap()
        .get_single_material_pass()
        .unwrap();
    let noise_texture = asset_manager
        .committed_asset(&static_resources.blue_noise_texture)
        .unwrap()
        .image_view
        .clone();

    let ssao_rt = if graph_config.enable_ssao {
        let ssao_pass = ssao_pass::ssao_pass(
            &mut graph_context,
            &ssao_material_pass,
            depth_prepass.depth,
            &noise_texture,
        );

        let mut ssao_rt = ssao_pass.ssao_rt;

        let bloom_blur_material_pass = asset_manager
            .committed_asset(&static_resources.bloom_blur_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        ssao_rt = bloom_blur_pass::blur_pass(
            &mut graph_context,
            bloom_blur_material_pass.clone(),
            ssao_rt,
            1,
        )
        .color;

        Some(ssao_rt)
    } else {
        None
    };

    let shadow_map_pass_output = shadow_map_pass::shadow_map_passes(
        &mut graph_context,
        shadow_atlas_image,
        shadow_atlas_needs_full_clear,
    );

    let light_bin_pass = light_binning::lights_bin_pass(&mut graph_context);
    let build_light_lists_pass =
        light_binning::lights_build_lists_pass(&mut graph_context, light_bin_pass);

    let mesh_culling_pipeline = asset_manager
        .committed_asset(&static_resources.mesh_culling_pipeline)
        .unwrap()
        .compute_pipeline
        .clone();

    let mesh_culling_node = if render_options.enable_occlusion_culling {
        // No outputs here because the buffer is being managed outside the render graph
        Some(mesh_culling::mesh_culling_pass(
            &mut graph_context,
            &mesh_culling_pipeline,
            &swapchain_info.swapchain_surface_info,
            &depth_pyramid_pass,
            mesh_culling_debug_data,
            mesh_culling_debug_output,
        ))
    } else {
        None
    };

    let opaque_pass = opaque_pass::opaque_pass(
        &mut graph_context,
        depth_prepass.depth,
        &shadow_map_pass_output,
        &build_light_lists_pass,
        ssao_rt,
    );

    if render_options.enable_occlusion_culling {
        graph_context
            .graph
            .add_explicit_dependency(mesh_culling_node.unwrap().node, opaque_pass.node);
    }

    let taa_material_pass = asset_manager
        .committed_asset(&static_resources.taa_material)
        .unwrap()
        .get_single_material_pass()
        .unwrap();

    let color_rt = if render_options.anti_alias_method == AntiAliasMethodAdv::Taa {
        let taa_history_rt_image_id = graph_context.graph.add_external_image(
            taa_history_rt.unwrap(),
            Default::default(),
            RafxResourceState::COPY_DST,
            RafxResourceState::COPY_DST,
        );

        let taa_pass = taa_pass::taa_pass(
            &mut graph_context,
            &render_options.taa_options,
            taa_material_pass,
            opaque_pass.color,
            depth_prepass.depth,
            depth_prepass.velocity_rt,
            taa_history_rt_image_id,
            taa_history_rt_has_data,
        );

        taa_pass.color_rt
    } else {
        opaque_pass.color
    };

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

        let luma_build_histogram = asset_manager
            .committed_asset(&static_resources.luma_build_histogram)
            .unwrap()
            .compute_pipeline
            .clone();

        let luma_average_histogram = asset_manager
            .committed_asset(&static_resources.luma_average_histogram)
            .unwrap()
            .compute_pipeline
            .clone();

        let cas_pipeline = asset_manager
            .committed_asset(&static_resources.cas_pipeline)
            .unwrap()
            .compute_pipeline
            .clone();

        let bloom_extract_pass = bloom_extract_pass::bloom_extract_pass(
            &mut graph_context,
            bloom_extract_material_pass,
            color_rt,
        );

        let cas_pass = cas_pass::cas_pass(
            &mut graph_context,
            &cas_pipeline,
            bloom_extract_pass.sdr_image,
            &swapchain_info.swapchain_surface_info,
        );

        let luma_build_histogram_pass = luma_pass::luma_build_histogram_pass(
            &mut graph_context,
            &luma_build_histogram,
            color_rt,
            &swapchain_info.swapchain_surface_info,
        );

        let luma_average_histogram_pass = luma_pass::luma_average_histogram_pass(
            &mut graph_context,
            &luma_build_histogram_pass,
            &luma_average_histogram,
            tonemap_histogram_result,
            tonemap_debug_data,
            tonemap_debug_output,
            &swapchain_info.swapchain_surface_info,
            previous_update_dt,
        );

        let blurred_color = if graph_config.enable_bloom && graph_config.blur_pass_count > 0 {
            let bloom_blur_pass = bloom_blur_pass::bloom_blur_pass(
                &mut graph_context,
                bloom_blur_material_pass,
                bloom_extract_pass.hdr_image,
            );
            bloom_blur_pass.color
        } else {
            bloom_extract_pass.hdr_image
        };

        let sdr_image = if render_options.enable_sharpening {
            cas_pass.color_rt
        } else {
            bloom_extract_pass.sdr_image
        };

        let bloom_combine_pass = bloom_combine_pass::bloom_combine_pass(
            &mut graph_context,
            bloom_combine_material_pass,
            sdr_image,
            blurred_color,
            &luma_average_histogram_pass,
            &*swapchain_render_resource,
        );

        bloom_combine_pass.color
    } else {
        color_rt
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

    Ok(prepared_render_graph)
}
