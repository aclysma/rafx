use crate::features::debug3d::{ExtractedDebug3dData, Debug3dRenderFeature, DebugDraw3DResource};
use crate::render_contexts::{RenderJobExtractContext, RenderJobWriteContext, RenderJobPrepareContext};
use renderer::nodes::{
    FramePacket, RenderView, PrepareJob, RenderFeatureIndex, RenderFeature, ExtractJob,
};
use crate::features::debug3d::prepare::Debug3dPrepareJobImpl;
use atelier_assets::loader::handle::Handle;
use renderer::assets::MaterialAsset;

pub struct Debug3dExtractJob {
    debug3d_material: Handle<MaterialAsset>,
}

impl Debug3dExtractJob {
    pub fn new(debug3d_material: &Handle<MaterialAsset>) -> Self {
        Debug3dExtractJob {
            debug3d_material: debug3d_material.clone(),
        }
    }
}

impl ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>
    for Debug3dExtractJob
{
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let line_lists = extract_context
            .resources
            .get_mut::<DebugDraw3DResource>()
            .unwrap()
            .take_line_lists();

        let debug3d_material_pass = extract_context
            .resource_manager
            .get_material_pass_by_index(&self.debug3d_material, 0)
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
