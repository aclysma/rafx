use rafx::render_feature_extract_job_predule::*;

use super::prepare::PrepareJobImpl;
use super::{DebugDraw3DResource, StaticResources};
use rafx::assets::AssetManagerRenderResource;

pub struct ExtractJobImpl {}

impl ExtractJobImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob for ExtractJobImpl {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!(super::extract_scope);

        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        let line_lists = extract_context
            .extract_resources
            .fetch_mut::<DebugDraw3DResource>()
            .take_line_lists();

        let debug3d_material = &extract_context
            .render_resources
            .fetch::<StaticResources>()
            .debug3d_material;

        let debug3d_material_pass = asset_manager
            .committed_asset(&debug3d_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        Box::new(PrepareJobImpl::new(debug3d_material_pass, line_lists))
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
