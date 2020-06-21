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
use renderer_assets::vk_description::SwapchainSurfaceInfo;
use super::PipelineCreateData;
use std::mem::ManuallyDrop;
use std::borrow::Borrow;
use renderer_assets::vk_description as dsc;
use atelier_assets::loader::LoadHandle;
use super::ResourceId;
use crate::resource_managers::ResourceArc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
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
pub struct DynResourceAllocatorInner<ResourceT>
where
    ResourceT: VkResource + Clone,
{
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    next_index: AtomicU64,
    active_count: Arc<AtomicU32>,
}

#[derive(Clone)]
pub struct DynResourceAllocator<ResourceT>
    where
        ResourceT: VkResource + Clone
{
    inner: Arc<DynResourceAllocatorInner<ResourceT>>
}

impl<ResourceT> DynResourceAllocator<ResourceT>
    where
        ResourceT: VkResource + Clone + std::fmt::Debug,
{
    fn new(
        drop_tx: Sender<ResourceWithHash<ResourceT>>,
        allocator_index: u32,
        active_count: Arc<AtomicU32>,
    ) -> Self {
        let next_index = (allocator_index as u64) << 32 + 1;

        let inner = DynResourceAllocatorInner {
            drop_tx,
            next_index: AtomicU64::new(next_index),
            active_count
        };

        DynResourceAllocator {
            inner: Arc::new(inner)
        }
    }

    fn insert(
        &self,
        resource: ResourceT,
    ) -> ResourceArc<ResourceT> {
        // This index is not strictly necessary. However, we do want to be compatible with ResourceArc,
        // and in other usecases a working index is necessary. Since we have the index anyways, we
        // might as well produce some sort of index if only to make logging easier to follow
        let resource_index = DynResourceIndex(self.inner.next_index.fetch_add(1, Ordering::Relaxed));
        self.inner.active_count.fetch_add(1, Ordering::Relaxed);

        log::trace!(
            "insert resource {} {:?}",
            core::any::type_name::<ResourceT>(),
            resource
        );

        ResourceArc::new(resource, resource_index.into(), self.inner.drop_tx.clone())
    }
}

#[derive(Clone)]
pub struct DynResourceAllocatorSet {
    pub images: DynResourceAllocator<VkImageRaw>,
    pub buffers: DynResourceAllocator<VkBufferRaw>,
}

impl DynResourceAllocatorSet {
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

pub struct DynResourceAllocatorManager<ResourceT>
    where
        ResourceT: VkResource + Clone,
{
    drop_sink: VkResourceDropSink<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    drop_rx: Receiver<ResourceWithHash<ResourceT>>,
    //allocator: DynResourceAllocator<ResourceT>
    next_allocator_index: AtomicU32,
    active_count: Arc<AtomicU32>,
}

impl<ResourceT> DynResourceAllocatorManager<ResourceT>
where
    ResourceT: VkResource + Clone + std::fmt::Debug,
{
    fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();
        let drop_sink = VkResourceDropSink::new(max_frames_in_flight);

        DynResourceAllocatorManager {
            drop_sink,
            drop_tx,
            drop_rx,
            next_allocator_index: AtomicU32::new(0),
            active_count: Arc::new(AtomicU32::new(0))
        }
    }

    fn create_allocator(&self) -> DynResourceAllocator<ResourceT> {
        let allocator_index = self.next_allocator_index.fetch_add(1, Ordering::Relaxed);
        let allocator = DynResourceAllocator::new(
            self.drop_tx.clone(),
            allocator_index,
            self.active_count.clone()
        );
        allocator
    }

    fn handle_dropped_resources(&mut self) {
        for dropped in self.drop_rx.try_iter() {
            log::trace!(
                "dropping {} {:?}",
                core::any::type_name::<ResourceT>(),
                dropped.resource
            );
            self.drop_sink.retire(dropped.resource);
            self.active_count.fetch_sub(1, Ordering::Relaxed);
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

        if self.len() > 0 {
            log::warn!(
                "{} resource count {} > 0, resources will leak",
                core::any::type_name::<ResourceT>(),
                self.len()
            );
        }

        self.drop_sink.destroy(device_context);
    }

    fn len(&self) -> usize {
        self.active_count.load(Ordering::Relaxed) as usize
    }
}

#[derive(Debug)]
pub struct ResourceMetrics {
    pub image_count: usize,
    pub buffer_count: usize,
}

//
// Handles raw lookup and destruction of GPU resources. Everything is reference counted. No safety
// is provided for dependencies/order of destruction. The general expectation is that anything
// dropped can safely be destroyed after a few frames have passed (based on max number of frames
// that can be submitted to the GPU)
//
pub struct DynResourceAllocatorManagerSet {
    pub device_context: VkDeviceContext,
    pub images: DynResourceAllocatorManager<VkImageRaw>,
    pub buffers: DynResourceAllocatorManager<VkBufferRaw>,
}

impl DynResourceAllocatorManagerSet {
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> Self {
        DynResourceAllocatorManagerSet {
            device_context: device_context.clone(),
            images: DynResourceAllocatorManager::new(max_frames_in_flight),
            buffers: DynResourceAllocatorManager::new(max_frames_in_flight),
        }
    }

    pub fn create_allocator_set(&self) -> DynResourceAllocatorSet {
        DynResourceAllocatorSet {
            images: self.images.create_allocator(),
            buffers: self.buffers.create_allocator()
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
