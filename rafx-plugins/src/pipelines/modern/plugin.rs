use crate::phases::{
    DebugPipRenderPhase, DepthPrepassRenderPhase, OpaqueRenderPhase, PostProcessRenderPhase,
    ShadowMapRenderPhase, TransparentRenderPhase, UiRenderPhase, WireframeRenderPhase,
};
use crate::shaders::mesh_adv::mesh_culling_comp;
use crate::shaders::post_adv::luma_average_histogram_comp;
use hydrate_base::handle::Handle;
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::{
    RafxBufferDef, RafxFormat, RafxMemoryUsage, RafxQueueType, RafxResourceType, RafxResult,
};
use rafx::assets::AssetResource;
use rafx::assets::{AssetManager, ComputePipelineAsset, ImageAsset, MaterialAsset};
use rafx::framework::{
    BufferResource, ImageViewResource, RenderResources, ResourceArc, MAX_FRAMES_IN_FLIGHT,
};
use rafx::graph::PreparedRenderGraph;
use rafx::render_features::{ExtractResources, RenderRegistryBuilder, RenderView};
use rafx::renderer::{RendererLoadContext, RendererPipelinePlugin};
use std::sync::{Arc, Mutex};

// A plugin that add demo-specific configuration

#[derive(Debug)]
pub struct ModernPipelineTonemapDebugDataInner {
    // The values will only be updated if this is set to true
    pub enable_debug_data_collection: bool,

    pub result_average: f32,
    pub result_average_bin: f32,
    pub result_min_bin: u32,
    pub result_low_bin: u32,
    pub result_high_bin: u32,
    pub result_max_bin: u32,
    pub histogram: [u32; 256],
    pub histogram_sample_count: u32,
    pub histogram_max_value: u32,
}

impl Default for ModernPipelineTonemapDebugDataInner {
    fn default() -> Self {
        ModernPipelineTonemapDebugDataInner {
            enable_debug_data_collection: false,
            result_average: 0.0,
            result_average_bin: 0.0,
            result_min_bin: 0,
            result_low_bin: 0,
            result_high_bin: 0,
            result_max_bin: 0,
            histogram_sample_count: 0,
            histogram_max_value: 0,
            histogram: [0; 256],
        }
    }
}

#[derive(Clone)]
pub struct ModernPipelineTonemapDebugData {
    pub inner: Arc<Mutex<ModernPipelineTonemapDebugDataInner>>,
}

impl Default for ModernPipelineTonemapDebugData {
    fn default() -> Self {
        ModernPipelineTonemapDebugData {
            inner: Arc::new(Mutex::new(ModernPipelineTonemapDebugDataInner::default())),
        }
    }
}

#[derive(Debug)]
pub struct ModernPipelineMeshCullingDebugDataInner {
    // The values will only be updated if this is set to true
    pub enable_debug_data_collection: bool,

    pub culled_mesh_count: u32,
    pub total_mesh_count: u32,
    pub culled_primitive_count: u32,
    pub total_primitive_count: u32,
}

