use rafx_assets::distill_impl::AssetResource;
use rafx_assets::{image_upload, AssetManagerRenderResource, GpuImageDataColorSpace};
use rafx_assets::{AssetManager, GpuImageData};
use rafx_framework::render_features::render_features_prelude::*;
use rafx_framework::visibility::{VisibilityConfig, VisibilityRegion};
use rafx_framework::{DynResourceAllocatorSet, RenderResources};
use rafx_framework::{ImageViewResource, ResourceArc};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;

use super::{RenderFeaturePlugin, RenderGraphGenerator, ViewportsResource};
use rafx_api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx_api::{
    RafxDeviceContext, RafxError, RafxPresentableFrame, RafxQueue, RafxResourceType, RafxResult,
    RafxSwapchainHelper,
};
use rafx_assets::image_upload::ImageUploadParams;

#[derive(Default, Copy, Clone, Debug)]
pub struct RendererConfigResource {
    pub visibility_config: VisibilityConfig,
}

#[derive(Clone)]
pub struct InvalidResources {
    pub invalid_image_color: ResourceArc<ImageViewResource>,
    pub invalid_image_depth: ResourceArc<ImageViewResource>,
    pub invalid_cube_map_image_color: ResourceArc<ImageViewResource>,
    pub invalid_cube_map_image_depth: ResourceArc<ImageViewResource>,
}

pub struct RendererInner {
    // This is a separate lock
    pub(super) temporary_work: RenderJobExtractAllocationContext,
    pub(super) thread_pool: Box<dyn RendererThreadPool>,
}

pub struct Renderer {
    pub(super) inner: Arc<Mutex<RendererInner>>,
    pub(super) render_thread: Option<RenderThread>,
    pub(super) render_graph_generator: Box<dyn RenderGraphGenerator>,
    pub(super) asset_plugins: Vec<Arc<dyn RendererAssetPlugin>>,
    pub(super) feature_plugins: Arc<Vec<Arc<dyn RenderFeaturePlugin>>>,
    pub(super) render_resources: Arc<RenderResources>,
    pub(super) graphics_queue: RafxQueue,
    pub(super) transfer_queue: RafxQueue,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        for plugin in &*self.feature_plugins {
            plugin
                .prepare_renderer_destroy(&self.render_resources)
                .unwrap();
        }

        for plugin in &self.asset_plugins {
            plugin
                .prepare_renderer_destroy(&self.render_resources)
                .unwrap();
        }
    }
}

