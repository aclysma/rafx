use crate::graph::graph_buffer::PhysicalBufferId;
use crate::graph::graph_image::{PhysicalImageId, PhysicalImageViewId};
use crate::graph::{
    RenderGraphBufferSpecification, RenderGraphImageSpecification, RenderGraphPlan,
    SwapchainSurfaceInfo,
};
use crate::{BufferResource, ImageResource, ImageViewResource, ResourceArc, ResourceLookupSet};
use fnv::FnvHashMap;
use rafx_api::{
    RafxBufferDef, RafxDeviceContext, RafxMemoryUsage, RafxRenderTargetDef, RafxResult,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct RenderGraphCachedBufferKey {
    specification: RenderGraphBufferSpecification,
}

struct RenderGraphCachedBuffer {
    keep_until_frame: u64,
    buffer: ResourceArc<BufferResource>,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct RenderGraphCachedImageKey {
    specification: RenderGraphImageSpecification,
    swapchain_surface_info: SwapchainSurfaceInfo,
}

struct RenderGraphCachedImage {
    keep_until_frame: u64,
    image: ResourceArc<ImageResource>,
}

pub struct RenderGraphCacheInner {
    buffers: FnvHashMap<RenderGraphCachedBufferKey, Vec<RenderGraphCachedBuffer>>,
    images: FnvHashMap<RenderGraphCachedImageKey, Vec<RenderGraphCachedImage>>,
    current_frame_index: u64,
    frames_to_persist: u64,
}

impl RenderGraphCacheInner {
    pub fn new(max_frames_in_flight: u32) -> Self {
        RenderGraphCacheInner {
            buffers: Default::default(),
            images: Default::default(),
            current_frame_index: 0,
            frames_to_persist: max_frames_in_flight as u64 + 1,
        }
    }

    pub fn on_frame_complete(&mut self) {
        //println!("-- FRAME COMPLETE -- drop framebuffer if keep_until <= {}", self.current_frame_index);
        let current_frame_index = self.current_frame_index;

        for value in self.buffers.values_mut() {
            value.retain(|x| x.keep_until_frame > current_frame_index);
        }

        self.buffers.retain(|_k, v| !v.is_empty());

        for value in self.images.values_mut() {
            value.retain(|x| x.keep_until_frame > current_frame_index);
        }

        self.images.retain(|_k, v| !v.is_empty());

        self.current_frame_index += 1;
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
        self.images.clear();
    }

    pub(super) fn allocate_buffers(
        &mut self,
        device_context: &RafxDeviceContext,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
    ) -> RafxResult<FnvHashMap<PhysicalBufferId, ResourceArc<BufferResource>>> {
        log::trace!("Allocate buffers for rendergraph");
        let mut buffer_resources: FnvHashMap<PhysicalBufferId, ResourceArc<BufferResource>> =
            Default::default();

        // Keeps track of what index in the cache we will use next. This starts at 0 for each key
        // and increments every time we use an image. If the next image is >= length of buffers, we
        // allocate one and push it into that key's list of cached buffers
        let mut next_buffer_to_use = FnvHashMap::<RenderGraphCachedBufferKey, usize>::default();

        // Using a buffer will bump the keep_until_frame for that buffer
        let keep_until_frame = self.current_frame_index + self.frames_to_persist;

        for (&physical_id, buffer) in &graph.output_buffers {
            buffer_resources.insert(physical_id, buffer.dst_buffer.clone());
        }

        // Iterate all intermediate buffers, assigning an existing buffer from a previous frame or
        // allocating a new one
        for (&id, specification) in &graph.intermediate_buffers {
            let key = RenderGraphCachedBufferKey {
                specification: specification.clone(),
            };

            let next_buffer_index = next_buffer_to_use.entry(key.clone()).or_insert(0);
            let matching_cached_buffers = self
                .buffers
                .entry(key.clone())
                .or_insert_with(Default::default);

            if let Some(cached_buffer) = matching_cached_buffers.get_mut(*next_buffer_index) {
                log::trace!(
                    "  Buffer {:?} - REUSE {:?}  (key: {:?}, index: {})",
                    id,
                    cached_buffer.buffer,
                    key,
                    next_buffer_index
                );

                // Reuse a buffer from a previous frame, bump keep_until_frame
                cached_buffer.keep_until_frame = keep_until_frame;
                *next_buffer_index += 1;

                buffer_resources.insert(id, cached_buffer.buffer.clone());
            } else {
                // No unused buffer available, create one
                let buffer = device_context.create_buffer(&RafxBufferDef {
                    size: key.specification.size,
                    //alignment: key.specification.alignment,
                    memory_usage: RafxMemoryUsage::GpuOnly,
                    resource_type: key.specification.resource_type,
                    //initial_state: key.specification.initial_state,
                    ..Default::default()
                })?;
                let buffer = resources.insert_buffer(buffer);

                log::trace!(
                    "  Buffer {:?} - CREATE {:?}  (key: {:?}, index: {})",
                    id,
                    buffer.get_raw().buffer,
                    key,
                    next_buffer_index
                );

                // Add the buffer to the cache
                debug_assert_eq!(matching_cached_buffers.len(), *next_buffer_index);
                matching_cached_buffers.push(RenderGraphCachedBuffer {
                    keep_until_frame,
                    buffer: buffer.clone(),
                });
                *next_buffer_index += 1;

                // Associate the physical id with this buffer
                buffer_resources.insert(id, buffer);
            }
        }

        Ok(buffer_resources)
    }

    pub(super) fn allocate_images(
        &mut self,
        device_context: &RafxDeviceContext,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> RafxResult<FnvHashMap<PhysicalImageId, ResourceArc<ImageResource>>> {
        log::trace!("Allocate images for rendergraph");
        let mut image_resources: FnvHashMap<PhysicalImageId, ResourceArc<ImageResource>> =
            Default::default();

        // Keeps track of what index in the cache we will use next. This starts at 0 for each key
        // and increments every time we use an image. If the next image is >= length of images, we
        // allocate one and push it into that key's list of cached images
        let mut next_image_to_use = FnvHashMap::<RenderGraphCachedImageKey, usize>::default();

        // Using an image will bump the keep_until_frame for that image
        let keep_until_frame = self.current_frame_index + self.frames_to_persist;

        for (id, image) in &graph.output_images {
            let physical_id = graph.image_views[id.0].physical_image;
            image_resources.insert(physical_id, image.dst_image.get_raw().image);
        }

        // Iterate all intermediate images, assigning an existing image from a previous frame or
        // allocating a new one
        for (&id, specification) in &graph.intermediate_images {
            let key = RenderGraphCachedImageKey {
                specification: specification.clone(),
                swapchain_surface_info: swapchain_surface_info.clone(),
            };

            let next_image_index = next_image_to_use.entry(key.clone()).or_insert(0);
            let matching_cached_images = self
                .images
                .entry(key.clone())
                .or_insert_with(Default::default);

            if let Some(cached_image) = matching_cached_images.get_mut(*next_image_index) {
                log::trace!(
                    "  Image {:?} - REUSE {:?}  (key: {:?}, index: {})",
                    id,
                    cached_image.image.get_raw().image,
                    key,
                    next_image_index
                );

                // Reuse an image from a previous frame, bump keep_until_frame
                cached_image.keep_until_frame = keep_until_frame;
                *next_image_index += 1;

                image_resources.insert(id, cached_image.image.clone());
            } else {
                // No unused image available, create one
                let extents = key
                    .specification
                    .extents
                    .into_rafx_extents(&key.swapchain_surface_info);

                let render_target = device_context.create_render_target(&RafxRenderTargetDef {
                    extents,
                    array_length: specification.layer_count,
                    mip_count: specification.mip_count,
                    format: specification.format,
                    sample_count: specification.samples,
                    resource_type: specification.resource_type,
                    dimensions: Default::default(),
                })?;
                let image = resources.insert_render_target(render_target);

                log::trace!(
                    "  Image {:?} - CREATE {:?}  (key: {:?}, index: {})",
                    id,
                    image.get_raw().image,
                    key,
                    next_image_index
                );

                // Add the image to the cache
                debug_assert_eq!(matching_cached_images.len(), *next_image_index);
                matching_cached_images.push(RenderGraphCachedImage {
                    keep_until_frame,
                    image: image.clone(),
                });
                *next_image_index += 1;

                // Associate the physical id with this image
                image_resources.insert(id, image);
            }
        }

        Ok(image_resources)
    }

    pub(super) fn allocate_image_views(
        &mut self,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
        image_resources: &FnvHashMap<PhysicalImageId, ResourceArc<ImageResource>>,
    ) -> RafxResult<FnvHashMap<PhysicalImageViewId, ResourceArc<ImageViewResource>>> {
        let mut image_view_resources: FnvHashMap<
            PhysicalImageViewId,
            ResourceArc<ImageViewResource>,
        > = Default::default();

        // For output images, the physical id just needs to be associated with the image provided by
        // the user
        for (id, image) in &graph.output_images {
            image_view_resources.insert(*id, image.dst_image.clone());
        }

        for (id, view) in graph.image_views.iter().enumerate() {
            let id = PhysicalImageViewId(id);

            // Skip output images (handled above). They already have ImageViewResources
            if image_view_resources.contains_key(&id) {
                continue;
            }

            log::trace!("get_or_create_image_view for {:?}", view.physical_image);
            let image_resource = &image_resources[&view.physical_image];

            let old = image_view_resources.insert(
                id,
                resources.get_or_create_image_view(
                    image_resource,
                    view.view_options.texture_bind_type,
                )?,
            );
            assert!(old.is_none());
        }

        Ok(image_view_resources)
    }
}

#[derive(Clone)]
pub struct RenderGraphCache {
    pub(super) inner: Arc<Mutex<RenderGraphCacheInner>>,
}

impl RenderGraphCache {
    pub fn new(max_frames_in_flight: u32) -> Self {
        let inner = RenderGraphCacheInner::new(max_frames_in_flight);
        RenderGraphCache {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    #[profiling::function]
    pub fn on_frame_complete(&self) {
        self.inner.lock().unwrap().on_frame_complete();
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}
