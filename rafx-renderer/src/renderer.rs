use rafx_assets::distill_impl::AssetResource;
use rafx_assets::{image_upload, AssetManagerRenderResource, GpuImageDataColorSpace};
use rafx_assets::{AssetManager, GpuImageData};
use rafx_framework::nodes::{
    ExtractJobSet, ExtractResources, FramePacketBuilder, RenderJobExtractContext,
    RenderNodeReservations, RenderViewSet,
};
use rafx_framework::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use rafx_framework::{DynResourceAllocatorSet, RenderResources};
use rafx_framework::{ImageViewResource, ResourceArc};
use std::sync::{Arc, Mutex};

use super::*;

use super::{RenderGraphGenerator, RendererPlugin, ViewportsResource};
use rafx_api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx_api::{
    RafxDeviceContext, RafxError, RafxPresentableFrame, RafxQueue, RafxResourceType, RafxResult,
    RafxSwapchainHelper,
};
use rafx_assets::image_upload::ImageUploadParams;

#[derive(Clone)]
pub struct InvalidResources {
    pub invalid_image: ResourceArc<ImageViewResource>,
    pub invalid_cube_map_image: ResourceArc<ImageViewResource>,
}

pub struct RendererInner {
    pub(super) render_graph_generator: Box<dyn RenderGraphGenerator>,
    pub(super) render_thread: RenderThread,
    pub(super) plugins: Arc<Vec<Box<dyn RendererPlugin>>>,
}

#[derive(Clone)]
pub struct Renderer {
    pub(super) inner: Arc<Mutex<RendererInner>>,
    pub(super) graphics_queue: RafxQueue,
    pub(super) transfer_queue: RafxQueue,
}

