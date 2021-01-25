use super::*;
use crate::vulkan::{RafxDeviceContextVulkan, RafxRenderpassVulkan};
use crate::*;
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

pub(crate) struct RafxFramebufferVulkanCache {
    cache: LruCache<RafxFramebufferVulkan>,
}

impl RafxFramebufferVulkanCache {
    pub(crate) fn new(max_count: usize) -> Self {
        RafxFramebufferVulkanCache {
            cache: LruCache::new(max_count),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }

    pub(crate) fn framebuffer_hash(
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<&RafxDepthRenderTargetBinding>,
    ) -> u64 {
        let mut hasher = FnvHasher::default();
        for color_target in color_targets {
            color_target
                .render_target
                .vk_render_target()
                .unwrap()
                .render_target_id()
                .hash(&mut hasher);
            color_target.mip_slice.hash(&mut hasher);
            color_target.array_slice.hash(&mut hasher);

            if let Some(resolve_target) = color_target.resolve_target {
                resolve_target
                    .vk_render_target()
                    .unwrap()
                    .render_target_id()
                    .hash(&mut hasher);
                color_target.resolve_mip_slice.hash(&mut hasher);
                color_target.resolve_array_slice.hash(&mut hasher);
            }
        }

        if let Some(depth_target) = &depth_target {
            depth_target
                .render_target
                .vk_render_target()
                .unwrap()
                .render_target_id()
                .hash(&mut hasher);
            depth_target.mip_slice.hash(&mut hasher);
            depth_target.array_slice.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub(crate) fn create_framebuffer(
        device_context: &RafxDeviceContextVulkan,
        renderpass: &RafxRenderpassVulkan,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<&RafxDepthRenderTargetBinding>,
    ) -> RafxResult<RafxFramebufferVulkan> {
        let mut color_attachments = Vec::with_capacity(color_targets.len());
        let mut resolve_attachments = Vec::with_capacity(color_targets.len());

        for color_target in color_targets {
            color_attachments.push(RafxFramebufferVulkanAttachment {
                render_target: color_target
                    .render_target
                    .vk_render_target()
                    .unwrap()
                    .clone(),
                array_slice: color_target.array_slice,
                mip_slice: color_target.mip_slice,
            });

            if let Some(resolve_target) = color_target.resolve_target {
                resolve_attachments.push(RafxFramebufferVulkanAttachment {
                    render_target: resolve_target.vk_render_target().unwrap().clone(),
                    array_slice: color_target.resolve_array_slice,
                    mip_slice: color_target.resolve_mip_slice,
                })
            }
        }

        RafxFramebufferVulkan::new(
            device_context,
            &RafxFramebufferVulkanDef {
                renderpass: renderpass.clone(),
                color_attachments,
                resolve_attachments,
                depth_stencil_attachment: depth_target.as_ref().map(|x| {
                    RafxFramebufferVulkanAttachment {
                        render_target: x.render_target.vk_render_target().unwrap().clone(),
                        array_slice: x.array_slice,
                        mip_slice: x.mip_slice,
                    }
                }),
            },
        )
    }

    pub(crate) fn get_or_create_framebuffer(
        &mut self,
        device_context: &RafxDeviceContextVulkan,
        renderpass: &RafxRenderpassVulkan,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<&RafxDepthRenderTargetBinding>,
    ) -> RafxResult<RafxFramebufferVulkan> {
        //
        // Hash it
        //
        let hash = Self::framebuffer_hash(color_targets, depth_target);

        self.cache.get_or_create(hash, || {
            Self::create_framebuffer(device_context, renderpass, color_targets, depth_target)
        })
    }
}
