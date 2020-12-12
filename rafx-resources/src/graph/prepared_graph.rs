use super::PhysicalImageId;
use crate::graph::graph_buffer::PhysicalBufferId;
use crate::graph::graph_image::PhysicalImageViewId;
use crate::graph::graph_node::{RenderGraphNodeId, RenderGraphNodeName};
use crate::graph::graph_pass::RenderGraphOutputPass;
use crate::graph::graph_plan::RenderGraphPlan;
use crate::graph::{
    RenderGraphBufferSpecification, RenderGraphBufferUsageId, RenderGraphBuilder,
    RenderGraphImageSpecification, RenderGraphImageUsageId,
};
use crate::resources::FramebufferResource;
use crate::resources::RenderPassResource;
use crate::resources::ResourceLookupSet;
use crate::vk_description::SwapchainSurfaceInfo;
use crate::vulkan::VkBuffer;
use crate::{vk_description as dsc, BufferResource, ImageResource};
use crate::{ImageViewResource, ResourceArc, ResourceContext};
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk;
use fnv::{FnvHashMap, FnvHashSet};
use rafx_nodes::{RenderPhase, RenderPhaseIndex};
use rafx_shell_vulkan::{VkDeviceContext, VkImage};
use std::hash::Hash;
use std::sync::{Arc, Mutex};

pub struct ResourceCache<T: Eq + Hash> {
    resources: FnvHashMap<T, u64>,
}

impl<T: Eq + Hash> ResourceCache<T> {
    pub fn new() -> Self {
        ResourceCache {
            resources: Default::default(),
        }
    }

    pub fn touch_resource(
        &mut self,
        resource: T,
        keep_until_frame: u64,
    ) {
        let x = self.resources.entry(resource).or_insert(keep_until_frame);
        *x = keep_until_frame;
    }

    pub fn on_frame_complete(
        &mut self,
        current_frame_index: u64,
    ) {
        self.resources
            .retain(|_, keep_until_frame| *keep_until_frame > current_frame_index);
    }

    pub fn clear(&mut self) {
        self.resources.clear();
    }
}

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
    render_passes: ResourceCache<ResourceArc<RenderPassResource>>,
    framebuffers: ResourceCache<ResourceArc<FramebufferResource>>,
    current_frame_index: u64,
    frames_to_persist: u64,
}

