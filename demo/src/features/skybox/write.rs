use rafx::render_feature_write_job_prelude::*;

use rafx::api::RafxPrimitiveTopology;
use rafx::framework::VertexDataSetLayout;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

use rafx::framework::{DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct FeatureCommandWriterImpl {
    material_pass_resource: ResourceArc<MaterialPassResource>,
    submit_nodes: Vec<SubmitNodeData>,
}

impl FeatureCommandWriterImpl {
    pub fn new(material_pass_resource: ResourceArc<MaterialPassResource>) -> Self {
        FeatureCommandWriterImpl {
            material_pass_resource,
            submit_nodes: Default::default(),
        }
    }

    pub fn push_submit_node(
        &mut self,
        per_view_descriptor_set: DescriptorSetArc,
    ) -> SubmitNodeId {
        let idx = self.submit_nodes.len();
        self.submit_nodes.push(SubmitNodeData {
            per_view_descriptor_set,
        });
        return idx as SubmitNodeId;
    }
}

struct SubmitNodeData {
    per_view_descriptor_set: DescriptorSetArc,
}

impl FeatureCommandWriter for FeatureCommandWriterImpl {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        profiling::scope!(super::apply_setup_scope);

        let command_buffer = &write_context.command_buffer;

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                render_phase_index,
                &self.material_pass_resource,
                &write_context.render_target_meta,
                &EMPTY_VERTEX_LAYOUT,
            )?;

        command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;

        Ok(())
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_element_scope);

        let command_buffer = &write_context.command_buffer;

        let submit_node = &self.submit_nodes[index as usize];
        submit_node.per_view_descriptor_set.bind(command_buffer)?;

        command_buffer.cmd_draw(3, 0)?;

        Ok(())
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
