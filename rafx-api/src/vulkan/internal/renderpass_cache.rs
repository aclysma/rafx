use super::LruCache;
use super::*;
use crate::vulkan::{RafxDeviceContextVulkan, RafxRenderpassVulkan};
use crate::*;
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

pub(crate) struct RafxRenderpassVulkanCache {
    cache: LruCache<RafxRenderpassVulkan>,
}

impl RafxRenderpassVulkanCache {
    pub(crate) fn new(max_count: usize) -> Self {
        RafxRenderpassVulkanCache {
            cache: LruCache::new(max_count),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }

    pub(crate) fn renderpass_hash(
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<&RafxDepthRenderTargetBinding>,
    ) -> u64 {
        let mut hasher = FnvHasher::default();
        for color_target in color_targets {
            let rt_def = color_target.render_target.render_target_def();
            rt_def.format.hash(&mut hasher);
            rt_def.sample_count.hash(&mut hasher);
            color_target.clear_value.hash(&mut hasher);
            color_target.load_op.hash(&mut hasher);
        }

        if let Some(depth_target) = &depth_target {
            let rt_def = depth_target.render_target.render_target_def();
            rt_def.format.hash(&mut hasher);
            rt_def.sample_count.hash(&mut hasher);
            depth_target.clear_value.hash(&mut hasher);
            depth_target.stencil_load_op.hash(&mut hasher);
            depth_target.depth_load_op.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub(crate) fn create_renderpass(
        device_context: &RafxDeviceContextVulkan,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<&RafxDepthRenderTargetBinding>,
    ) -> RafxResult<RafxRenderpassVulkan> {
        let sample_count = if let Some(depth_target) = &depth_target {
            depth_target.render_target.render_target_def().sample_count
        } else {
            color_targets
                .first()
                .unwrap()
                .render_target
                .render_target_def()
                .sample_count
        };

        let color_attachments: Vec<_> = color_targets
            .iter()
            .map(|x| RafxRenderpassVulkanColorAttachment {
                format: x.render_target.render_target_def().format,
                load_op: x.load_op,
            })
            .collect();

        let resolve_attachments: Vec<_> = color_targets
            .iter()
            .map(|x| {
                x.resolve_target
                    .map(|x| RafxRenderpassVulkanResolveAttachment {
                        format: x.render_target_def().format,
                    })
            })
            .collect();

        let depth_attachment = depth_target
            .as_ref()
            .map(|x| RafxRenderpassVulkanDepthAttachment {
                format: x.render_target.render_target_def().format,
                depth_load_op: x.depth_load_op,
                stencil_load_op: x.stencil_load_op,
            });

        assert_eq!(color_attachments.len(), resolve_attachments.len());
        RafxRenderpassVulkan::new(
            device_context,
            &RafxRenderpassVulkanDef {
                color_attachments,
                resolve_attachments,
                depth_attachment,
                sample_count,
            },
        )
    }

    pub(crate) fn get_or_create_renderpass(
        &mut self,
        device_context: &RafxDeviceContextVulkan,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<&RafxDepthRenderTargetBinding>,
    ) -> RafxResult<RafxRenderpassVulkan> {
        //
        // Hash it
        //
        let hash = Self::renderpass_hash(color_targets, depth_target);

        self.cache.get_or_create(hash, || {
            Self::create_renderpass(device_context, color_targets, depth_target)
        })
    }
}