impl RenderGraphCacheInner {
    pub fn new(max_frames_in_flight: u32) -> Self {
        RenderGraphCacheInner {
            buffers: Default::default(),
            images: Default::default(),
            render_passes: ResourceCache::new(),
            framebuffers: ResourceCache::new(),
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

        self.render_passes.on_frame_complete(current_frame_index);
        self.framebuffers.on_frame_complete(current_frame_index);

        self.current_frame_index += 1;
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
        self.images.clear();
        self.render_passes.clear();
        self.framebuffers.clear();
    }

    fn allocate_buffers(
        &mut self,
        device_context: &VkDeviceContext,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
    ) -> VkResult<FnvHashMap<PhysicalBufferId, ResourceArc<BufferResource>>> {
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
                let buffer = VkBuffer::new(
                    device_context,
                    rafx_shell_vulkan::vk_mem::MemoryUsage::GpuOnly,
                    key.specification.usage_flags,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    key.specification.size,
                )?;
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

    fn allocate_images(
        &mut self,
        device_context: &VkDeviceContext,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<FnvHashMap<PhysicalImageId, ResourceArc<ImageResource>>> {
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
                let extent = key
                    .specification
                    .extents
                    .into_vk_extent_3d(&key.swapchain_surface_info);
                let image = VkImage::new(
                    device_context,
                    rafx_shell_vulkan::vk_mem::MemoryUsage::GpuOnly,
                    key.specification.create_flags,
                    key.specification.usage_flags,
                    extent,
                    key.specification.format,
                    vk::ImageTiling::OPTIMAL,
                    key.specification.samples,
                    specification.layer_count,
                    1,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                )?;
                let image = resources.insert_image(image);

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

    fn allocate_image_views(
        &mut self,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
        image_resources: &FnvHashMap<PhysicalImageId, ResourceArc<ImageResource>>,
    ) -> VkResult<FnvHashMap<PhysicalImageViewId, ResourceArc<ImageViewResource>>> {
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

            let specification = &graph.intermediate_images[&view.physical_image];
            let image_view_meta = dsc::ImageViewMeta {
                format: specification.format.into(),
                components: Default::default(),
                subresource_range: view.subresource_range.clone(),
                view_type: view.view_type,
            };

            log::trace!("get_or_create_image_view for {:?}", view.physical_image);
            let image_resource = &image_resources[&view.physical_image];

            let old = image_view_resources.insert(
                id,
                resources.get_or_create_image_view(image_resource, &image_view_meta)?,
            );
            assert!(old.is_none());
        }

        Ok(image_view_resources)
    }

    fn allocate_render_passes(
        &mut self,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<Vec<Option<ResourceArc<RenderPassResource>>>> {
        log::trace!("Allocate renderpasses for rendergraph");
        let mut render_pass_resources = Vec::with_capacity(graph.passes.len());
        for (pass_index, pass) in graph.passes.iter().enumerate() {
            if let RenderGraphOutputPass::Renderpass(pass) = pass {
                let render_pass_resource = resources
                    .get_or_create_renderpass(pass.description.clone(), swapchain_surface_info)?;
                log::trace!(
                    "(pass {}) Keep renderpass {:?} until {}",
                    pass_index,
                    render_pass_resource.get_raw().renderpass,
                    self.current_frame_index + self.frames_to_persist
                );
                self.render_passes.touch_resource(
                    render_pass_resource.clone(),
                    self.current_frame_index + self.frames_to_persist,
                );
                render_pass_resources.push(Some(render_pass_resource));
            } else {
                render_pass_resources.push(None);
            }
        }
        Ok(render_pass_resources)
    }

    fn allocate_framebuffers(
        &mut self,
        graph: &RenderGraphPlan,
        resources: &ResourceLookupSet,
        image_resources: &FnvHashMap<PhysicalImageViewId, ResourceArc<ImageViewResource>>,
        render_pass_resources: &Vec<Option<ResourceArc<RenderPassResource>>>,
    ) -> VkResult<Vec<Option<ResourceArc<FramebufferResource>>>> {
        log::trace!("Allocate framebuffers for rendergraph");
        let mut framebuffers = Vec::with_capacity(graph.passes.len());
        for (pass_index, pass) in graph.passes.iter().enumerate() {
            if let RenderGraphOutputPass::Renderpass(pass) = pass {
                let attachments: Vec<_> = pass
                    .attachment_images
                    .iter()
                    .map(|x| image_resources[x].clone())
                    .collect();

                let framebuffer_meta = dsc::FramebufferMeta {
                    width: pass.extents.width,
                    height: pass.extents.height,
                    layers: 1,
                };

                let framebuffer = resources.get_or_create_framebuffer(
                    render_pass_resources[pass_index].as_ref().unwrap().clone(),
                    &attachments,
                    &framebuffer_meta,
                )?;

                log::trace!(
                    "(pass {}) Keep framebuffer {:?} until {}",
                    pass_index,
                    framebuffer.get_raw().framebuffer,
                    self.current_frame_index + self.frames_to_persist
                );

                self.framebuffers.touch_resource(
                    framebuffer.clone(),
                    self.current_frame_index + self.frames_to_persist,
                );
                framebuffers.push(Some(framebuffer));
            } else {
                framebuffers.push(None);
            }
        }
        Ok(framebuffers)
    }
}

#[derive(Clone)]
pub struct RenderGraphCache {
    inner: Arc<Mutex<RenderGraphCacheInner>>,
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

#[derive(Copy, Clone)]
pub struct RenderGraphContext<'a> {
    prepared_graph: &'a PreparedRenderGraph,
}

impl<'a> RenderGraphContext<'a> {
    pub fn buffer(
        &self,
        buffer: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        self.prepared_graph.buffer(buffer)
    }

    pub fn image_view(
        &self,
        image: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        self.prepared_graph.image_view(image)
    }

    pub fn device_context(&self) -> &VkDeviceContext {
        &self.prepared_graph.device_context
    }

    pub fn resource_context(&self) -> &ResourceContext {
        &self.prepared_graph.resource_context
    }
}

pub struct VisitComputeNodeArgs<'a> {
    pub command_buffer: vk::CommandBuffer,
    pub graph_context: RenderGraphContext<'a>,
}

pub struct VisitRenderpassNodeArgs<'a> {
    pub command_buffer: vk::CommandBuffer,
    pub renderpass_resource: &'a ResourceArc<RenderPassResource>,
    pub framebuffer_resource: &'a ResourceArc<FramebufferResource>,
    pub subpass_index: usize,
    pub graph_context: RenderGraphContext<'a>,
}

/// Encapsulates a render graph plan and all resources required to execute it
pub struct PreparedRenderGraph {
    device_context: VkDeviceContext,
    resource_context: ResourceContext,
    buffer_resources: FnvHashMap<PhysicalBufferId, ResourceArc<BufferResource>>,
    image_resources: FnvHashMap<PhysicalImageId, ResourceArc<ImageResource>>,
    image_view_resources: FnvHashMap<PhysicalImageViewId, ResourceArc<ImageViewResource>>,
    renderpass_resources: Vec<Option<ResourceArc<RenderPassResource>>>,
    framebuffer_resources: Vec<Option<ResourceArc<FramebufferResource>>>,
    graph_plan: RenderGraphPlan,
}

impl PreparedRenderGraph {
    pub fn node_debug_name(
        &self,
        node_id: RenderGraphNodeId,
    ) -> Option<RenderGraphNodeName> {
        let pass_index = *self.graph_plan.node_to_pass_index.get(&node_id)?;
        self.graph_plan.passes[pass_index].debug_name()
    }

    pub fn new(
        device_context: &VkDeviceContext,
        resource_context: &ResourceContext,
        resources: &ResourceLookupSet,
        graph: RenderGraphBuilder,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> VkResult<Self> {
        let graph_plan = graph.build_plan(swapchain_surface_info);
        let mut cache_guard = resource_context.render_graph_cache().inner.lock().unwrap();
        let cache = &mut *cache_guard;

        profiling::scope!("allocate resources");
        let buffer_resources = cache.allocate_buffers(device_context, &graph_plan, resources)?;

        let image_resources = cache.allocate_images(
            device_context,
            &graph_plan,
            resources,
            swapchain_surface_info,
        )?;

        let image_view_resources =
            cache.allocate_image_views(&graph_plan, resources, &image_resources)?;

        let render_pass_resources =
            cache.allocate_render_passes(&graph_plan, resources, swapchain_surface_info)?;

        let framebuffer_resources = cache.allocate_framebuffers(
            &graph_plan,
            resources,
            &image_view_resources,
            &render_pass_resources,
        )?;

        Ok(PreparedRenderGraph {
            device_context: device_context.clone(),
            resource_context: resource_context.clone(),
            buffer_resources,
            image_resources,
            image_view_resources,
            renderpass_resources: render_pass_resources,
            framebuffer_resources,
            graph_plan,
        })
    }

    fn buffer(
        &self,
        buffer: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        let physical_buffer = self.graph_plan.buffer_usage_to_physical.get(&buffer)?;
        self.buffer_resources.get(physical_buffer).cloned()
    }

    fn image_view(
        &self,
        image: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        let physical_image = self.graph_plan.image_usage_to_view.get(&image)?;
        self.image_view_resources.get(physical_image).cloned()
    }

    pub fn execute_graph(
        &self,
        node_visitor: &dyn RenderGraphNodeVisitor,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        profiling::scope!("Execute Graph");
        //
        // Start a command writer. For now just do a single primary writer, later we can multithread this.
        //
        let mut command_writer = self
            .resource_context
            .dyn_command_writer_allocator()
            .allocate_writer(
                self.device_context
                    .queue_family_indices()
                    .graphics_queue_family_index,
                vk::CommandPoolCreateFlags::TRANSIENT,
                0,
            )?;

        let command_buffer = command_writer.begin_command_buffer(
            vk::CommandBufferLevel::PRIMARY,
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            None,
        )?;

        let device = self.device_context.device();

        let render_graph_context = RenderGraphContext {
            prepared_graph: &self,
        };

        //
        // Iterate through all passes
        //
        for (pass_index, pass) in self.graph_plan.passes.iter().enumerate() {
            //TODO output pass is?
            //TODO: add_compute_node/add_render_node?

            profiling::scope!("pass", pass.debug_name().unwrap_or("unnamed"));
            log::trace!("Execute pass name: {:?}", pass.debug_name());

            assert_eq!(pass.nodes().len(), 1);
            let subpass_index = 0;
            let node_id = pass.nodes()[subpass_index];

            unsafe {
                if let Some(pre_pass_barrier) = pass.pre_pass_barrier() {
                    let mut buffer_memory_barriers =
                        Vec::with_capacity(pre_pass_barrier.buffer_barriers.len());

                    for buffer_barrier in &pre_pass_barrier.buffer_barriers {
                        log::trace!("add buffer barrier for buffer {:?}", buffer_barrier.buffer);
                        let buffer = &self.buffer_resources[&buffer_barrier.buffer];

                        let buffer_memory_barrier = vk::BufferMemoryBarrier::builder()
                            .src_access_mask(buffer_barrier.src_access)
                            .dst_access_mask(buffer_barrier.dst_access)
                            .src_queue_family_index(buffer_barrier.src_queue_family_index)
                            .dst_queue_family_index(buffer_barrier.dst_queue_family_index)
                            .buffer(buffer.get_raw().buffer.buffer)
                            .offset(0)
                            .size(buffer_barrier.size)
                            .build();

                        buffer_memory_barriers.push(buffer_memory_barrier);
                    }

                    let mut image_memory_barriers =
                        Vec::with_capacity(pre_pass_barrier.image_barriers.len());

                    for image_barrier in &pre_pass_barrier.image_barriers {
                        log::trace!("add image barrier for image {:?}", image_barrier.image);
                        let image = &self.image_resources[&image_barrier.image];
                        let subresource_range = image_barrier.subresource_range.clone().into();

                        let image_memory_barrier = vk::ImageMemoryBarrier::builder()
                            .src_access_mask(image_barrier.src_access)
                            .dst_access_mask(image_barrier.dst_access)
                            .old_layout(image_barrier.old_layout)
                            .new_layout(image_barrier.new_layout)
                            .src_queue_family_index(image_barrier.src_queue_family_index)
                            .dst_queue_family_index(image_barrier.dst_queue_family_index)
                            .image(image.get_raw().image.image)
                            .subresource_range(subresource_range)
                            .build();

                        image_memory_barriers.push(image_memory_barrier);
                    }

                    device.cmd_pipeline_barrier(
                        command_buffer,
                        pre_pass_barrier.src_stage,
                        pre_pass_barrier.dst_stage,
                        vk::DependencyFlags::empty(),
                        &[],
                        &buffer_memory_barriers,
                        &image_memory_barriers,
                    );
                }

                match pass {
                    RenderGraphOutputPass::Renderpass(pass) => {
                        let renderpass_resource =
                            self.renderpass_resources[pass_index].as_ref().unwrap();
                        let framebuffer_resource =
                            self.framebuffer_resources[pass_index].as_ref().unwrap();
                        // this code will need to be updated to support subpasses
                        debug_assert_eq!(1, pass.subpass_nodes.len());

                        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                            .render_pass(renderpass_resource.get_raw().renderpass)
                            .framebuffer(framebuffer_resource.get_raw().framebuffer)
                            .render_area(vk::Rect2D {
                                offset: vk::Offset2D { x: 0, y: 0 },
                                extent: pass.extents,
                            })
                            .clear_values(&pass.clear_values);

                        device.cmd_begin_render_pass(
                            command_buffer,
                            &render_pass_begin_info,
                            vk::SubpassContents::INLINE,
                        );

                        let args = VisitRenderpassNodeArgs {
                            renderpass_resource,
                            framebuffer_resource,
                            graph_context: render_graph_context,
                            subpass_index,
                            command_buffer,
                        };

                        node_visitor.visit_renderpass_node(node_id, args)?;

                        device.cmd_end_render_pass(command_buffer);
                    }
                    RenderGraphOutputPass::Compute(_pass) => {
                        let args = VisitComputeNodeArgs {
                            graph_context: render_graph_context,
                            command_buffer,
                        };

                        node_visitor.visit_compute_node(node_id, args)?;
                    }
                }
            }
        }

        command_writer.end_command_buffer()?;

        Ok(vec![command_buffer])
    }
}

pub trait RenderGraphNodeVisitor {
    fn visit_renderpass_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitRenderpassNodeArgs,
    ) -> VkResult<()>;

    fn visit_compute_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitComputeNodeArgs,
    ) -> VkResult<()>;
}

type RenderGraphNodeVisitRenderpassNodeCallback<RenderGraphUserContextT> =
    dyn Fn(VisitRenderpassNodeArgs, &RenderGraphUserContextT) -> VkResult<()> + Send;

type RenderGraphNodeVisitComputeNodeCallback<RenderGraphUserContextT> =
    dyn Fn(VisitComputeNodeArgs, &RenderGraphUserContextT) -> VkResult<()> + Send;

enum RenderGraphNodeVisitNodeCallback<RenderGraphUserContextT> {
    Renderpass(Box<RenderGraphNodeVisitRenderpassNodeCallback<RenderGraphUserContextT>>),
    Compute(Box<RenderGraphNodeVisitComputeNodeCallback<RenderGraphUserContextT>>),
}

/// Created by RenderGraphNodeCallbacks::create_visitor(). Implements RenderGraphNodeVisitor and
/// forwards the call, adding the user context as a parameter.
struct RenderGraphNodeVisitorImpl<'b, RenderGraphUserContextT> {
    context: &'b RenderGraphUserContextT,
    callbacks: &'b FnvHashMap<
        RenderGraphNodeId,
        RenderGraphNodeVisitNodeCallback<RenderGraphUserContextT>,
    >,
}

