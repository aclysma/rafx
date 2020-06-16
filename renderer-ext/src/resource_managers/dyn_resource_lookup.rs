use crossbeam_channel::{Sender, Receiver};
use std::fmt::Formatter;
use std::hash::Hash;
use std::sync::{Weak, Arc, Mutex};
use renderer_shell_vulkan::{
    VkResource, VkResourceDropSink, VkDeviceContext, VkImageRaw, VkImage, VkBufferRaw, VkBuffer,
};
use fnv::FnvHashMap;
use std::marker::PhantomData;
use ash::vk;
use ash::prelude::VkResult;
use crate::pipeline_description::SwapchainSurfaceInfo;
use super::PipelineCreateData;
use std::mem::ManuallyDrop;
use std::borrow::Borrow;
use crate::pipeline_description as dsc;
use atelier_assets::loader::LoadHandle;
use dashmap::DashMap;
use super::ResourceId;
use crate::resource_managers::ResourceArc;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::resource_managers::resource_arc::{WeakResourceArc, ResourceWithHash};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DynResourceIndex(u64);

impl From<ResourceId> for DynResourceIndex {
    fn from(resource_id: ResourceId) -> Self {
        DynResourceIndex(resource_id.0)
    }
}

impl Into<ResourceId> for DynResourceIndex {
    fn into(self) -> ResourceId {
        ResourceId(self.0)
    }
}

//
// A lookup of dynamic resources. They reference count using Arcs internally and send a signal when they
// drop. This allows the resources to be collected and disposed of. This is threadsafe, as opposed to
// ResourceLookup. It's intended for things that get created/thrown away
//
pub struct DynResourceLookupInner<ResourceT>
where
    ResourceT: VkResource + Clone,
{
    resources: DashMap<DynResourceIndex, WeakResourceArc<ResourceT>>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    next_index: AtomicU64,
}

#[derive(Clone)]
pub struct DynResourceLookup<ResourceT>
    where
        ResourceT: VkResource + Clone
{
    inner: Arc<DynResourceLookupInner<ResourceT>>
}


impl<ResourceT> DynResourceLookup<ResourceT>
    where
        ResourceT: VkResource + Clone + std::fmt::Debug,
{
    fn new(
        drop_tx: Sender<ResourceWithHash<ResourceT>>
    ) -> Self {
        let inner = DynResourceLookupInner {
            resources: Default::default(),
            drop_tx,
            next_index: AtomicU64::new(1)
        };

        DynResourceLookup {
            inner: Arc::new(inner)
        }
    }

    fn get(
        &self,
        hash: ResourceId,
    ) -> Option<ResourceArc<ResourceT>> {
        if let Some(resource) = self.inner.resources.get(&hash.into()) {
            let upgrade = resource.upgrade();
            upgrade
        } else {
            None
        }
    }

    fn insert(
        &self,
        resource: ResourceT,
    ) -> ResourceArc<ResourceT> {
        let resource_index = DynResourceIndex(self.inner.next_index.fetch_add(1, Ordering::Relaxed));

        log::trace!(
            "insert resource {} {:?}",
            core::any::type_name::<ResourceT>(),
            resource
        );

        let arc = ResourceArc::new(resource, resource_index.into(), self.inner.drop_tx.clone());
        let downgraded = arc.downgrade();
        let old = self.inner.resources.insert(resource_index, downgraded);
        assert!(old.is_none());

        arc
    }

    fn len(&self) -> usize {
        self.inner.resources.len()
    }
}

pub struct DynResourceLookupManager<ResourceT>
    where
        ResourceT: VkResource + Clone,
{
    drop_sink: VkResourceDropSink<ResourceT>,
    drop_rx: Receiver<ResourceWithHash<ResourceT>>,
    allocator: DynResourceLookup<ResourceT>
}

impl<ResourceT> DynResourceLookupManager<ResourceT>
where
    ResourceT: VkResource + Clone + std::fmt::Debug,
{
    fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();
        let drop_sink = VkResourceDropSink::new(max_frames_in_flight);

        DynResourceLookupManager {
            drop_sink,
            drop_rx,
            allocator: DynResourceLookup::new(drop_tx)
        }
    }

    fn handle_dropped_resources(&mut self) {
        for dropped in self.drop_rx.try_iter() {
            log::trace!(
                "dropping {} {:?}",
                core::any::type_name::<ResourceT>(),
                dropped.resource
            );
            self.drop_sink.retire(dropped.resource);
            self.allocator.inner.resources.remove(&dropped.resource_hash.into());
        }
    }

    fn on_frame_complete(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        self.handle_dropped_resources();
        self.drop_sink.on_frame_complete(device_context);
    }

    fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        self.handle_dropped_resources();

        if self.allocator.inner.resources.len() > 0 {
            log::warn!(
                "{} resource count {} > 0, resources will leak",
                core::any::type_name::<ResourceT>(),
                self.allocator.inner.resources.len()
            );
        }

        self.drop_sink.destroy(device_context);
    }

    fn len(&self) -> usize {
        self.allocator.len()
    }
}

#[derive(Debug)]
pub struct ResourceMetrics {
    pub image_count: usize,
    pub buffer_count: usize,
}

#[derive(Clone)]
pub struct DynResourceLookupSet {
    pub images: DynResourceLookup<VkImageRaw>,
    pub buffers: DynResourceLookup<VkBufferRaw>,
}

impl DynResourceLookupSet {
    pub fn insert_image(
        &self,
        image: VkImage,
    ) -> ResourceArc<VkImageRaw> {
        let raw_image = image.take_raw().unwrap();
        let image = self.images.insert(raw_image);
        image
    }

    pub fn insert_buffer(
        &self,
        buffer: VkBuffer,
    ) -> ResourceArc<VkBufferRaw> {
        let raw_buffer = buffer.take_raw().unwrap();
        let buffer = self.buffers.insert(raw_buffer);
        buffer
    }
}

//
// Handles raw lookup and destruction of GPU resources. Everything is reference counted. No safety
// is provided for dependencies/order of destruction. The general expectation is that anything
// dropped can safely be destroyed after a few frames have passed (based on max number of frames
// that can be submitted to the GPU)
//
pub struct DynResourceLookupManagerSet {
    pub device_context: VkDeviceContext,
    pub images: DynResourceLookupManager<VkImageRaw>,
    pub buffers: DynResourceLookupManager<VkBufferRaw>,
}

impl DynResourceLookupManagerSet {
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> Self {
        DynResourceLookupManagerSet {
            device_context: device_context.clone(),
            images: DynResourceLookupManager::new(max_frames_in_flight),
            buffers: DynResourceLookupManager::new(max_frames_in_flight),
        }
    }

    pub fn create_allocator_set(&self) -> DynResourceLookupSet {
        DynResourceLookupSet {
            images: self.images.allocator.clone(),
            buffers: self.buffers.allocator.clone()
        }
    }

    pub fn on_frame_complete(&mut self) {
        self.buffers.on_frame_complete(&self.device_context);
        self.images.on_frame_complete(&self.device_context);
    }

    pub fn destroy(&mut self) {
        //WARNING: These need to be in order of dependencies to avoid frame-delays on destroying
        // resources.
        self.images.destroy(&self.device_context);
        self.buffers.destroy(&self.device_context);
    }

    pub fn metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            image_count: self.images.len(),
            buffer_count: self.buffers.len(),
        }
    }
}
