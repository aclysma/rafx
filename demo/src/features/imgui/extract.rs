use crate::features::imgui::{ExtractedImGuiData, ImGuiRenderFeature, ImGuiUniformBufferObject};
use crate::render_contexts::{RenderJobExtractContext, RenderJobWriteContext, RenderJobPrepareContext};
use renderer::nodes::{
    DefaultExtractJobImpl, FramePacket, RenderView, PrepareJob, RenderFeatureIndex, RenderFeature,
    ExtractJob,
};
use crate::features::imgui::prepare::ImGuiPrepareJobImpl;
use renderer::vulkan::VkDeviceContext;
use renderer::resources::resource_managers::{PipelineSwapchainInfo, DescriptorSetAllocatorRef};
use renderer::assets::assets::pipeline::MaterialAsset;
use atelier_assets::loader::handle::Handle;
use renderer::assets::assets::image::ImageAsset;
use crate::imgui_support::Sdl2ImguiManager;
use ash::vk::Extent2D;
use renderer::resources::{ImageViewResource, ResourceArc};

// This is almost copy-pasted from glam. I wanted to avoid pulling in the entire library for a
// single function
pub fn orthographic_rh_gl(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    let a = 2.0 / (right - left);
    let b = 2.0 / (top - bottom);
    let c = -2.0 / (far - near);
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -(far + near) / (far - near);

    [
        [a, 0.0, 0.0, 0.0],
        [0.0, b, 0.0, 0.0],
        [0.0, 0.0, c, 0.0],
        [tx, ty, tz, 1.0],
    ]
}

pub struct ImGuiExtractJobImpl {
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    extents: Extent2D,
    imgui_material: Handle<MaterialAsset>,
    font_atlas: ResourceArc<ImageViewResource>,
}

impl ImGuiExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        descriptor_set_allocator: DescriptorSetAllocatorRef,
        pipeline_info: PipelineSwapchainInfo,
        extents: Extent2D,
        imgui_material: &Handle<MaterialAsset>,
        font_atlas: ResourceArc<ImageViewResource>,
    ) -> Self {
        ImGuiExtractJobImpl {
            device_context,
            descriptor_set_allocator,
            pipeline_info,
            extents,
            imgui_material: imgui_material.clone(),
            font_atlas,
        }
    }
}

impl ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>
    for ImGuiExtractJobImpl
{
    fn extract(
        mut self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let imgui_draw_data = extract_context
            .resources
            .get::<Sdl2ImguiManager>()
            .unwrap()
            .copy_draw_data();

        let framebuffer_scale = match &imgui_draw_data {
            Some(data) => data.framebuffer_scale,
            None => [1.0, 1.0],
        };

        let view_proj = orthographic_rh_gl(
            0.0,
            self.extents.width as f32 / framebuffer_scale[0],
            0.0,
            self.extents.height as f32 / framebuffer_scale[1],
            -100.0,
            100.0,
        );

        let ubo = ImGuiUniformBufferObject { view_proj };

        let dyn_resource_allocator = extract_context
            .resource_manager
            .create_dyn_resource_allocator_set();
        let per_pass_layout =
            extract_context
                .resource_manager
                .get_descriptor_set_info(&self.imgui_material, 0, 0);

        let mut per_pass_descriptor_set = self
            .descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(&per_pass_layout.descriptor_set_layout)
            .unwrap();
        per_pass_descriptor_set.set_buffer_data(0, &ubo);
        per_pass_descriptor_set
            .flush(&mut self.descriptor_set_allocator)
            .unwrap();

        let per_image_layout =
            extract_context
                .resource_manager
                .get_descriptor_set_info(&self.imgui_material, 0, 1);
        let mut per_image_descriptor_set = self
            .descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(&per_image_layout.descriptor_set_layout)
            .unwrap();
        per_image_descriptor_set.set_image(0, self.font_atlas);
        per_image_descriptor_set
            .flush(&mut self.descriptor_set_allocator)
            .unwrap();

        let per_pass_descriptor_set = per_pass_descriptor_set.descriptor_set().clone();
        let per_image_descriptor_sets = vec![per_image_descriptor_set.descriptor_set().clone()];

        Box::new(ImGuiPrepareJobImpl::new(
            self.device_context,
            self.pipeline_info,
            dyn_resource_allocator,
            per_pass_descriptor_set,
            per_image_descriptor_sets,
            ExtractedImGuiData { imgui_draw_data },
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        ImGuiRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        ImGuiRenderFeature::feature_index()
    }
}
