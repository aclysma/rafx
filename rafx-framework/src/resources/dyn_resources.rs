use super::ResourceId;
use crate::resources::resource_arc::ResourceWithHash;
use crate::resources::resource_lookup::ImageResource;
use crate::resources::ResourceArc;
use crate::ResourceDropSink;
use crate::{BufferResource, ImageViewResource};
use crossbeam_channel::{Receiver, Sender};
use rafx_api::extra::image::RafxImage;
use rafx_api::{RafxBuffer, RafxDeviceContext, RafxResult, RafxTexture, RafxTextureBindType};
use std::hash::Hash;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

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
// drop. This allows the resources to be collected and disposed of. This is threadsafe and the only
// sync point is when dropping to send via a channel. (Although VMA memory allocator is probably
// locking too) As opposed to ResourceLookup which uses mutexes to maintain a lookup map. It's
// intended for things that get created/thrown away frequently although there is no problem with
// keeping a resource created through this utility around for a long time.
//
pub struct DynResourceAllocatorInner<ResourceT>
where
    ResourceT: Clone,
{
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    next_index: AtomicU64,
    active_count: Arc<AtomicU32>,
}

pub struct DynResourceAllocator<ResourceT>
where
    ResourceT: Clone,
{
    inner: Arc<DynResourceAllocatorInner<ResourceT>>,
}

impl<ResourceT> DynResourceAllocator<ResourceT>
where
    ResourceT: Clone + std::fmt::Debug,
{
    fn new(
        drop_tx: Sender<ResourceWithHash<ResourceT>>,
        allocator_index: u32,
        active_count: Arc<AtomicU32>,
    ) -> Self {
        let next_index = ((allocator_index as u64) << 32) + 1;

        let inner = DynResourceAllocatorInner {
            drop_tx,
            next_index: AtomicU64::new(next_index),
            active_count,
        };

        DynResourceAllocator {
            inner: Arc::new(inner),
        }
    }

    fn insert(
        &self,
        resource: ResourceT,
    ) -> ResourceArc<ResourceT> {
        // This index is not strictly necessary. However, we do want to be compatible with ResourceArc,
        // and in other usecases a working index is necessary. Since we have the index anyways, we
        // might as well produce some sort of index if only to make logging easier to follow
        let resource_index =
            DynResourceIndex(self.inner.next_index.fetch_add(1, Ordering::Relaxed));
        self.inner.active_count.fetch_add(1, Ordering::Relaxed);

        log::trace!(
            "insert resource {} {:?}",
            core::any::type_name::<ResourceT>(),
            resource
        );

        ResourceArc::new(resource, resource_index.into(), self.inner.drop_tx.clone())
    }
}

pub struct DynResourceAllocatorManagerInner<ResourceT>
where
    ResourceT: Clone,
{
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    drop_rx: Receiver<ResourceWithHash<ResourceT>>,
    next_allocator_index: AtomicU32,
    active_count: Arc<AtomicU32>,
}

impl<ResourceT> DynResourceAllocatorManagerInner<ResourceT>
where
    ResourceT: Clone + std::fmt::Debug,
{
    fn create_allocator(&self) -> DynResourceAllocator<ResourceT> {
        let allocator_index = self.next_allocator_index.fetch_add(1, Ordering::Relaxed);
        DynResourceAllocator::new(
            self.drop_tx.clone(),
            allocator_index,
            self.active_count.clone(),
        )
    }
}

pub struct DynResourceAllocatorProvider<ResourceT>
where
    ResourceT: Clone,
{
    inner: Arc<DynResourceAllocatorManagerInner<ResourceT>>,
}

impl<ResourceT> DynResourceAllocatorProvider<ResourceT>
where
    ResourceT: Clone + std::fmt::Debug,
{
    fn create_allocator(&self) -> DynResourceAllocator<ResourceT> {
        self.inner.create_allocator()
    }
}

pub struct DynResourceAllocatorManager<ResourceT>
where
    ResourceT: Clone,
{
    inner: Arc<DynResourceAllocatorManagerInner<ResourceT>>,
    drop_sink: ResourceDropSink<ResourceT>,
}

