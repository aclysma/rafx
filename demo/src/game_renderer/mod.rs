use crate::imgui_support::{
    ImGuiFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener, Sdl2ImguiManager, ImguiManager,
};
use renderer::vulkan::{
    VkDevice, VkSwapchain, VkSurface, Window, VkTransferUpload, VkTransferUploadState, VkImage,
    VkDeviceContext, VkContextBuilder, VkCreateContextError, VkContext,
    VkSurfaceSwapchainLifetimeListener, MsaaLevel, MAX_FRAMES_IN_FLIGHT, VkBuffer, FrameInFlight,
};
use ash::prelude::VkResult;
use crate::renderpass::{VkDebugRenderPass, VkBloomRenderPassResources, VkOpaqueRenderPass};
use std::mem::{ManuallyDrop, swap};
use renderer::assets::image_utils::{decode_texture, enqueue_load_images};
use ash::vk;
use renderer::base::time::{ScopeTimer, TimeState};
use crossbeam_channel::{Sender, Receiver};
use std::ops::Deref;
use renderer::assets::vk_description::SwapchainSurfaceInfo;
use renderer::assets::assets::pipeline::{MaterialAsset, PipelineAsset, MaterialInstanceAsset};
use atelier_assets::loader::handle::Handle;
use renderer::assets::asset_resource::AssetResource;
use renderer::assets::assets::shader::ShaderAsset;
use renderer::assets::assets::image::ImageAsset;
use atelier_assets::core::asset_uuid;
use renderer::resources::resource_managers::{
    ResourceManager, DynDescriptorSet, DynMaterialInstance, ResourceArc, ImageViewResource,
    DynResourceAllocatorSet, PipelineSwapchainInfo,
};
use crate::assets::gltf::{
    MeshAsset, GltfMaterialAsset, GltfMaterialData, GltfMaterialDataShaderParam,
};
use renderer::assets::assets::buffer::BufferAsset;
use crate::renderpass::debug_renderpass::{DebugDraw3DResource, LineList3D};
use crate::renderpass::VkBloomExtractRenderPass;
use crate::renderpass::VkBloomBlurRenderPass;
use crate::renderpass::VkBloomCombineRenderPass;
use crate::features::sprite::{
    SpriteRenderNodeSet, SpriteRenderFeature, create_sprite_extract_job,
};
use renderer::visibility::{StaticVisibilityNodeSet, DynamicVisibilityNodeSet};
use renderer::nodes::{
    RenderRegistryBuilder, RenderPhaseMaskBuilder, RenderPhaseMask, RenderRegistry, RenderViewSet,
    AllRenderNodes, FramePacketBuilder, ExtractJobSet, PrepareJobSet, FramePacket, RenderView,
};
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use legion::prelude::*;
use crate::render_contexts::{
    RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContextFactory,
};
use crate::render_contexts::RenderJobWriteContext;
use renderer::vulkan::cleanup::{VkCombinedDropSink, VkResourceDropSinkChannel};
use crate::features::mesh::{MeshPerViewShaderParam, create_mesh_extract_job, MeshRenderNodeSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{Thread, JoinHandle};
use crossbeam_channel::internal::SelectHandle;

mod static_resources;
use static_resources::GameRendererStaticResources;

mod render_thread;
use render_thread::RenderThread;

mod swapchain_resources;
use swapchain_resources::SwapchainResources;

mod render_frame_job;
use render_frame_job::RenderFrameJob;

//TODO: Find a way to not expose this
mod swapchain_handling;
pub use swapchain_handling::SwapchainLifetimeListener;

pub struct GameRendererInner {
    imgui_event_listener: ImguiRenderEventListener,

    static_resources: GameRendererStaticResources,
    swapchain_resources: Option<SwapchainResources>,

    main_camera_render_phase_mask: RenderPhaseMask,

    previous_frame_result: Option<VkResult<()>>,

    render_thread: RenderThread,
}

#[derive(Clone)]
pub struct GameRenderer {
    inner: Arc<Mutex<GameRendererInner>>,
}

impl GameRenderer {
    pub fn new(
        window: &dyn Window,
        resources: &Resources,
    ) -> VkResult<Self> {
        let mut asset_resource_fetch = resources.get_mut::<AssetResource>().unwrap();
        let asset_resource = &mut *asset_resource_fetch;

        let mut resource_manager_fetch = resources.get_mut::<ResourceManager>().unwrap();
        let mut resource_manager = &mut *resource_manager_fetch;

        let mut render_registry_fetch = resources.get::<RenderRegistry>().unwrap();
        let render_registry = &*render_registry_fetch;

        let vk_context = resources.get_mut::<VkContext>().unwrap();
        let device_context = vk_context.device_context();

        let imgui_font_atlas = resources
            .get::<Sdl2ImguiManager>()
            .unwrap()
            .build_font_atlas();
        let imgui_event_listener = ImguiRenderEventListener::new(imgui_font_atlas);

        let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
            .add_render_phase::<DrawOpaqueRenderPhase>()
            .add_render_phase::<DrawTransparentRenderPhase>()
            .build();

        log::info!("all waits complete");
        let game_renderer_resources =
            GameRendererStaticResources::new(asset_resource, resource_manager)?;

        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        let debug_per_frame_layout =
            resource_manager.get_descriptor_set_info(&game_renderer_resources.debug_material, 0, 0);
        let debug_material_per_frame_data = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(
                &debug_per_frame_layout.descriptor_set_layout,
            )?;

        let render_thread = RenderThread::start();

        let mut renderer = GameRendererInner {
            imgui_event_listener,
            static_resources: game_renderer_resources,
            swapchain_resources: None,

            main_camera_render_phase_mask,

            render_thread,

            previous_frame_result: Some(Ok(())),
        };

        Ok(GameRenderer {
            inner: Arc::new(Mutex::new(renderer)),
        })
    }
}

impl GameRenderer {
    // This is externally exposed, it checks result of the previous frame (which implicitly also
    // waits for the previous frame to complete if it hasn't already)
    pub fn begin_render(
        &self,
        resources: &Resources,
        world: &World,
        window: &dyn Window,
    ) -> VkResult<()> {
        let t0 = std::time::Instant::now();
        // This lock will delay until the previous frame completes being submitted to GPU
        resources
            .get_mut::<VkSurface>()
            .unwrap()
            .wait_until_frame_not_in_flight();
        let t1 = std::time::Instant::now();
        log::info!(
            "[main] wait for previous frame present {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        // Here, we error check from the previous frame. This includes checking for errors that happened
        // during setup (i.e. before we finished building the frame job). So
        {
            let mut result = self.inner.lock().unwrap().previous_frame_result.take();
            if let Some(result) = result {
                if let Err(e) = result {
                    match e {
                        ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                            SwapchainLifetimeListener::rebuild_swapchain(resources, window, self)
                        }
                        ash::vk::Result::SUCCESS => Ok(()),
                        ash::vk::Result::SUBOPTIMAL_KHR => Ok(()),
                        //ash::vk::Result::TIMEOUT => Ok(()),
                        _ => {
                            log::warn!("Unexpected rendering error");
                            return Err(e);
                        }
                    }?;
                }
            }
        }

        // If we get an error before kicking off rendering, stash it for the next frame. We could
        // consider acting on it instead, but for now lets just have a single consistent codepath
        if let Err(e) = self.do_begin_render(resources, world, window) {
            log::warn!("Received error immediately from do_begin_render: {:?}", e);
            self.inner.lock().unwrap().previous_frame_result = Some(Err(e));
        }

        Ok(())
    }

    //TODO: In a failure, return the frame_in_flight and cancel the render. This will make
    // previous_frame_result unnecessary
    pub fn do_begin_render(
        &self,
        resources: &Resources,
        world: &World,
        window: &dyn Window,
    ) -> VkResult<()> {
        // Fetch the next swapchain image
        let frame_in_flight = {
            let mut surface = resources.get_mut::<VkSurface>().unwrap();
            let t0 = std::time::Instant::now();
            let result = surface.acquire_next_swapchain_image(window);
            let t1 = std::time::Instant::now();
            log::info!(
                "[main] wait for swapchain image took {} ms",
                (t1 - t0).as_secs_f32() * 1000.0
            );
            result?
        };

        // Get command buffers to submit
        Self::render(self, world, resources, window, frame_in_flight)
    }

    pub fn render(
        game_renderer: &GameRenderer,
        world: &World,
        resources: &Resources,
        window: &Window,
        frame_in_flight: FrameInFlight,
    ) -> VkResult<()> {
        let t0 = std::time::Instant::now();

        //
        // Fetch resources
        //
        let asset_resource_fetch = resources.get::<AssetResource>().unwrap();
        let asset_resource = &*asset_resource_fetch;

        let time_state_fetch = resources.get::<TimeState>().unwrap();
        let time_state = &*time_state_fetch;

        let static_visibility_node_set_fetch = resources.get::<StaticVisibilityNodeSet>().unwrap();
        let static_visibility_node_set = &*static_visibility_node_set_fetch;

        let dynamic_visibility_node_set_fetch =
            resources.get::<DynamicVisibilityNodeSet>().unwrap();
        let dynamic_visibility_node_set = &*dynamic_visibility_node_set_fetch;

        let mut debug_draw_3d_line_lists = resources
            .get_mut::<DebugDraw3DResource>()
            .unwrap()
            .take_line_lists();

        let render_registry = resources.get::<RenderRegistry>().unwrap().clone();
        let device_context = resources.get::<VkDeviceContext>().unwrap().clone();

        let mut resource_manager_fetch = resources.get_mut::<ResourceManager>().unwrap();
        let resource_manager = &mut *resource_manager_fetch;

        // Call this here - represents that the previous frame was completed
        resource_manager.on_frame_complete();

        let mut guard = game_renderer.inner.lock().unwrap();
        let main_camera_render_phase_mask = guard.main_camera_render_phase_mask.clone();
        let swapchain_resources = guard.swapchain_resources.as_mut().unwrap();
        let swapchain_surface_info = swapchain_resources.swapchain_surface_info.clone();

        //
        // View Management
        //
        let camera_rotate_speed = 1.0;
        let camera_distance_multiplier = 1.0;
        let loop_time = time_state.total_time().as_secs_f32();
        let eye = glam::Vec3::new(
            camera_distance_multiplier * 8.0 * f32::cos(camera_rotate_speed * loop_time / 2.0),
            camera_distance_multiplier * 8.0 * f32::sin(camera_rotate_speed * loop_time / 2.0),
            camera_distance_multiplier * 5.0,
        );

        let extents_width = 900;
        let extents_height = 600;
        let aspect_ratio = extents_width as f32 / extents_height as f32;

        let render_view_set = RenderViewSet::default();
        let (main_view, view_proj) = {
            let view = glam::Mat4::look_at_rh(
                eye,
                glam::Vec3::new(0.0, 0.0, 0.0),
                glam::Vec3::new(0.0, 0.0, 1.0),
            );
            let proj = glam::Mat4::perspective_rh_gl(
                std::f32::consts::FRAC_PI_4,
                aspect_ratio,
                0.01,
                20.0,
            );
            let proj = glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0)) * proj;
            let view_proj = proj * view;

            let main_view = render_view_set.create_view(
                eye,
                view,
                proj,
                main_camera_render_phase_mask,
                "main".to_string(),
            );

            (main_view, view_proj)
        };

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

        let sprite_render_nodes = resources.get::<SpriteRenderNodeSet>().unwrap();
        let mesh_render_nodes = resources.get::<MeshRenderNodeSet>().unwrap();
        let mut all_render_nodes = AllRenderNodes::new();
        all_render_nodes.add_render_nodes(&*sprite_render_nodes);
        all_render_nodes.add_render_nodes(&*mesh_render_nodes);

        let frame_packet_builder = FramePacketBuilder::new(&all_render_nodes);

        // After these jobs end, user calls functions to start jobs that extract data
        frame_packet_builder.add_view(
            &main_view,
            &[
                main_view_static_visibility_result,
                main_view_dynamic_visibility_result,
            ],
        );

        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        swapchain_resources
            .debug_material_per_frame_data
            .set_buffer_data(0, &view_proj);
        swapchain_resources
            .debug_material_per_frame_data
            .flush(&mut descriptor_set_allocator);
        descriptor_set_allocator.flush_changes();

        //
        // Update Resources and flush descriptor set changes
        //
        resource_manager.on_begin_frame();

        //
        // Extract Jobs
        //
        let frame_packet = frame_packet_builder.build();
        let extract_job_set = {
            let sprite_pipeline_info = resource_manager.get_pipeline_info(
                &guard.static_resources.sprite_material,
                &swapchain_surface_info,
                0,
            );

            let mesh_pipeline_info = resource_manager.get_pipeline_info(
                &guard.static_resources.mesh_material,
                &swapchain_surface_info,
                0,
            );

            let mut extract_job_set = ExtractJobSet::new();

            // Sprites
            extract_job_set.add_job(create_sprite_extract_job(
                device_context.clone(),
                resource_manager.create_descriptor_set_allocator(),
                sprite_pipeline_info,
                &guard.static_resources.sprite_material,
            ));

            // Meshes
            extract_job_set.add_job(create_mesh_extract_job(
                device_context.clone(),
                resource_manager.create_descriptor_set_allocator(),
                mesh_pipeline_info,
                &guard.static_resources.mesh_material,
            ));
            extract_job_set
        };

        let mut extract_context =
            RenderJobExtractContext::new(&world, &resources, resource_manager);
        let prepare_job_set =
            extract_job_set.extract(&mut extract_context, &frame_packet, &[&main_view]);

        let opaque_pipeline_info = resource_manager.get_pipeline_info(
            &guard.static_resources.sprite_material,
            &swapchain_surface_info,
            0,
        );

        let debug_pipeline_info = resource_manager.get_pipeline_info(
            &guard.static_resources.debug_material,
            &swapchain_surface_info,
            0,
        );

        let dyn_resource_allocator_set = resource_manager.create_dyn_resource_allocator_set();

        let t1 = std::time::Instant::now();
        log::info!(
            "[main] render extract took {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        let game_renderer = game_renderer.clone();

        let imgui_draw_data = resources
            .get::<Sdl2ImguiManager>()
            .unwrap()
            .copy_draw_data();

        let prepared_frame = RenderFrameJob {
            game_renderer,
            prepare_job_set,
            dyn_resource_allocator_set,
            frame_packet,
            main_view,
            render_registry: render_registry.clone(),
            device_context: device_context.clone(),
            opaque_pipeline_info,
            debug_pipeline_info,
            debug_draw_3d_line_lists,
            window_scale_factor: window.scale_factor(),
            imgui_draw_data,
            frame_in_flight,
        };

        guard.render_thread.render(prepared_frame);

        Ok(())
    }
}
