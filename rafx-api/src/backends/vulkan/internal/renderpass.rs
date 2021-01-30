use crate::vulkan::RafxDeviceContextVulkan;
use crate::{RafxFormat, RafxLoadOp, RafxResult, RafxSampleCount, RafxStoreOp};
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct RafxRenderpassVulkanColorAttachment {
    pub(crate) format: RafxFormat,
    pub(crate) load_op: RafxLoadOp,
    pub(crate) store_op: RafxStoreOp,
}

#[derive(Clone, Debug)]
pub(crate) struct RafxRenderpassVulkanResolveAttachment {
    pub(crate) format: RafxFormat,
}

#[derive(Clone, Debug)]
pub(crate) struct RafxRenderpassVulkanDepthAttachment {
    pub(crate) format: RafxFormat,
    pub(crate) depth_load_op: RafxLoadOp,
    pub(crate) stencil_load_op: RafxLoadOp,
    pub(crate) depth_store_op: RafxStoreOp,
    pub(crate) stencil_store_op: RafxStoreOp,
}

#[derive(Clone, Debug)]
pub(crate) struct RafxRenderpassVulkanDef {
    pub(crate) color_attachments: Vec<RafxRenderpassVulkanColorAttachment>,
    pub(crate) resolve_attachments: Vec<Option<RafxRenderpassVulkanResolveAttachment>>,
    pub(crate) depth_attachment: Option<RafxRenderpassVulkanDepthAttachment>,
    pub(crate) sample_count: RafxSampleCount,
}

pub(crate) struct RafxRenderpassVulkanInner {
    device_context: RafxDeviceContextVulkan,
    renderpass: vk::RenderPass,
}

impl Drop for RafxRenderpassVulkanInner {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_render_pass(self.renderpass, None);
        }
    }
}

#[derive(Clone)]
pub(crate) struct RafxRenderpassVulkan {
    inner: Arc<RafxRenderpassVulkanInner>,
}

impl RafxRenderpassVulkan {
    pub fn vk_renderpass(&self) -> vk::RenderPass {
        self.inner.renderpass
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        renderpass_def: &RafxRenderpassVulkanDef,
    ) -> RafxResult<Self> {
        //println!("Create renderpass\n{:#?}", renderpass_def);

        let samples = renderpass_def.sample_count.into();
        let mut attachments = Vec::with_capacity(renderpass_def.color_attachments.len() * 2 + 1);
        let mut color_attachment_refs = Vec::with_capacity(renderpass_def.color_attachments.len());
        let mut resolve_attachment_refs =
            Vec::with_capacity(renderpass_def.color_attachments.len());

        for (color_attachment_index, color_attachment) in
            renderpass_def.color_attachments.iter().enumerate()
        {
            attachments.push(
                vk::AttachmentDescription::builder()
                    .format(color_attachment.format.into())
                    .samples(samples)
                    .load_op(color_attachment.load_op.into())
                    .store_op(color_attachment.store_op.into())
                    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .build(),
            );

            color_attachment_refs.push(
                vk::AttachmentReference::builder()
                    .attachment(color_attachment_index as u32)
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .build(),
            );
        }

        for (resolve_attachment_index, resolve_attachment) in
            renderpass_def.resolve_attachments.iter().enumerate()
        {
            if let Some(resolve_attachment) = resolve_attachment {
                let attachment_index = attachments.len() as u32;

                attachments.push(
                    vk::AttachmentDescription::builder()
                        .format(resolve_attachment.format.into())
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .load_op(vk::AttachmentLoadOp::DONT_CARE)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                        .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .build(),
                );

                while resolve_attachment_refs.len() < resolve_attachment_index {
                    resolve_attachment_refs.push(
                        vk::AttachmentReference::builder()
                            .attachment(vk::ATTACHMENT_UNUSED)
                            .layout(vk::ImageLayout::UNDEFINED)
                            .build(),
                    )
                }

                resolve_attachment_refs.push(
                    vk::AttachmentReference::builder()
                        .attachment(attachment_index)
                        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .build(),
                );
            }
        }

        // The resolve attachment length must be empty or same length as color attachment refs
        if !resolve_attachment_refs.is_empty() {
            while resolve_attachment_refs.len() < color_attachment_refs.len() {
                resolve_attachment_refs.push(
                    vk::AttachmentReference::builder()
                        .attachment(vk::ATTACHMENT_UNUSED)
                        .layout(vk::ImageLayout::UNDEFINED)
                        .build(),
                )
            }
        }

        let mut depth_stencil_attachment_ref = None;
        if let Some(depth_attachment) = &renderpass_def.depth_attachment {
            assert_ne!(depth_attachment.format, RafxFormat::UNDEFINED);
            let attachment_index = attachments.len() as u32;
            attachments.push(
                vk::AttachmentDescription::builder()
                    .format(depth_attachment.format.into())
                    .samples(samples)
                    .load_op(depth_attachment.depth_load_op.into())
                    .store_op(depth_attachment.depth_store_op.into())
                    .stencil_load_op(depth_attachment.stencil_load_op.into())
                    .stencil_store_op(depth_attachment.stencil_store_op.into())
                    .initial_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .build(),
            );

            depth_stencil_attachment_ref = Some(
                vk::AttachmentReference::builder()
                    .attachment(attachment_index)
                    .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .build(),
            );
        }

        let mut subpass_description = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs);

        if !resolve_attachment_refs.is_empty() {
            subpass_description = subpass_description.resolve_attachments(&resolve_attachment_refs);
        }

        if let Some(depth_stencil_attachment_ref) = depth_stencil_attachment_ref.as_ref() {
            subpass_description =
                subpass_description.depth_stencil_attachment(depth_stencil_attachment_ref);
        }

        let subpass_descriptions = [subpass_description.build()];

        let renderpass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpass_descriptions);

        let renderpass = unsafe {
            device_context
                .device()
                .create_render_pass(&*renderpass_create_info, None)?
        };

        let inner = RafxRenderpassVulkanInner {
            device_context: device_context.clone(),
            renderpass,
        };

        Ok(RafxRenderpassVulkan {
            inner: Arc::new(inner),
        })
    }
}
