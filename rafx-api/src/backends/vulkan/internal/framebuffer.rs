use crate::vulkan::{RafxDeviceContextVulkan, RafxRenderTargetVulkan, RafxRenderpassVulkan};
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::Arc;
use crate::*;

pub(crate) struct RafxFramebufferVulkanAttachment {
    pub(crate) render_target: RafxRenderTargetVulkan,
    pub(crate) array_slice: Option<u16>,
    pub(crate) mip_slice: Option<u8>,
}

pub(crate) struct RafxFramebufferVulkanDef {
    pub(crate) renderpass: RafxRenderpassVulkan,
    pub(crate) color_attachments: Vec<RafxFramebufferVulkanAttachment>,
    pub(crate) resolve_attachments: Vec<RafxFramebufferVulkanAttachment>,
    pub(crate) depth_stencil_attachment: Option<RafxFramebufferVulkanAttachment>,
}

pub(crate) struct RafxFramebufferVulkanInner {
    device_context: RafxDeviceContextVulkan,
    framebuffer: vk::Framebuffer,
    width: u32,
    height: u32,
}

impl Drop for RafxFramebufferVulkanInner {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_framebuffer(self.framebuffer, None);
        }
    }
}

#[derive(Clone)]
pub(crate) struct RafxFramebufferVulkan {
    inner: Arc<RafxFramebufferVulkanInner>,
}

impl RafxFramebufferVulkan {
    pub fn width(&self) -> u32 {
        self.inner.width
    }

    pub fn height(&self) -> u32 {
        self.inner.height
    }

    pub fn vk_framebuffer(&self) -> vk::Framebuffer {
        self.inner.framebuffer
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        framebuffer_def: &RafxFramebufferVulkanDef,
    ) -> RafxResult<Self> {
        let (extents, array_length) =
            if let Some(first_color_rt) = framebuffer_def.color_attachments.first() {
                let rt_def = first_color_rt.render_target.render_target_def();
                let extents = rt_def.extents.clone();

                let array_length = if extents.depth > 1 {
                    extents.depth
                } else if first_color_rt.array_slice.is_some() {
                    1u32
                } else {
                    rt_def.array_length
                };

                (extents, array_length)
            } else if let Some(depth_rt) = &framebuffer_def.depth_stencil_attachment {
                let rt_def = depth_rt.render_target.render_target_def();
                let extents = rt_def.extents.clone();

                let array_length = if depth_rt.array_slice.is_some() {
                    1u32
                } else {
                    rt_def.array_length
                };

                (extents, array_length)
            } else {
                return Err(RafxError::StringError(
                    "No render target in framebuffer def".to_string(),
                ));
            };

        let mut image_views = Vec::with_capacity(framebuffer_def.color_attachments.len() + 1);

        for color_rt in &framebuffer_def.color_attachments {
            let image_view = if color_rt.array_slice.is_none() && color_rt.mip_slice.is_none() {
                color_rt.render_target.render_target_vk_view()
            } else {
                color_rt.render_target.render_target_slice_vk_view(
                    0,
                    color_rt.array_slice.unwrap_or(0),
                    color_rt.mip_slice.unwrap_or(0),
                )
            };
            image_views.push(image_view);
        }

        for resolve_rt in &framebuffer_def.resolve_attachments {
            let image_view = if resolve_rt.array_slice.is_none() && resolve_rt.mip_slice.is_none() {
                resolve_rt.render_target.render_target_vk_view()
            } else {
                resolve_rt.render_target.render_target_slice_vk_view(
                    0,
                    resolve_rt.array_slice.unwrap_or(0),
                    resolve_rt.mip_slice.unwrap_or(0),
                )
            };
            image_views.push(image_view);
        }

        if let Some(depth_rt) = &framebuffer_def.depth_stencil_attachment {
            let image_view = if depth_rt.mip_slice.is_none() && depth_rt.array_slice.is_none() {
                depth_rt.render_target.render_target_vk_view()
            } else {
                depth_rt.render_target.render_target_slice_vk_view(
                    0,
                    depth_rt.array_slice.unwrap_or(0),
                    depth_rt.mip_slice.unwrap_or(0),
                )
            };
            image_views.push(image_view);
        };

        let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(framebuffer_def.renderpass.vk_renderpass())
            .attachments(&image_views)
            .width(extents.width)
            .height(extents.height)
            .layers(array_length);

        let framebuffer = unsafe {
            device_context
                .device()
                .create_framebuffer(&*framebuffer_create_info, None)?
        };

        let inner = RafxFramebufferVulkanInner {
            device_context: device_context.clone(),
            width: extents.width,
            height: extents.height,
            framebuffer,
        };

        Ok(RafxFramebufferVulkan {
            inner: Arc::new(inner),
        })
    }
}