impl Renderer {
    pub fn new(
        extract_resources: ExtractResources,
        asset_resource: &mut AssetResource,
        asset_manager: &mut AssetManager,
        graphics_queue: &RafxQueue,
        transfer_queue: &RafxQueue,
        feature_plugins: Vec<Arc<dyn RenderFeaturePlugin>>,
        asset_plugins: Vec<Arc<dyn RendererAssetPlugin>>,
        render_graph_generator: Box<dyn RenderGraphGenerator>,
        thread_pool: Box<dyn RendererThreadPool>,
        allow_use_render_thread: bool,
    ) -> RafxResult<Self> {
        let feature_plugins = Arc::new(feature_plugins);

        let device_context = graphics_queue.device_context();

        let dyn_resource_allocator = asset_manager.create_dyn_resource_allocator_set();

        let mut upload = RafxTransferUpload::new(
            &device_context,
            asset_manager.transfer_queue(),
            asset_manager.graphics_queue(),
            16 * 1024 * 1024,
            None,
        )?;

        let invalid_image_color = Self::upload_image_data(
            &device_context,
            &mut upload,
            &dyn_resource_allocator,
            &GpuImageData::new_1x1_rgba8(255, 0, 255, 255, GpuImageDataColorSpace::Linear),
            ImageUploadParams::default(),
        )
        .map_err(|x| Into::<RafxError>::into(x))?;

        let invalid_image_depth = Self::upload_image_data(
            &device_context,
            &mut upload,
            &dyn_resource_allocator,
            &GpuImageData::new_1x1_d32(0.0),
            ImageUploadParams::default(),
        )
        .map_err(|x| Into::<RafxError>::into(x))?;

        let invalid_cube_map_image_color = Self::upload_image_data(
            &device_context,
            &mut upload,
            &dyn_resource_allocator,
            &GpuImageData::new_1x1_rgba8(255, 0, 255, 255, GpuImageDataColorSpace::Linear),
            ImageUploadParams {
                generate_mips: false,
                resource_type: RafxResourceType::TEXTURE_CUBE,
                layer_swizzle: Some(&[0, 0, 0, 0, 0, 0]),
            },
        )
        .map_err(|x| Into::<RafxError>::into(x))?;

        let invalid_cube_map_image_depth = Self::upload_image_data(
            &device_context,
            &mut upload,
            &dyn_resource_allocator,
            &GpuImageData::new_1x1_d32(0.0),
            ImageUploadParams {
                generate_mips: false,
                resource_type: RafxResourceType::TEXTURE_CUBE,
                layer_swizzle: Some(&[0, 0, 0, 0, 0, 0]),
            },
        )
        .map_err(|x| Into::<RafxError>::into(x))?;

        let invalid_resources = InvalidResources {
            invalid_image_color,
            invalid_image_depth,
            invalid_cube_map_image_color,
            invalid_cube_map_image_depth,
        };

        let mut render_resources = RenderResources::default();
        render_resources.insert(SwapchainRenderResource::default());
        render_resources.insert(AssetManagerRenderResource::default());
        render_resources.insert(TimeRenderResource::default());

        for plugin in &*asset_plugins {
            plugin.initialize_static_resources(
                asset_manager,
                asset_resource,
                &extract_resources,
                &mut render_resources,
                &mut upload,
            )?;
        }

        for plugin in &*feature_plugins {
            plugin.initialize_static_resources(
                asset_manager,
                asset_resource,
                &extract_resources,
                &mut render_resources,
                &mut upload,
            )?;
        }

        render_resources.insert(invalid_resources.clone());

        upload.block_until_upload_complete()?;

        let use_render_thread =
            allow_use_render_thread && device_context.device_info().supports_multithreaded_usage;
        let render_thread = if use_render_thread {
            Some(RenderThread::start())
        } else {
            None
        };

        let num_features = RenderRegistry::registered_feature_count() as usize;
        let renderer = RendererInner {
            thread_pool,
            temporary_work: RenderJobExtractAllocationContext::new(num_features),
        };

        Ok(Renderer {
            inner: Arc::new(Mutex::new(renderer)),
            render_thread,
            render_graph_generator,
            asset_plugins,
            feature_plugins,
            render_resources: Arc::new(render_resources),
            graphics_queue: graphics_queue.clone(),
            transfer_queue: transfer_queue.clone(),
        })
    }

    pub fn clear_temporary_work(&mut self) {
        let mut guard = self.inner.lock().unwrap();
        let renderer_inner = &mut *guard;
        renderer_inner.temporary_work.clear();
    }

    pub fn graphics_queue(&self) -> &RafxQueue {
        &self.graphics_queue
    }

    pub fn transfer_queue(&self) -> &RafxQueue {
        &self.transfer_queue
    }

    fn upload_image_data(
        device_context: &RafxDeviceContext,
        upload: &mut RafxTransferUpload,
        dyn_resource_allocator: &DynResourceAllocatorSet,
        image_data: &GpuImageData,
        params: ImageUploadParams,
    ) -> Result<ResourceArc<ImageViewResource>, RafxUploadError> {
        let texture = image_upload::enqueue_load_image(device_context, upload, image_data, params)?;

        let image = dyn_resource_allocator.insert_texture(texture);

        Ok(dyn_resource_allocator.insert_image_view(&image, None)?)
    }