impl Renderer {
    pub fn new(
        extract_resources: ExtractResources,
        asset_resource: &mut AssetResource,
        asset_manager: &mut AssetManager,
        graphics_queue: &RafxQueue,
        transfer_queue: &RafxQueue,
        plugins: Vec<Box<dyn RendererPlugin>>,
        render_graph_generator: Box<dyn RenderGraphGenerator>,
    ) -> RafxResult<Self> {
        let plugins = Arc::new(plugins);
        let device_context = graphics_queue.device_context();

        let dyn_resource_allocator = asset_manager.create_dyn_resource_allocator_set();

        let mut upload = RafxTransferUpload::new(
            &device_context,
            asset_manager.transfer_queue(),
            asset_manager.graphics_queue(),
            16 * 1024 * 1024,
        )?;

        let invalid_image = Self::upload_image_data(
            &device_context,
            &mut upload,
            &dyn_resource_allocator,
            &GpuImageData::new_1x1_rgba8(255, 0, 255, 255, GpuImageDataColorSpace::Linear),
            ImageUploadParams::default(),
        )
        .map_err(|x| Into::<RafxError>::into(x))?;

        let invalid_cube_map_image = Self::upload_image_data(
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

        let invalid_resources = InvalidResources {
            invalid_image,
            invalid_cube_map_image,
        };

        let mut render_resources = RenderResources::default();
        for plugin in &*plugins {
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

        let render_thread = RenderThread::start(render_resources);

        let renderer = RendererInner {
            plugins,
            render_thread,
            render_graph_generator,
        };

        Ok(Renderer {
            inner: Arc::new(Mutex::new(renderer)),
            graphics_queue: graphics_queue.clone(),
            transfer_queue: transfer_queue.clone(),
        })
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
    ) -> RafxResult<()> {
        //
        // Block until the previous frame completes being submitted to GPU
        //
        let t0 = std::time::Instant::now();

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

        self.inner
            .lock()
            .unwrap()
            .render_thread
            .wait_for_render_finish(std::time::Duration::from_secs(30));

        let t1 = std::time::Instant::now();
        log::trace!(
            "[main] wait for previous frame present {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        Self::create_and_start_render_job(self, extract_resources, presentable_frame);

        Ok(())
    }

    fn create_and_start_render_job(
        renderer: &Renderer,
        extract_resources: &mut ExtractResources,
        presentable_frame: RafxPresentableFrame,
    ) {
        let result = Self::try_create_render_job(&renderer, extract_resources, &presentable_frame);

        let mut guard = renderer.inner.lock().unwrap();
        let renderer_inner = &mut *guard;
        match result {
            Ok(prepared_frame) => renderer_inner
                .render_thread
                .render(prepared_frame, presentable_frame),
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
    ) -> RafxResult<RenderFrameJob> {
        //
        // Fetch resources
        //
        let mut static_visibility_node_set_fetch =
            extract_resources.fetch_mut::<StaticVisibilityNodeSet>();
        let static_visibility_node_set = &mut *static_visibility_node_set_fetch;

        let mut dynamic_visibility_node_set_fetch =
            extract_resources.fetch_mut::<DynamicVisibilityNodeSet>();
        let dynamic_visibility_node_set = &mut *dynamic_visibility_node_set_fetch;

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
        let render_resources = &mut renderer_inner
            .render_thread
            .render_resources()
            .lock()
            .unwrap();

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

        let swapchain_surface_info = render_resources
            .fetch::<SwapchainResources>()
            .swapchain_surface_info
            .clone();

        //
        // Build the frame packet - this takes the views and visibility results and creates a
        // structure that's used during the extract/prepare/write phases
        //
        let frame_packet_builder = {
            let mut render_node_reservations = RenderNodeReservations::default();
            for plugin in &*renderer_inner.plugins {
                plugin
                    .add_render_node_reservations(&mut render_node_reservations, extract_resources);
            }

            FramePacketBuilder::new(&render_node_reservations)
        };

        let render_view_set = RenderViewSet::default();

        //
        // Determine Camera Location
        //
        let viewports_resource = extract_resources.fetch::<ViewportsResource>();
        let view_meta = viewports_resource
            .main_view_meta
            .clone()
            .unwrap_or_default();
        let main_window_size = viewports_resource.main_window_size;

        let main_view = render_view_set.create_view(
            view_meta.eye_position,
            view_meta.view,
            view_meta.proj,
            (main_window_size.width, main_window_size.height),
            view_meta.depth_range,
            view_meta.render_phase_mask,
            view_meta.debug_name,
        );

        //
        // Visibility
        //
        let main_view_static_visibility_result =
            static_visibility_node_set.calculate_static_visibility(&main_view);
        let main_view_dynamic_visibility_result =
            dynamic_visibility_node_set.calculate_dynamic_visibility(&main_view);

        log::trace!(
            "main view static node count: {}",
            main_view_static_visibility_result.handles.len()
        );

        log::trace!(
            "main view dynamic node count: {}",
            main_view_dynamic_visibility_result.handles.len()
        );

        // After these jobs end, user calls functions to start jobs that extract data
        frame_packet_builder.add_view(
            &main_view,
            &[
                main_view_static_visibility_result,
                main_view_dynamic_visibility_result,
            ],
        );

        let mut render_views = Vec::default();
        render_views.push(main_view.clone());

        for plugin in &*renderer_inner.plugins {
            plugin.add_render_views(
                extract_resources,
                render_resources,
                &render_view_set,
                &frame_packet_builder,
                static_visibility_node_set,
                dynamic_visibility_node_set,
                &mut render_views,
            );
        }

        let frame_packet = frame_packet_builder.build();

        //
        // Update Resources and flush descriptor set changes
        //
        asset_manager.on_begin_frame()?;

        //
        // Extract Jobs
        //
        let mut extract_jobs = Vec::default();
        for plugin in &*renderer_inner.plugins {
            plugin.add_extract_jobs(&extract_resources, render_resources, &mut extract_jobs);
        }

        let extract_job_set = ExtractJobSet::new(extract_jobs);

        //
        //
        //
        render_resources.insert(swapchain_surface_info.clone());
        unsafe {
            render_resources.insert(AssetManagerRenderResource::new(asset_manager));
        }

        let prepare_job_set = {
            profiling::scope!("renderer extract");

            let extract_context =
                RenderJobExtractContext::new(&extract_resources, &render_resources);

            extract_job_set.extract(&extract_context, &frame_packet, &render_views)
        };

        render_resources.remove::<AssetManagerRenderResource>();

        //TODO: This is now possible to run on the render thread
        let prepared_render_graph = renderer_inner
            .render_graph_generator
            .generate_render_graph(
                asset_manager,
                swapchain_image,
                main_view.clone(),
                extract_resources,
                render_resources,
            )?;

        let renderer = renderer.clone();
        let graphics_queue = renderer.graphics_queue.clone();
        let plugins = renderer_inner.plugins.clone();

        let prepared_frame = RenderFrameJob {
            renderer,
            prepare_job_set,
            prepared_render_graph,
            resource_context,
            frame_packet,
            render_registry,
            device_context,
            graphics_queue,
            plugins,
            render_views,
        };

        Ok(prepared_frame)
    }
}
