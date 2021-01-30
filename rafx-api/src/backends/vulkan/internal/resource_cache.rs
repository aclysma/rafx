use crate::vulkan::{RafxFramebufferVulkanCache, RafxRenderpassVulkanCache};
use std::sync::Mutex;

pub(crate) struct RafxDeviceVulkanResourceCacheInner {
    pub(crate) renderpass_cache: RafxRenderpassVulkanCache,
    pub(crate) framebuffer_cache: RafxFramebufferVulkanCache,
}

pub(crate) struct RafxDeviceVulkanResourceCache {
    pub(crate) inner: Mutex<RafxDeviceVulkanResourceCacheInner>,
}

impl RafxDeviceVulkanResourceCache {
    pub(crate) fn clear_caches(&self) {
        let mut lock = self.inner.lock().unwrap();
        lock.framebuffer_cache.clear();
        lock.renderpass_cache.clear();
    }
}

impl Default for RafxDeviceVulkanResourceCache {
    fn default() -> Self {
        let inner = RafxDeviceVulkanResourceCacheInner {
            renderpass_cache: RafxRenderpassVulkanCache::new(200),
            framebuffer_cache: RafxFramebufferVulkanCache::new(200),
        };

        RafxDeviceVulkanResourceCache {
            inner: Mutex::new(inner),
        }
    }
}