impl Default for ModernPipelineMeshCullingDebugDataInner {
    fn default() -> Self {
        ModernPipelineMeshCullingDebugDataInner {
            enable_debug_data_collection: false,
            culled_mesh_count: 0,
            total_mesh_count: 0,
            culled_primitive_count: 0,
            total_primitive_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct ModernPipelineMeshCullingDebugData {
    pub inner: Arc<Mutex<ModernPipelineMeshCullingDebugDataInner>>,
}

impl Default for ModernPipelineMeshCullingDebugData {
    fn default() -> Self {
        ModernPipelineMeshCullingDebugData {
            inner: Arc::new(Mutex::new(
                ModernPipelineMeshCullingDebugDataInner::default(),
            )),
        }
    }
}

pub struct ModernPipelineStaticResources {
    pub bloom_extract_material: Handle<MaterialAsset>,
    pub bloom_blur_material: Handle<MaterialAsset>,
    pub bloom_combine_material: Handle<MaterialAsset>,
    pub ssao_material: Handle<MaterialAsset>,
    pub blue_noise_texture: Handle<ImageAsset>,
    pub taa_material: Handle<MaterialAsset>,
    pub luma_build_histogram: Handle<ComputePipelineAsset>,
    pub luma_average_histogram: Handle<ComputePipelineAsset>,
    pub cas_pipeline: Handle<ComputePipelineAsset>,
    pub mesh_culling_pipeline: Handle<ComputePipelineAsset>,
    pub depth_pyramid_pipeline: Handle<ComputePipelineAsset>,
    pub tonemap_histogram_result: ResourceArc<BufferResource>,
    pub tonemap_debug_output: Vec<ResourceArc<BufferResource>>,
    pub mesh_culling_debug_output: Vec<ResourceArc<BufferResource>>,
    pub taa_history_rt: Option<ResourceArc<ImageViewResource>>,
}

pub struct ModernPipelineRendererPlugin;

impl RendererPipelinePlugin for ModernPipelineRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry_builder: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry_builder
            .register_render_phase::<DepthPrepassRenderPhase>("DepthPrepass")
            .register_render_phase::<ShadowMapRenderPhase>("ShadowMap")
            .register_render_phase::<OpaqueRenderPhase>("Opaque")
            .register_render_phase::<TransparentRenderPhase>("Transparent")
            .register_render_phase::<WireframeRenderPhase>("Wireframe")
            .register_render_phase::<PostProcessRenderPhase>("PostProcess")
            .register_render_phase::<DebugPipRenderPhase>("DebugPipRenderPhase")
            .register_render_phase::<UiRenderPhase>("Ui")
    }

    fn initialize_static_resources(
        &self,
        renderer_load_context: &RendererLoadContext,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut RenderResources,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        //
        // Bloom extract resources
        //
        // let bloom_extract_material = asset_resource
        //     .load_artifact_symbol_name::<MaterialAsset, _>("pipelines/bloom_extract.material");
        let bloom_extract_material = asset_resource.load_artifact_symbol_name::<MaterialAsset>(
            "rafx-plugins://materials/bloom_extract.material",
        );

        //
        // Bloom blur resources
        //
        let bloom_blur_material = asset_resource.load_artifact_symbol_name::<MaterialAsset>(
            "rafx-plugins://materials/bloom_blur.material",
        );

        let ssao_material = asset_resource.load_artifact_symbol_name::<MaterialAsset>(
            "rafx-plugins://materials/modern_pipeline/ssao.material",
        );

        let blue_noise_texture = asset_resource.load_artifact_symbol_name::<ImageAsset>(
            "rafx-plugins://images/blue_noise/LDR_RGBA_64_64_0.png",
        );

        let taa_material = asset_resource.load_artifact_symbol_name::<MaterialAsset>(
            "rafx-plugins://materials/modern_pipeline/taa.material",
        );

        //
        // Bloom combine resources
        //
        let bloom_combine_material = asset_resource.load_artifact_symbol_name::<MaterialAsset>(
            "rafx-plugins://materials/modern_pipeline/bloom_combine_adv.material",
        );

        let luma_build_histogram = asset_resource
            .load_artifact_symbol_name::<ComputePipelineAsset>(
                "rafx-plugins://compute_pipelines/luma_build_histogram.compute",
            );

        let luma_average_histogram = asset_resource
            .load_artifact_symbol_name::<ComputePipelineAsset>(
                "rafx-plugins://compute_pipelines/luma_average_histogram.compute",
            );

        let cas_asset_path = if asset_manager.device_context().is_vulkan() {
            //TODO: Validation errors if trying to use f16 on vulkan
            "rafx-plugins://compute_pipelines/cas32.compute"
        } else {
            "rafx-plugins://compute_pipelines/cas16.compute"
        };

        let cas_pipeline =
            asset_resource.load_artifact_symbol_name::<ComputePipelineAsset>(cas_asset_path);

        let mesh_culling_pipeline = asset_resource
            .load_artifact_symbol_name::<ComputePipelineAsset>(
                "rafx-plugins://compute_pipelines/mesh_culling.compute",
            );

        let depth_pyramid_pipeline = asset_resource
            .load_artifact_symbol_name::<ComputePipelineAsset>(
                "rafx-plugins://compute_pipelines/depth_pyramid.compute",
            );

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &bloom_extract_material,
            asset_resource,
            "bloom extract material",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &bloom_blur_material,
            asset_resource,
            "bloom blur material",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &ssao_material,
            asset_resource,
            "ssao material",
        )?;
        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &taa_material,
            asset_resource,
            "taa material",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &bloom_combine_material,
            asset_resource,
            "bloom combine material",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &luma_build_histogram,
            asset_resource,
            "luma_build_histogram",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &luma_average_histogram,
            asset_resource,
            "luma_average_histogram",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &cas_pipeline,
            asset_resource,
            "cas_pipeline",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &mesh_culling_pipeline,
            asset_resource,
            "mesh_culling_pipeline",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &depth_pyramid_pipeline,
            asset_resource,
            "depth_pyramid_pipeline",
        )?;

