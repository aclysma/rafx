use std::hash::Hash;
use fnv::FnvHashMap;
use crate::{RenderPassResource, ResourceArc, FramebufferResource};

pub struct ResourceCache<T: Eq + Hash> {
    resources: FnvHashMap<T, u64>,
    current_frame_index: u64,
    frames_to_persist: u64,
}

impl<T: Eq + Hash> ResourceCache<T> {
    pub fn new(frames_to_persist: u64) -> Self {
        ResourceCache {
            resources: Default::default(),
            current_frame_index: 0,
            frames_to_persist,
        }
    }

    pub fn touch_resource(
        &mut self,
        resource: T,
    ) {
        self.resources
            .entry(resource)
            .or_insert(self.current_frame_index + self.frames_to_persist);
    }

    pub fn on_frame_complete(&mut self) {
        let current_frame_index = self.current_frame_index;
        self.resources
            .retain(|_, keep_until_frame| *keep_until_frame > current_frame_index);
        self.current_frame_index += 1;
    }

    pub fn clear(&mut self) {
        self.resources.clear();
    }
}

pub struct ResourceCacheSet {
    render_passes: ResourceCache<ResourceArc<RenderPassResource>>,
    framebuffers: ResourceCache<ResourceArc<FramebufferResource>>,
}

impl ResourceCacheSet {
    pub fn cache_render_pass(
        &mut self,
        resource: ResourceArc<RenderPassResource>,
    ) {
        self.render_passes.touch_resource(resource);
    }

    pub fn cache_framebuffer(
        &mut self,
        resource: ResourceArc<FramebufferResource>,
    ) {
        self.framebuffers.touch_resource(resource);
    }

    pub fn on_frame_complete(&mut self) {
        self.render_passes.on_frame_complete();
        self.framebuffers.on_frame_complete();
    }

    pub fn clear(&mut self) {
        self.render_passes.clear();
        self.framebuffers.clear();
    }
}

impl Default for ResourceCacheSet {
    fn default() -> Self {
        const DEFAULT_FRAMES_TO_PERSIST: u64 = 3;

        ResourceCacheSet {
            render_passes: ResourceCache::new(DEFAULT_FRAMES_TO_PERSIST),
            framebuffers: ResourceCache::new(DEFAULT_FRAMES_TO_PERSIST),
        }
    }
}