    // This is externally exposed, it checks result of the previous frame (which implicitly also
    // waits for the previous frame to complete if it hasn't already)
    #[profiling::function]
    pub fn start_rendering_next_frame(
        &self,
        extract_resources: &mut ExtractResources,
        previous_update_time: Duration,
    ) -> RafxResult<()> {
        //
        // Block until the previous frame completes being submitted to GPU
        //
        let t0 = rafx_base::Instant::now();

        let presentable_frame = {
            let viewports_resource = extract_resources.fetch::<ViewportsResource>();
            let mut swapchain_helper = extract_resources.fetch_mut::<RafxSwapchainHelper>();
            let mut asset_manager = extract_resources.fetch_mut::<AssetManager>();
            SwapchainHandler::acquire_next_image(
                &mut *swapchain_helper,
                &mut *asset_manager,
                self,
                viewports_resource.main_window_size.width,
                viewports_resource.main_window_size.height,
            )
        }?;

        if let Some(render_thread) = &self.render_thread {
            render_thread.wait_for_render_finish();
        }

        let t1 = rafx_base::Instant::now();
        log::trace!(
            "[main] wait for previous frame present {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        Self::create_and_start_render_job(
            self,
            extract_resources,
            presentable_frame,
            previous_update_time,
        );

        Ok(())
    }

    fn create_and_start_render_job(
        renderer: &Renderer,
        extract_resources: &mut ExtractResources,
        presentable_frame: RafxPresentableFrame,
        previous_update_time: Duration,
    ) {
        let result = Self::try_create_render_job(
            &renderer,
            extract_resources,
            &presentable_frame,
            previous_update_time,
        );

        match result {
            Ok(prepared_frame) => {
                if let Some(render_thread) = &renderer.render_thread {
                    render_thread.render(prepared_frame, presentable_frame);
                } else {
                    // This path is required for backends that do not support multithreaded use
                    prepared_frame.render_async(presentable_frame);
                }
            }
            Err(e) => {
                let graphics_queue = renderer.graphics_queue();
                presentable_frame.present_with_error(graphics_queue, e)
            }
        };
    }

