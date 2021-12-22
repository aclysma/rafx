use crate::phases::{
    DebugPipRenderPhase, DepthPrepassRenderPhase, OpaqueRenderPhase, PostProcessRenderPhase,
    ShadowMapRenderPhase, TransparentRenderPhase, UiRenderPhase, WireframeRenderPhase,
};
use crate::shaders::post_adv::luma_average_histogram_comp;
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::{
    RafxBufferDef, RafxFormat, RafxMemoryUsage, RafxQueueType, RafxResourceType, RafxResult,
};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, ComputePipelineAsset, MaterialAsset};
use rafx::base::resource_map::ResourceMap;
use rafx::distill::loader::handle::Handle;
use rafx::framework::{BufferResource, ResourceArc, MAX_FRAMES_IN_FLIGHT};
use rafx::render_features::{ExtractResources, RenderRegistryBuilder};
use rafx::renderer::RendererAssetPlugin;
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

pub struct ModernPipelineStaticResources {
    pub bloom_extract_material: Handle<MaterialAsset>,
    pub bloom_blur_material: Handle<MaterialAsset>,
    pub bloom_combine_material: Handle<MaterialAsset>,
    pub luma_build_histogram: Handle<ComputePipelineAsset>,
    pub luma_average_histogram: Handle<ComputePipelineAsset>,
    pub tonemap_histogram_result: ResourceArc<BufferResource>,
    pub tonemap_debug_output: Vec<ResourceArc<BufferResource>>,
}

pub struct ModernPipelineRendererPlugin;

impl RendererAssetPlugin for ModernPipelineRendererPlugin {
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
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        //
        // Bloom extract resources
        //
        // let bloom_extract_material = asset_resource
        //     .load_asset_path::<MaterialAsset, _>("pipelines/bloom_extract.material");
        let bloom_extract_material = asset_resource
            .load_asset_path::<MaterialAsset, _>("rafx-plugins/materials/bloom_extract.material");

        //
        // Bloom blur resources
        //
        let bloom_blur_material = asset_resource
            .load_asset_path::<MaterialAsset, _>("rafx-plugins/materials/bloom_blur.material");

        //
        // Bloom combine resources
        //
        let bloom_combine_material = asset_resource.load_asset_path::<MaterialAsset, _>(
            "rafx-plugins/materials/modern_pipeline/bloom_combine_adv.material",
        );

        let luma_build_histogram = asset_resource.load_asset_path::<ComputePipelineAsset, _>(
            "rafx-plugins/compute_pipelines/luma_build_histogram.compute",
        );

        let luma_average_histogram = asset_resource.load_asset_path::<ComputePipelineAsset, _>(
            "rafx-plugins/compute_pipelines/luma_average_histogram.compute",
        );

        asset_manager.wait_for_asset_to_load(
            &bloom_extract_material,
            asset_resource,
            "bloom extract material",
        )?;

        asset_manager.wait_for_asset_to_load(
            &bloom_blur_material,
            asset_resource,
            "bloom blur material",
        )?;

        asset_manager.wait_for_asset_to_load(
            &bloom_combine_material,
            asset_resource,
            "bloom combine material",
        )?;

        asset_manager.wait_for_asset_to_load(
            &luma_build_histogram,
            asset_resource,
            "luma_build_histogram",
        )?;

        asset_manager.wait_for_asset_to_load(
            &luma_average_histogram,
            asset_resource,
            "luma_average_histogram",
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

        let tonemap_histogram_result = asset_manager
            .resource_manager()
            .resources()
            .insert_buffer(tonemap_histogram_result);

        let mut tonemap_debug_output = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT + 1);
        for _ in 0..=MAX_FRAMES_IN_FLIGHT {
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
            tonemap_debug_output.push(
                asset_manager
                    .resource_manager()
                    .resources()
                    .insert_buffer(tonemap_debug_output_buffer),
            );
        }

        render_resources.insert(ModernPipelineStaticResources {
            bloom_extract_material,
            bloom_blur_material,
            bloom_combine_material,
            luma_build_histogram,
            luma_average_histogram,
            tonemap_histogram_result,
            tonemap_debug_output,
        });

        Ok(())
    }
}
