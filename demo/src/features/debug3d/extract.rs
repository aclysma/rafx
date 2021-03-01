use crate::features::debug3d::prepare::Debug3dPrepareJobImpl;
use crate::features::debug3d::{Debug3dRenderFeature, DebugDraw3DResource, ExtractedDebug3dData};
use crate::game_renderer::GameRendererStaticResources;
use rafx::nodes::{ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex, RenderView, RenderJobExtractContext};
use crate::legion_support::LegionResources;
use rafx::assets::AssetManagerRenderResource;

pub struct Debug3dExtractJob {}

impl Debug3dExtractJob {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob
    for Debug3dExtractJob
{
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!("Debug3d Extract");
        let legion_resources = extract_context.render_resources.fetch::<LegionResources>();
        let asset_manager = extract_context.render_resources.fetch::<AssetManagerRenderResource>();

        let line_lists = legion_resources
            .get_mut::<DebugDraw3DResource>()
            .unwrap()
            .take_line_lists();

        let debug3d_material = &extract_context
            .render_resources
            .fetch::<GameRendererStaticResources>()
            .debug3d_material;
        let debug3d_material_pass = asset_manager
            .get_material_pass_by_index(&debug3d_material, 0)
            .unwrap();

        Box::new(Debug3dPrepareJobImpl::new(
            debug3d_material_pass,
            ExtractedDebug3dData { line_lists },
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        Debug3dRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        Debug3dRenderFeature::feature_index()
    }
}
