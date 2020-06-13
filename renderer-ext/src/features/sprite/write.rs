use crate::features::sprite::{SpriteRenderFeature, SpriteDrawCall};
use renderer_base::{RenderFeatureIndex, RenderFeature, SubmitNodeId, FeatureCommandWriter};
use crate::CommandWriterContext;
use renderer_shell_vulkan::VkBuffer;
use std::mem::ManuallyDrop;

pub struct SpriteCommandWriter {
    pub vertex_buffers: Vec<ManuallyDrop<VkBuffer>>,
    pub index_buffers: Vec<ManuallyDrop<VkBuffer>>,
    pub draw_calls: Vec<SpriteDrawCall>,
}

impl FeatureCommandWriter<CommandWriterContext> for SpriteCommandWriter {
    fn apply_setup(
        &self,
        _write_context: &mut CommandWriterContext,
    ) {
        //println!("apply");
    }

    fn render_element(
        &self,
        _write_context: &mut CommandWriterContext,
        index: SubmitNodeId,
    ) {
        //println!("render");
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

impl Drop for SpriteCommandWriter {
    fn drop(&mut self) {
        for buffer in &mut self.vertex_buffers {
            unsafe {
                ManuallyDrop::drop(buffer);
            }
        }

        for buffer in &mut self.index_buffers {
            unsafe {
                ManuallyDrop::drop(buffer);
            }
        }
    }
}