impl<'b, RenderGraphUserContextT> RenderGraphNodeVisitor
    for RenderGraphNodeVisitorImpl<'b, RenderGraphUserContextT>
{
    fn visit_renderpass_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitRenderpassNodeArgs,
    ) -> VkResult<()> {
        if let Some(callback) = self.callbacks.get(&node_id) {
            if let RenderGraphNodeVisitNodeCallback::Renderpass(render_callback) = callback {
                (render_callback)(args, self.context)?
            } else {
                let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
                log::error!("Tried to call a render node callback but a compute callback was registered for node {:?} ({:?})", node_id, debug_name);
            }
        } else {
            //let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
            //log::error!("No callback found for node {:?} ({:?})", node_id, debug_name);
        }

        Ok(())
    }

    fn visit_compute_node(
        &self,
        node_id: RenderGraphNodeId,
        args: VisitComputeNodeArgs,
    ) -> VkResult<()> {
        if let Some(callback) = self.callbacks.get(&node_id) {
            if let RenderGraphNodeVisitNodeCallback::Compute(compute_callback) = callback {
                (compute_callback)(args, self.context)?
            } else {
                let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
                log::error!("Tried to call a compute node callback but a render node callback was registered for node {:?} ({:?})", node_id, debug_name);
            }
        } else {
            //let debug_name = args.graph_context.prepared_graph.node_debug_name(node_id);
            //log::error!("No callback found for node {:?} {:?}", node_id, debug_name);
        }

        Ok(())
    }
}