        let tonemap_histogram_result =
            asset_manager
                .device_context()
                .create_buffer(&RafxBufferDef {
                    size: std::mem::size_of::<luma_average_histogram_comp::HistogramResultBuffer>()
                        as u64,
                    alignment: 256,
                    memory_usage: RafxMemoryUsage::GpuOnly,
                    queue_type: RafxQueueType::Graphics,
                    resource_type: RafxResourceType::BUFFER_READ_WRITE,
                    elements: Default::default(),
                    format: RafxFormat::UNDEFINED,
                    always_mapped: false,
                })?;
        tonemap_histogram_result.set_debug_name("Tonemap Histogram Result");

        let tonemap_histogram_result = asset_manager
            .resource_manager()
            .resources()
            .insert_buffer(tonemap_histogram_result);

        let mut tonemap_debug_output = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT + 1);
        for i in 0..=MAX_FRAMES_IN_FLIGHT {
            let tonemap_debug_output_buffer =
                asset_manager
                    .device_context()
                    .create_buffer(&RafxBufferDef {
                        size: std::mem::size_of::<luma_average_histogram_comp::DebugOutputBuffer>()
                            as u64,
                        alignment: 256,
                        memory_usage: RafxMemoryUsage::GpuToCpu,
                        queue_type: RafxQueueType::Graphics,
                        resource_type: RafxResourceType::BUFFER_READ_WRITE,
                        elements: Default::default(),
                        format: RafxFormat::UNDEFINED,
                        always_mapped: false,
                    })?;
            tonemap_debug_output_buffer
                .set_debug_name(format!("Tonemap Debug Output Buffer {}", i));
            tonemap_debug_output.push(
                asset_manager
                    .resource_manager()
                    .resources()
                    .insert_buffer(tonemap_debug_output_buffer),
            );
        }

        let mut mesh_culling_debug_output = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT + 1);
        for i in 0..=MAX_FRAMES_IN_FLIGHT {
            let mesh_culling_debug_output_buffer =
                asset_manager
                    .device_context()
                    .create_buffer(&RafxBufferDef {
                        size: std::mem::size_of::<mesh_culling_comp::DebugOutputBuffer>() as u64,
                        alignment: 256,
                        memory_usage: RafxMemoryUsage::GpuToCpu,
                        queue_type: RafxQueueType::Graphics,
                        resource_type: RafxResourceType::BUFFER_READ_WRITE,
                        elements: Default::default(),
                        format: RafxFormat::UNDEFINED,
                        always_mapped: false,
                    })?;
            mesh_culling_debug_output_buffer
                .set_debug_name(format!("Mesh Culling Debug Output {}", i));
            mesh_culling_debug_output.push(
                asset_manager
                    .resource_manager()
                    .resources()
                    .insert_buffer(mesh_culling_debug_output_buffer),
            );
        }

        let taa_history_rt = None;

        render_resources.insert(ModernPipelineStaticResources {
            bloom_extract_material,
            bloom_blur_material,
            bloom_combine_material,
            ssao_material,
            blue_noise_texture,
            taa_material,
            luma_build_histogram,
            luma_average_histogram,
            cas_pipeline,
            mesh_culling_pipeline,
            depth_pyramid_pipeline,
            tonemap_histogram_result,
            tonemap_debug_output,
            mesh_culling_debug_output,
            taa_history_rt,
        });

        Ok(())
    }

    fn generate_render_graph(
        &self,
        asset_manager: &AssetManager,
        swapchain_image: ResourceArc<ImageViewResource>,
        rotating_frame_index: usize,
        main_view: RenderView,
        extract_resources: &ExtractResources,
        render_resources: &RenderResources,
    ) -> RafxResult<PreparedRenderGraph> {
        super::graph_generator::generate_render_graph(
            asset_manager,
            swapchain_image,
            rotating_frame_index,
            main_view,
            extract_resources,
            render_resources,
        )
    }
}
