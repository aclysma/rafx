use crate::features::debug3d::{
    ExtractedDebug3dData, Debug3dRenderFeature, DebugDraw3DResource, Debug3dUniformBufferObject,
};
use crate::render_contexts::{RenderJobExtractContext, RenderJobWriteContext, RenderJobPrepareContext};
use renderer::nodes::{
    FramePacket, RenderView, PrepareJob, RenderFeatureIndex, RenderFeature, ExtractJob,
};
use crate::features::debug3d::prepare::Debug3dPrepareJobImpl;
use renderer::vulkan::VkDeviceContext;
use renderer::resources::resource_managers::{PipelineSwapchainInfo, DescriptorSetAllocatorRef};
use renderer::assets::assets::pipeline::MaterialAssetData;
use atelier_assets::loader::handle::Handle;

pub struct Debug3dExtractJobImpl {
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    debug3d_material: Handle<MaterialAssetData>,
}

impl Debug3dExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        descriptor_set_allocator: DescriptorSetAllocatorRef,
        pipeline_info: PipelineSwapchainInfo,
        debug3d_material: &Handle<MaterialAssetData>,
    ) -> Self {
        Debug3dExtractJobImpl {
            device_context,
            descriptor_set_allocator,
            pipeline_info,
            debug3d_material: debug3d_material.clone(),
        }
    }
}

impl ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>
    for Debug3dExtractJobImpl
{
    fn extract(
        mut self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let dyn_resource_allocator = extract_context
            .resource_manager
            .create_dyn_resource_allocator_set();
        let layout =
            extract_context
                .resource_manager
                .get_descriptor_set_info(&self.debug3d_material, 0, 0);

        let per_view_descriptor_sets: Vec<_> = views
            .iter()
            .map(|view| {
                let debug3d_view = Debug3dUniformBufferObject {
                    view_proj: (view.projection_matrix() * view.view_matrix()).to_cols_array_2d(),
                };

                let mut descriptor_set = self
                    .descriptor_set_allocator
                    .create_dyn_descriptor_set_uninitialized(&layout.descriptor_set_layout)
                    .unwrap();
                descriptor_set.set_buffer_data(0, &debug3d_view);
                descriptor_set
                    .flush(&mut self.descriptor_set_allocator)
                    .unwrap();
                descriptor_set.descriptor_set().clone()
            })
            .collect();

        let line_lists = extract_context
            .resources
            .get_mut::<DebugDraw3DResource>()
            .unwrap()
            .take_line_lists();

        Box::new(Debug3dPrepareJobImpl::new(
            self.device_context,
            self.pipeline_info,
            dyn_resource_allocator,
            per_view_descriptor_sets,
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
