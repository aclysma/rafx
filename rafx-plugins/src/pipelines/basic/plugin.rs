use crate::phases::{
    DebugPipRenderPhase, DepthPrepassRenderPhase, OpaqueRenderPhase, PostProcessRenderPhase,
    ShadowMapRenderPhase, TransparentRenderPhase, UiRenderPhase, WireframeRenderPhase,
};
use hydrate_base::handle::Handle;
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::RafxResult;
use rafx::assets::AssetResource;
use rafx::assets::{AssetManager, MaterialAsset};
use rafx::framework::{ImageViewResource, RenderResources, ResourceArc};
use rafx::graph::PreparedRenderGraph;
use rafx::render_features::{ExtractResources, RenderRegistryBuilder, RenderView};
use rafx::renderer::{RendererLoadContext, RendererPipelinePlugin};

// A plugin that add demo-specific configuration

pub struct BasicPipelineStaticResources {
    pub bloom_extract_material: Handle<MaterialAsset>,
    pub bloom_blur_material: Handle<MaterialAsset>,
    pub bloom_combine_material: Handle<MaterialAsset>,
}

pub struct BasicPipelineRendererPlugin;

impl RendererPipelinePlugin for BasicPipelineRendererPlugin {
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
        //     .load_asset_path::<MaterialAsset, _>("pipelines/bloom_extract.material");
        let bloom_extract_material = asset_resource.load_asset_path::<MaterialAsset, _>(
            "db:/path_file_system/rafx-plugins/materials/bloom_extract.material",
        );

        //
        // Bloom blur resources
        //
        let bloom_blur_material = asset_resource.load_asset_path::<MaterialAsset, _>(
            "db:/path_file_system/rafx-plugins/materials/bloom_blur.material",
        );

        //
        // Bloom combine resources
        //
        let bloom_combine_material = asset_resource.load_asset_path::<MaterialAsset, _>(
            "db:/path_file_system/rafx-plugins/materials/basic_pipeline/bloom_combine_basic.material",
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
            &bloom_combine_material,
            asset_resource,
            "bloom combine material",
        )?;

        render_resources.insert(BasicPipelineStaticResources {
            bloom_extract_material,
            bloom_blur_material,
            bloom_combine_material,
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
