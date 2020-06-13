use crate::features::sprite::SpriteRenderFeature;
use renderer_base::{RenderFeatureIndex, RenderFeature, SubmitNodeId, FeatureCommandWriter};
use crate::CommandWriterContext;

pub struct SpriteCommandWriter {}

impl FeatureCommandWriter<CommandWriterContext> for SpriteCommandWriter {
    fn apply_setup(
        &self,
        _write_context: &mut CommandWriterContext,
    ) {

    }

    fn render_element(
        &self,
        _write_context: &mut CommandWriterContext,
        index: SubmitNodeId,
    ) {

    }

    fn revert_setup(
        &self,
        _write_context: &mut CommandWriterContext,
    ) {

    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}