    fn try_create_render_job(
        renderer: &Renderer,
        extract_resources: &mut ExtractResources,
        presentable_frame: &RafxPresentableFrame,
        previous_update_time: Duration,
    ) -> RafxResult<RenderFrameJob> {
        //
        // Fetch resources
        //
        let mut asset_manager_fetch = extract_resources.fetch_mut::<AssetManager>();
        let asset_manager = &mut *asset_manager_fetch;

        let render_registry = asset_manager.resource_manager().render_registry().clone();
        let device_context = asset_manager.device_context().clone();

        //
        // Mark the previous frame as completed
        //
        asset_manager.on_frame_complete()?;

        let resource_context = asset_manager.resource_manager().resource_context();

        let mut guard = renderer.inner.lock().unwrap();
        let renderer_inner = &mut *guard;
        let render_resources = &renderer.render_resources;

        render_resources
            .fetch_mut::<TimeRenderResource>()
            .update(previous_update_time);

        let renderer_config = extract_resources
            .try_fetch::<RendererConfigResource>()
            .map(|x| *x)
            .unwrap_or_default();

        //
        // Swapchain Status
        //
        let swapchain_image = {
            // Temporary hack to jam a swapchain image into the existing resource lookups.. may want
            // to reconsider this later since the ResourceArc can be held past the lifetime of the
            // swapchain image
            let swapchain_image = presentable_frame.swapchain_texture().clone();

            let swapchain_image = resource_context.resources().insert_image(swapchain_image);

            resource_context
                .resources()
                .get_or_create_image_view(&swapchain_image, None)?
        };

        let swapchain_guard = presentable_frame.swapchain().lock().unwrap();
        let max_color_component_value = match &*swapchain_guard {
            #[cfg(feature = "rafx-metal")]
            rafx_api::RafxSwapchain::Metal(swapchain) => {
                swapchain.edr_info().max_edr_color_component_value
            }
            #[allow(unreachable_patterns)]
            _ => 1.0,
        };

        render_resources
            .fetch_mut::<SwapchainRenderResource>()
            .set_max_color_component_value(max_color_component_value);

        let render_view_set = RenderViewSet::new(presentable_frame.incrementing_frame_index());

        //
        // Determine Camera Location
        //

        let viewports_resource = extract_resources.fetch::<ViewportsResource>();
        let view_meta = viewports_resource.main_view_meta.clone().unwrap();

        let main_window_size = viewports_resource.main_window_size;

        let main_view = render_view_set.create_view(
            view_meta.view_frustum,
            view_meta.eye_position,
            view_meta.view,
            view_meta.proj,
            (main_window_size.width, main_window_size.height),
            view_meta.depth_range,
            view_meta.render_phase_mask,
            view_meta.render_feature_mask,
            view_meta.render_feature_flag_mask,
            view_meta.debug_name,
        );

        //TODO: Add main view to a resource of some kind?

        //
        // Compute Views
        //

        let mut render_views = Vec::default();

        {
            profiling::scope!("Compute Views");
            render_views.push(main_view.clone());
            for plugin in &*renderer.feature_plugins {
                plugin.add_render_views(
                    extract_resources,
                    render_resources,
                    &render_view_set,
                    &mut render_views,
                );
            }
        }

        //
        // Update Resources and flush descriptor set changes
        //

        asset_manager.on_begin_frame()?;

        //
        // Build the frame packet - this takes the views and visibility results and creates a
        // structure that's used during the extract/prepare/write phases
        //

        unsafe {
            render_resources
                .fetch_mut::<AssetManagerRenderResource>()
                .begin_extract(&asset_manager);
        }

        let frame_packets = {
            profiling::scope!("Renderer Extract");

            let extract_context = RenderJobExtractContext::new(
                &renderer_inner.temporary_work,
                &extract_resources,
                &render_resources,
                &renderer_config.visibility_config,
            );

            let visibility_results = {
                profiling::scope!("Calculate View Visibility");

                let visibility_region = extract_resources.fetch::<VisibilityRegion>();

                let view_visibility_jobs =
                    Renderer::create_view_visibility_jobs(&render_views, &visibility_region);

                renderer_inner
                    .thread_pool
                    .run_view_visibility_jobs(&view_visibility_jobs, &extract_context)
            };

            {
                profiling::scope!("Determine Frame Packet Sizes");
                renderer_inner
                    .thread_pool
                    .count_render_features_render_objects(
                        &renderer.feature_plugins,
                        &extract_context,
                        &visibility_results,
                    );
            }

            {
                profiling::scope!("Allocate Frame Packets");
                Renderer::create_frame_packets(&renderer.feature_plugins, &extract_context);
            }

            let extract_jobs = {
                profiling::scope!("Create Extract Jobs");
                renderer_inner.thread_pool.create_extract_jobs(
                    &renderer.feature_plugins,
                    &extract_context,
                    visibility_results,
                )
            };

            {
                profiling::scope!("Run Extract Jobs");
                renderer_inner.thread_pool.run_extract_jobs(&extract_jobs);
            }

            Renderer::take_frame_packets(extract_jobs)
        };

        render_resources
            .fetch_mut::<AssetManagerRenderResource>()
            .end_extract();

        //TODO: This is now possible to run on the render thread
        let prepared_render_graph = renderer.render_graph_generator.generate_render_graph(
            asset_manager,
            swapchain_image,
            presentable_frame.rotating_frame_index(),
            main_view.clone(),
            extract_resources,
            render_resources,
        )?;

        let graphics_queue = renderer.graphics_queue.clone();
        let feature_plugins = renderer.feature_plugins.clone();
        let thread_pool = renderer_inner.thread_pool.clone_to_box();
        let render_resources = renderer.render_resources.clone();

        let prepared_frame = RenderFrameJob {
            thread_pool,
            render_resources,
            prepared_render_graph,
            resource_context,
            frame_packets,
            render_registry,
            device_context,
            graphics_queue,
            feature_plugins,
            render_views,
        };

        Ok(prepared_frame)
    }