/// All the callbacks associated with rendergraph nodes. We keep them separate from the nodes so
/// that we can avoid propagating generic parameters throughout the rest of the rendergraph code
pub struct RenderGraphNodeCallbacks<RenderGraphUserContextT> {
    callbacks:
        FnvHashMap<RenderGraphNodeId, RenderGraphNodeVisitNodeCallback<RenderGraphUserContextT>>,
    render_phase_dependencies: FnvHashMap<RenderGraphNodeId, FnvHashSet<RenderPhaseIndex>>,
}

impl<RenderGraphUserContextT> RenderGraphNodeCallbacks<RenderGraphUserContextT> {
    /// Adds a callback that receives the renderpass associated with the node
    pub fn set_renderpass_callback<CallbackFnT>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: CallbackFnT,
    ) where
        CallbackFnT:
            Fn(VisitRenderpassNodeArgs, &RenderGraphUserContextT) -> VkResult<()> + 'static + Send,
    {
        let old = self.callbacks.insert(
            node_id,
            RenderGraphNodeVisitNodeCallback::Renderpass(Box::new(f)),
        );
        // If this trips, multiple callbacks were set on the node
        assert!(old.is_none());
    }

    /// Adds a callback for compute based nodes
    pub fn set_compute_callback<CallbackFnT>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: CallbackFnT,
    ) where
        CallbackFnT:
            Fn(VisitComputeNodeArgs, &RenderGraphUserContextT) -> VkResult<()> + 'static + Send,
    {
        let old = self.callbacks.insert(
            node_id,
            RenderGraphNodeVisitNodeCallback::Compute(Box::new(f)),
        );
        // If this trips, multiple callbacks were set on the node
        assert!(old.is_none());
    }

    pub fn add_renderphase_dependency<PhaseT: RenderPhase>(
        &mut self,
        node_id: RenderGraphNodeId,
    ) {
        self.render_phase_dependencies
            .entry(node_id)
            .or_default()
            .insert(PhaseT::render_phase_index());
    }

    /// Pass to PreparedRenderGraph::execute_graph, this will cause the graph to be executed,
    /// triggering any registered callbacks
    pub fn create_visitor<'a>(
        &'a self,
        context: &'a RenderGraphUserContextT,
    ) -> Box<dyn RenderGraphNodeVisitor + 'a> {
        Box::new(RenderGraphNodeVisitorImpl::<'a, RenderGraphUserContextT> {
            context,
            callbacks: &self.callbacks,
        })
    }
}