impl<ResourceT> DynResourceAllocatorManager<ResourceT>
where
    ResourceT: Clone + std::fmt::Debug,
{
    fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();
        let drop_sink = ResourceDropSink::new(max_frames_in_flight);

        let inner = DynResourceAllocatorManagerInner {
            drop_tx,
            drop_rx,
            next_allocator_index: AtomicU32::new(1),
            active_count: Arc::new(AtomicU32::new(0)),
        };

        DynResourceAllocatorManager {
            inner: Arc::new(inner),
            drop_sink,
        }
    }

    fn create_allocator(&self) -> DynResourceAllocator<ResourceT> {
        self.inner.create_allocator()
    }

    fn create_allocator_provider(&self) -> DynResourceAllocatorProvider<ResourceT> {
        DynResourceAllocatorProvider {
            inner: self.inner.clone(),
        }
    }

    fn handle_dropped_resources(&mut self) {
        for dropped in self.inner.drop_rx.try_iter() {
            log::trace!(
                "dropping {} {:?}",
                core::any::type_name::<ResourceT>(),
                dropped.resource
            );
            self.drop_sink.retire(dropped.resource);
            self.inner.active_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[profiling::function]
    fn on_frame_complete(&mut self) -> RafxResult<()> {
        self.handle_dropped_resources();
        self.drop_sink.on_frame_complete()?;
        Ok(())
    }

    fn destroy(&mut self) -> RafxResult<()> {
        self.handle_dropped_resources();

        if self.len() > 0 {
            log::warn!(
                "{} resource count {} > 0, resources will leak",
                core::any::type_name::<ResourceT>(),
                self.len()
            );
        }

        self.drop_sink.destroy()?;
        Ok(())
    }

    fn len(&self) -> usize {
        self.inner.active_count.load(Ordering::Relaxed) as usize
    }
}

// This is for providing per-frame allocation where the resource does not need to be
pub struct DynResourceAllocatorSet {
    pub device_context: RafxDeviceContext,
    pub images: DynResourceAllocator<ImageResource>,
    pub image_views: DynResourceAllocator<ImageViewResource>,
    pub buffers: DynResourceAllocator<BufferResource>,
}

impl DynResourceAllocatorSet {
    pub fn insert_texture(
        &self,
        texture: RafxTexture,
    ) -> ResourceArc<ImageResource> {
        let image = RafxImage::Texture(texture);

        let image_resource = ImageResource {
            image_key: None,
            image: Arc::new(image),
        };
        self.images.insert(image_resource)
    }

    pub fn insert_image_view(
        &self,
        image: &ResourceArc<ImageResource>,
        texture_bind_type: Option<RafxTextureBindType>,
    ) -> RafxResult<ResourceArc<ImageViewResource>> {
        Ok(self.insert_image_view_raw(image.clone(), texture_bind_type))
    }

    pub fn insert_image_view_raw(
        &self,
        image: ResourceArc<ImageResource>,
        texture_bind_type: Option<RafxTextureBindType>,
    ) -> ResourceArc<ImageViewResource> {
        let image_view_resource = ImageViewResource {
            image,
            texture_bind_type,
            image_view_key: None,
        };

        self.image_views.insert(image_view_resource)
    }

    pub fn insert_buffer(
        &self,
        buffer: RafxBuffer,
    ) -> ResourceArc<BufferResource> {
        let buffer_resource = BufferResource {
            buffer_key: None,
            buffer: Arc::new(buffer),
        };

        self.buffers.insert(buffer_resource)
    }
}

#[derive(Debug)]
pub struct ResourceMetrics {
    pub image_count: usize,
    pub image_view_count: usize,
    pub buffer_count: usize,
}

pub struct DynResourceAllocatorSetProvider {
    pub device_context: RafxDeviceContext,
    pub images: DynResourceAllocatorProvider<ImageResource>,
    pub image_views: DynResourceAllocatorProvider<ImageViewResource>,
    pub buffers: DynResourceAllocatorProvider<BufferResource>,
}

impl DynResourceAllocatorSetProvider {
    pub fn get_allocator(&self) -> DynResourceAllocatorSet {
        DynResourceAllocatorSet {
            device_context: self.device_context.clone(),
            images: self.images.create_allocator(),
            image_views: self.image_views.create_allocator(),
            buffers: self.buffers.create_allocator(),
        }
    }
}

pub struct DynResourceAllocatorSetManager {
    pub device_context: RafxDeviceContext,
    pub images: DynResourceAllocatorManager<ImageResource>,
    pub image_views: DynResourceAllocatorManager<ImageViewResource>,
    pub buffers: DynResourceAllocatorManager<BufferResource>,
}

impl DynResourceAllocatorSetManager {
    pub fn new(
        device_context: &RafxDeviceContext,
        max_frames_in_flight: u32,
    ) -> Self {
        DynResourceAllocatorSetManager {
            device_context: device_context.clone(),
            images: DynResourceAllocatorManager::new(max_frames_in_flight),
            image_views: DynResourceAllocatorManager::new(max_frames_in_flight),
            buffers: DynResourceAllocatorManager::new(max_frames_in_flight),
        }
    }

    pub fn create_allocator_provider(&self) -> DynResourceAllocatorSetProvider {
        DynResourceAllocatorSetProvider {
            device_context: self.device_context.clone(),
            images: self.images.create_allocator_provider(),
            image_views: self.image_views.create_allocator_provider(),
            buffers: self.buffers.create_allocator_provider(),
        }
    }

    pub fn get_allocator(&self) -> DynResourceAllocatorSet {
        DynResourceAllocatorSet {
            device_context: self.device_context.clone(),
            images: self.images.create_allocator(),
            image_views: self.image_views.create_allocator(),
            buffers: self.buffers.create_allocator(),
        }
    }

    #[profiling::function]
    pub fn on_frame_complete(&mut self) -> RafxResult<()> {
        self.buffers.on_frame_complete()?;
        self.images.on_frame_complete()?;
        self.image_views.on_frame_complete()?;
        Ok(())
    }

    pub fn destroy(&mut self) -> RafxResult<()> {
        //WARNING: These need to be in order of dependencies to avoid frame-delays on destroying
        // resources.
        self.image_views.destroy()?;
        self.images.destroy()?;
        self.buffers.destroy()?;
        Ok(())
    }

    pub fn metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            image_count: self.images.len(),
            image_view_count: self.image_views.len(),
            buffer_count: self.buffers.len(),
        }
    }
}