    fn create_view_visibility_jobs<'visibility>(
        render_views: &[RenderView],
        visibility_region: &'visibility VisibilityRegion,
    ) -> Vec<Arc<ViewVisibilityJob<'visibility>>> {
        render_views
            .iter()
            .map(|view| {
                log::trace!("Add visibility job {}", view.debug_name());
                Arc::new(ViewVisibilityJob::new(view.clone(), visibility_region))
            })
            .collect::<Vec<_>>()
    }

    pub fn run_view_visibility_job<'extract>(
        view_visibility_job: &Arc<ViewVisibilityJob>,
        extract_context: &RenderJobExtractContext<'extract>,
    ) -> RenderViewVisibilityQuery {
        view_visibility_job.query_visibility(extract_context)
    }

    pub fn calculate_frame_packet_size(
        #[allow(unused_variables)] debug_constants: &'static RenderFeatureDebugConstants,
        feature_index: RenderFeatureIndex,
        is_relevant: impl Fn(&RenderViewVisibilityQuery) -> bool,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
        render_object_instance_object_ids: &mut RenderObjectInstanceObjectIds,
        frame_packet_size: &mut FramePacketSize,
    ) {
        profiling::scope!(debug_constants.feature_name);

        for view_visibility_result in visibility_results
            .iter()
            .filter(|result| is_relevant(result))
        {
            if let Some(visible_render_objects) =
                view_visibility_result.render_object_instances_per_view(feature_index)
            {
                for render_object in visible_render_objects {
                    render_object_instance_object_ids.insert(*render_object);
                }

                frame_packet_size.view_packet_sizes.push(ViewPacketSize {
                    view: view_visibility_result.view.clone(),
                    num_render_object_instances: visible_render_objects.len(),
                    num_volumes: 0, // TODO(dvd): Volumes
                });
            } else {
                frame_packet_size.view_packet_sizes.push(ViewPacketSize {
                    view: view_visibility_result.view.clone(),
                    num_render_object_instances: 0,
                    num_volumes: 0, // TODO(dvd): Volumes
                });
            }
        }
    }

    pub fn count_render_feature_render_objects<'extract>(
        feature: &Arc<dyn RenderFeaturePlugin>,
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
    ) {
        profiling::scope!(feature.feature_debug_constants().feature_name);

        let temporary_work = extract_context.allocation_context;

        let mut render_object_instances = temporary_work
            .render_object_instances
            .get(feature.feature_index() as usize)
            .unwrap()
            .borrow_mut();
        render_object_instances.clear();

        let mut frame_packet_metadata = temporary_work
            .frame_packet_metadata
            .get(feature.feature_index() as usize)
            .unwrap()
            .borrow_mut();

        frame_packet_metadata
            .frame_packet_size
            .view_packet_sizes
            .clear();

        // NOTE(dvd): Delegate to the feature for the actual frame packet size.
        // Most features will just use the default implementation. The default
        // implementation uses the visibility results to size the frame packet
        // by filtering each view relevant to the feature to only the entities
        // with render objects associated with the feature.
        feature.calculate_frame_packet_size(
            extract_context,
            visibility_results,
            &mut *render_object_instances,
            &mut frame_packet_metadata.frame_packet_size,
        );

        frame_packet_metadata
            .frame_packet_size
            .num_render_object_instances = render_object_instances.len();

        let num_view_packets = frame_packet_metadata
            .frame_packet_size
            .view_packet_sizes
            .len();

        frame_packet_metadata.is_relevant = num_view_packets > 0;
    }

    fn create_frame_packets<'extract>(
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        extract_context: &RenderJobExtractContext<'extract>,
    ) {
        let temporary_work = extract_context.allocation_context;

        features.iter().for_each(|feature| {
            profiling::scope!(feature.feature_debug_constants().feature_name);

            let frame_packet_metadata = temporary_work
                .frame_packet_metadata
                .get(feature.feature_index() as usize)
                .unwrap()
                .borrow();

            if frame_packet_metadata.is_relevant {
                let frame_packet =
                    feature.new_frame_packet(&frame_packet_metadata.frame_packet_size);

                *temporary_work
                    .frame_packets
                    .get(feature.feature_index() as usize)
                    .unwrap()
                    .borrow_mut() = Some(frame_packet);
            }
        });
    }

    pub fn populate_frame_packet(
        #[allow(unused_variables)] debug_constants: &'static RenderFeatureDebugConstants,
        feature_index: RenderFeatureIndex,
        is_relevant: impl Fn(&RenderViewVisibilityQuery) -> bool,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
        frame_packet: &mut Box<dyn RenderFeatureFramePacket>,
    ) {
        profiling::scope!(debug_constants.feature_name);

        for (view_frame_index, view_visibility_result) in visibility_results
            .iter()
            .filter(|result| is_relevant(result))
            .enumerate()
        {
            let view_frame_index = view_frame_index as ViewFrameIndex;
            if let Some(visible_render_objects) =
                view_visibility_result.render_object_instances_per_view(feature_index)
            {
                for render_object_instance in visible_render_objects {
                    let render_object_instance_id =
                        frame_packet.get_or_push_render_object_instance(*render_object_instance);

                    frame_packet.push_render_object_instance_per_view(
                        view_frame_index,
                        render_object_instance_id,
                        *render_object_instance,
                    );
                }
            }

            // TODO(dvd): One could imagine volumes having a bitfield mask and filtering on that.
            // for per_view_volume in view_visibility_result.per_view_volumes.iter() {
            //    let object_id = ObjectId::from(per_view_volume.id);
            //    frame_packet.push_volume(per_frame_view_index, object_id);
            // }
        }
    }

    pub fn create_extract_job<'extract>(
        feature: &Arc<dyn RenderFeaturePlugin>,
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
    ) -> Option<Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>> {
        profiling::scope!(feature.feature_debug_constants().feature_name);

        let temporary_work = extract_context.allocation_context;

        if !temporary_work
            .frame_packets
            .get(feature.feature_index() as usize)
            .map(|frame_packet| frame_packet.borrow().is_some())
            .unwrap_or(false)
        {
            return None;
        }

        let frame_packet_metadata = temporary_work
            .frame_packet_metadata
            .get(feature.feature_index() as usize)
            .unwrap()
            .borrow();

        let mut frame_packet = temporary_work
            .frame_packets
            .get(feature.feature_index() as usize)
            .unwrap()
            .borrow_mut()
            .take()
            .unwrap();

        // NOTE(dvd): Delegate to the feature for populating the frame packet.
        // Most features will just use the default implementation.
        feature.populate_frame_packet(
            extract_context,
            visibility_results,
            &frame_packet_metadata.frame_packet_size,
            &mut frame_packet,
        );

        Some(feature.new_extract_job(extract_context, frame_packet))
    }

    pub fn extract_render_object_instance_chunk<'extract>(
        extract_job: &Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>,
        chunk_index: usize,
        chunk_size: usize,
    ) {
        let num_render_object_instances = extract_job.num_render_object_instances();
        let start_id = chunk_index * chunk_size;
        let end_id = usize::min(start_id + chunk_size, num_render_object_instances);
        extract_job.extract_render_object_instance(start_id..end_id);
    }

    pub fn extract_render_object_instance_all<'extract>(
        extract_job: &Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>
    ) {
        Renderer::extract_render_object_instance_chunk(
            extract_job,
            0,
            extract_job.num_render_object_instances(),
        );
    }

    pub fn extract_render_object_instance_per_view_chunk<'extract>(
        extract_job: &Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>,
        view_packet: &dyn RenderFeatureViewPacket,
        chunk_index: usize,
        chunk_size: usize,
    ) {
        let num_render_object_instances = view_packet.num_render_object_instances();
        let start_id = chunk_index * chunk_size;
        let end_id = usize::min(start_id + chunk_size, num_render_object_instances);
        extract_job.extract_render_object_instance_per_view(view_packet, start_id..end_id);
    }

    pub fn extract_render_object_instance_per_view_all<'extract>(
        extract_job: &Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>,
        view_packet: &dyn RenderFeatureViewPacket,
    ) {
        Renderer::extract_render_object_instance_per_view_chunk(
            extract_job,
            view_packet,
            0,
            view_packet.num_render_object_instances(),
        );
    }

    fn take_frame_packet<'extract>(
        extract_job: &mut Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>
    ) -> Box<dyn RenderFeatureFramePacket> {
        let extract_job = Arc::get_mut(extract_job).unwrap();
        extract_job.take_frame_packet()
    }

    fn take_frame_packets<'extract>(
        mut finished_extract_jobs: Vec<Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>>
    ) -> Vec<Box<dyn RenderFeatureFramePacket>> {
        finished_extract_jobs
            .iter_mut()
            .map(|extract_job| Renderer::take_frame_packet(extract_job))
            .collect()
    }
}