impl<T> Default for RenderGraphNodeCallbacks<T> {
    fn default() -> Self {
        RenderGraphNodeCallbacks {
            callbacks: Default::default(),
            render_phase_dependencies: Default::default(),
        }
    }
}

/// A wrapper around a prepared render graph and callbacks that will be hit when executing the graph
pub struct RenderGraphExecutor<T> {
    prepared_graph: PreparedRenderGraph,
    callbacks: RenderGraphNodeCallbacks<T>,
}

impl<T> RenderGraphExecutor<T> {
    /// Create the executor. This allows the prepared graph, resources required to execute it, and
    /// callbacks that will be triggered while executing it to be passed around and executed later.
    pub fn new(
        device_context: &VkDeviceContext,
        resource_context: &ResourceContext,
        graph: RenderGraphBuilder,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        callbacks: RenderGraphNodeCallbacks<T>,
    ) -> VkResult<Self> {
        //
        // Allocate the resources for the graph
        //
        let prepared_graph = PreparedRenderGraph::new(
            device_context,
            resource_context,
            resource_context.resources(),
            graph,
            swapchain_surface_info,
        )?;

        //
        // Pre-warm caches for pipelines that we may need
        //
        for (node_id, render_phase_indices) in &callbacks.render_phase_dependencies {
            // Passes may get culled if the images are not used. This means the renderpass would
            // not be created so pipelines are also not needed
            if let Some(&renderpass_index) =
                prepared_graph.graph_plan.node_to_pass_index.get(node_id)
            {
                let renderpass = &prepared_graph.renderpass_resources[renderpass_index];
                if let Some(renderpass) = renderpass {
                    for &render_phase_index in render_phase_indices {
                        resource_context
                            .graphics_pipeline_cache()
                            .register_renderpass_to_phase_index_per_frame(
                                renderpass,
                                render_phase_index,
                            )
                    }
                } else {
                    log::error!("add_renderphase_dependency was called on node {:?} ({:?}) that is not a renderpass", node_id, prepared_graph.graph_plan.passes[renderpass_index].debug_name());
                }
            }
        }
        resource_context
            .graphics_pipeline_cache()
            .precache_pipelines_for_all_phases()?;

        //
        // Return the executor which can be triggered later
        //
        Ok(RenderGraphExecutor {
            prepared_graph,
            callbacks,
        })
    }

    pub fn renderpass_resource(
        &self,
        node_id: RenderGraphNodeId,
    ) -> Option<ResourceArc<RenderPassResource>> {
        let renderpass_index = *self
            .prepared_graph
            .graph_plan
            .node_to_pass_index
            .get(&node_id)?;
        self.prepared_graph.renderpass_resources[renderpass_index].clone()
    }

    pub fn buffer_resource(
        &self,
        buffer_usage: RenderGraphBufferUsageId,
    ) -> Option<ResourceArc<BufferResource>> {
        let buffer = self
            .prepared_graph
            .graph_plan
            .buffer_usage_to_physical
            .get(&buffer_usage)?;
        Some(self.prepared_graph.buffer_resources[buffer].clone())
    }

    pub fn image_resource(
        &self,
        image_usage: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageResource>> {
        let image = self
            .prepared_graph
            .graph_plan
            .image_usage_to_physical
            .get(&image_usage)?;
        Some(self.prepared_graph.image_resources[image].clone())
    }

    pub fn image_view_resource(
        &self,
        image_usage: RenderGraphImageUsageId,
    ) -> Option<ResourceArc<ImageViewResource>> {
        let image = self
            .prepared_graph
            .graph_plan
            .image_usage_to_view
            .get(&image_usage)?;
        Some(self.prepared_graph.image_view_resources[image].clone())
    }

    /// Executes the graph, passing through the given context parameter
    pub fn execute_graph(
        self,
        context: &T,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let visitor = self.callbacks.create_visitor(context);
        self.prepared_graph.execute_graph(&*visitor)
    }
}
