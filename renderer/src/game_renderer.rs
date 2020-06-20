use renderer_ext::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener, Sdl2ImguiManager};
use renderer_shell_vulkan::{VkDevice, VkSwapchain, VkSurface, Window, VkTransferUpload, VkTransferUploadState, VkImage, VkDeviceContext, VkContextBuilder, VkCreateContextError, VkContext, VkSurfaceSwapchainLifetimeListener, MsaaLevel, MAX_FRAMES_IN_FLIGHT, VkBuffer, FrameInFlight};
use ash::prelude::VkResult;
use renderer_ext::renderpass::{VkDebugRenderPass, VkBloomRenderPassResources, VkOpaqueRenderPass};
use std::mem::{ManuallyDrop, swap};
use renderer_ext::image_utils::{decode_texture, enqueue_load_images};
use ash::vk;
use renderer_ext::time::{ScopeTimer, TimeState};
use crossbeam_channel::Sender;
use std::ops::Deref;
use renderer_ext::pipeline_description::SwapchainSurfaceInfo;
use renderer_ext::pipeline::pipeline::{MaterialAsset, PipelineAsset, MaterialInstanceAsset};
use atelier_assets::loader::handle::Handle;
use renderer_ext::asset_resource::AssetResource;
use renderer_ext::pipeline::shader::ShaderAsset;
use renderer_ext::pipeline::image::ImageAsset;
use atelier_assets::core::asset_uuid;
use atelier_assets::loader::LoadStatus;
use atelier_assets::loader::handle::AssetHandle;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;
use renderer_ext::resource_managers::{ResourceManager, DynDescriptorSet, DynMaterialInstance, MeshInfo, ResourceArc, ImageViewResource, DynResourceAllocatorSet, PipelineSwapchainInfo};
use renderer_ext::pipeline::gltf::{MeshAsset, GltfMaterialAsset, GltfMaterialData, GltfMaterialDataShaderParam};
use renderer_ext::pipeline::buffer::BufferAsset;
use renderer_ext::renderpass::debug_renderpass::{DebugDraw3DResource, LineList3D};
use renderer_ext::renderpass::VkBloomExtractRenderPass;
use renderer_ext::renderpass::VkBloomBlurRenderPass;
use renderer_ext::renderpass::VkBloomCombineRenderPass;
use renderer_ext::features::sprite::{SpriteRenderNodeSet, SpriteRenderFeature, create_sprite_extract_job};
use renderer_base::visibility::{StaticVisibilityNodeSet, DynamicVisibilityNodeSet};
use renderer_base::{RenderRegistryBuilder, RenderPhaseMaskBuilder, RenderPhaseMask, RenderRegistry, RenderViewSet, AllRenderNodes, FramePacketBuilder, ExtractJobSet, PrepareJobSet, FramePacket, RenderView};
use renderer_ext::phases::draw_opaque::DrawOpaqueRenderPhase;
use renderer_ext::phases::draw_transparent::DrawTransparentRenderPhase;
use legion::prelude::*;
use renderer_ext::{RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContextFactory};
use renderer_ext::RenderJobWriteContext;
use renderer_shell_vulkan::cleanup::{VkCombinedDropSink, VkResourceDropSinkChannel};
use renderer_ext::features::mesh::{MeshPerViewShaderParam, create_mesh_extract_job, MeshRenderNodeSet};
use std::sync::{Arc, Mutex, MutexGuard};

fn begin_load_asset<T>(
    asset_uuid: AssetUuid,
    asset_resource: &AssetResource,
) -> atelier_assets::loader::handle::Handle<T> {
    use atelier_assets::loader::Loader;
    let load_handle = asset_resource.loader().add_ref(asset_uuid);
    atelier_assets::loader::handle::Handle::<T>::new(asset_resource.tx().clone(), load_handle)
}

fn wait_for_asset_to_load<T>(
    device_context: &VkDeviceContext,
    asset_handle: &atelier_assets::loader::handle::Handle<T>,
    asset_resource: &mut AssetResource,
    resource_manager: &mut ResourceManager,
    asset_name: &str
) {
    loop {
        asset_resource.update();
        resource_manager.update_resources();
        match asset_handle.load_status(asset_resource.loader()) {
            LoadStatus::NotRequested => {
                unreachable!();
            }
            LoadStatus::Loading => {
                log::info!("blocked waiting for asset to load {} {:?}", asset_name, asset_handle);
                std::thread::sleep(std::time::Duration::from_millis(10));
                // keep waiting
            }
            LoadStatus::Loaded => {
                break;
            }
            LoadStatus::Unloading => unreachable!(),
            LoadStatus::DoesNotExist => {
                println!("Essential asset not found");
            }
            LoadStatus::Error(err) => {
                println!("Error loading essential asset {:?}", err);
            }
        }
    }
}

pub struct GameRendererInner {
    imgui_event_listener: ImguiRenderEventListener,

    sprite_material: Handle<MaterialAsset>,

    debug_material: Handle<MaterialAsset>,
    debug_material_per_frame_data: DynDescriptorSet,

    // binding 0, contains info about lights
    mesh_material: Handle<MaterialAsset>,

    bloom_resources: Option<VkBloomRenderPassResources>,

    bloom_extract_material: Handle<MaterialAsset>,
    bloom_extract_material_dyn_set: Option<DynDescriptorSet>,

    bloom_blur_material: Handle<MaterialAsset>,

    bloom_combine_material: Handle<MaterialAsset>,
    bloom_combine_material_dyn_set: Option<DynDescriptorSet>,

    main_camera_render_phase_mask: RenderPhaseMask,

    opaque_renderpass: Option<VkOpaqueRenderPass>,
    debug_renderpass: Option<VkDebugRenderPass>,
    bloom_extract_renderpass: Option<VkBloomExtractRenderPass>,
    bloom_blur_renderpass: Option<VkBloomBlurRenderPass>,
    bloom_combine_renderpass: Option<VkBloomCombineRenderPass>,
    swapchain_surface_info: Option<SwapchainSurfaceInfo>,

    previous_frame_result: Option<VkResult<()>>
}

pub struct GameRenderer {
    inner: Arc<Mutex<GameRendererInner>>
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

        let imgui_font_atlas = resources.get::<Sdl2ImguiManager>().unwrap().build_font_atlas();
        let imgui_event_listener = ImguiRenderEventListener::new(imgui_font_atlas);

        let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
            .add_render_phase::<DrawOpaqueRenderPhase>()
            .add_render_phase::<DrawTransparentRenderPhase>()
            .build();

        //
        // Sprite resources
        //
        let sprite_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("f8c4897e-7c1d-4736-93b7-f2deda158ec7"),
            &asset_resource,
        );

        //
        // Debug resources
        //
        let debug_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("11d3b144-f564-42c9-b31f-82c8a938bf85"),
            &asset_resource,
        );

        //
        // Bloom extract resources
        //
        let bloom_extract_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("822c8e08-2720-4002-81da-fd9c4d61abdd"),
            &asset_resource,
        );

        //
        // Bloom blur resources
        //
        let bloom_blur_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("22aae4c1-fd0f-414a-9de1-7f68bdf1bfb1"),
            &asset_resource,
        );

        //
        // Bloom combine resources
        //
        let bloom_combine_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("256e6a2d-669b-426b-900d-3bcc4249a063"),
            &asset_resource,
        );

        //
        // Mesh resources
        //
        let mesh_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("267e0388-2611-441c-9c78-2d39d1bd3cf1"),
            &asset_resource,
        );

        // cobblestone gltf
        // ORIGINAL
        // let mesh_material_instance = begin_load_asset::<MaterialInstanceAsset>(
        //     asset_uuid!("dc740f08-8e06-4341-806e-a01ae37df314"),
        //     &asset_resource,
        // );
        // let mesh = begin_load_asset::<MeshAsset>(
        //     asset_uuid!("ef79835d-25de-4df0-99e8-1968d2826d05"),
        //     &asset_resource,
        // );

        // cobblestone glb
        // UNWRAPPED ALL SIDES EQUAL
        // let mesh_material_instance = begin_load_asset::<MaterialInstanceAsset>(
        //     asset_uuid!("0dc01376-ebfe-4da4-9b3c-05eaf7c848a1"),
        //     &asset_resource,
        // );
        // let mesh = begin_load_asset::<MeshAsset>(
        //     asset_uuid!("ffc9b240-0a17-4ff4-bb7d-72d13cc6e261"),
        //     &asset_resource,
        // );

        // cobblestone glb
        // FLAT NORMALS
        // let mesh_material_instance = begin_load_asset::<MaterialInstanceAsset>(
        //     asset_uuid!("3ef917f7-9aeb-427d-af6b-9914c6bf9d93"),
        //     &asset_resource,
        // );
        // let mesh = begin_load_asset::<MeshAsset>(
        //     asset_uuid!("e386d9bf-5dcf-4e5e-bca7-43f48a32b8c8"),
        //     &asset_resource,
        // );

        // light
        // let light_mesh = begin_load_asset::<MeshAsset>(
        //     asset_uuid!("eb44a445-2670-42ba-9faa-5fb4ec4a2242"),
        //     &asset_resource,
        // );
        //
        // // axis z-up (blender format)
        // let axis_mesh = begin_load_asset::<MeshAsset>(
        //     asset_uuid!("21ba465c-57f7-47de-9dd5-6b22060eaec3"),
        //     &asset_resource,
        // );

        // axis y-up (gltf standard)
        // let axis_mesh = begin_load_asset::<MeshAsset>(
        //     asset_uuid!("2365fe99-b618-4299-8bfc-0c2482bec5cd"),
        //     &asset_resource,
        // );

        wait_for_asset_to_load(
            device_context,
            &sprite_material,
            asset_resource,
            &mut resource_manager,
            "sprite_material"
        );

        wait_for_asset_to_load(
            device_context,
            &debug_material,
            asset_resource,
            &mut resource_manager,
            "debub material"
        );

        wait_for_asset_to_load(
            device_context,
            &bloom_extract_material,
            asset_resource,
            &mut resource_manager,
            "bloom extract material"
        );

        wait_for_asset_to_load(
            device_context,
            &bloom_blur_material,
            asset_resource,
            &mut resource_manager,
            "bloom blur material"
        );

        wait_for_asset_to_load(
            device_context,
            &bloom_combine_material,
            asset_resource,
            &mut resource_manager,
            "bloom combine material"
        );

        wait_for_asset_to_load(
            device_context,
            &mesh_material,
            asset_resource,
            &mut resource_manager,
            "mesh material"
        );

        log::info!("all waits complete");

        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        let debug_per_frame_layout = resource_manager.get_descriptor_set_info(&debug_material, 0, 0);
        let debug_material_per_frame_data = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&debug_per_frame_layout.descriptor_set_layout)?;

        let mut renderer = GameRendererInner {
            imgui_event_listener,

            sprite_material,

            debug_material,
            debug_material_per_frame_data,

            mesh_material,

            bloom_resources: None,

            bloom_extract_material,
            bloom_extract_material_dyn_set: None,

            bloom_blur_material,

            bloom_combine_material,
            bloom_combine_material_dyn_set: None,

            main_camera_render_phase_mask,

            swapchain_surface_info: None,
            opaque_renderpass: None,
            debug_renderpass: None,
            bloom_extract_renderpass: None,
            bloom_blur_renderpass: None,
            bloom_combine_renderpass: None,

            previous_frame_result: Some(Ok(()))
        };

        Ok(GameRenderer {
            inner: Arc::new(Mutex::new(renderer))
        })
    }
}


pub struct SwapchainLifetimeListener<'a> {
    pub resources: &'a Resources,
    pub resource_manager: &'a mut ResourceManager,
    pub render_registry: &'a RenderRegistry,
    pub game_renderer: &'a GameRenderer
}

impl<'a> VkSurfaceSwapchainLifetimeListener for SwapchainLifetimeListener<'a> {
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()> {
        let mut guard = self.game_renderer.inner.lock().unwrap();
        let mut game_renderer = &mut *guard;
        let mut resource_manager = &mut self.resource_manager;

        log::debug!("game renderer swapchain_created called");
        game_renderer.imgui_event_listener
            .swapchain_created(device_context, swapchain)?;

        let swapchain_surface_info = SwapchainSurfaceInfo {
            extents: swapchain.swapchain_info.extents,
            msaa_level: swapchain.swapchain_info.msaa_level,
            surface_format: swapchain.swapchain_info.surface_format,
            color_format: swapchain.color_format,
            depth_format: swapchain.depth_format,
        };

        game_renderer.swapchain_surface_info = Some(swapchain_surface_info.clone());
        resource_manager.add_swapchain(&swapchain_surface_info);

        log::trace!("Create VkOpaqueRenderPass");
        //TODO: We probably want to move to just using a pipeline here and not a specific material
        let opaque_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.sprite_material,
            &swapchain_surface_info,
            0,
        );

        game_renderer.opaque_renderpass = Some(VkOpaqueRenderPass::new(
            device_context,
            swapchain,
            opaque_pipeline_info,
        )?);

        log::trace!("Create VkDebugRenderPass");
        let debug_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.debug_material,
            &swapchain_surface_info,
            0,
        );

        game_renderer.debug_renderpass = Some(VkDebugRenderPass::new(
            device_context,
            swapchain,
            debug_pipeline_info,
        )?);

        log::trace!("Create VkBloomExtractRenderPass");

        game_renderer.bloom_resources = Some(VkBloomRenderPassResources::new(
            device_context,
            swapchain,
            resource_manager,
            game_renderer.bloom_blur_material.clone()
        )?);

        let bloom_extract_layout = resource_manager.get_descriptor_set_info(
            &game_renderer.bloom_extract_material,
            0,
            0
        );

        let bloom_extract_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.bloom_extract_material,
            &swapchain_surface_info,
            0,
        );

        game_renderer.bloom_extract_renderpass = Some(VkBloomExtractRenderPass::new(
            device_context,
            swapchain,
            bloom_extract_pipeline_info,
            game_renderer.bloom_resources.as_ref().unwrap()
        )?);

        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        let mut bloom_extract_material_dyn_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&bloom_extract_layout.descriptor_set_layout)?;
        bloom_extract_material_dyn_set.set_image_raw(0, swapchain.color_attachment.resolved_image_view());
        bloom_extract_material_dyn_set.flush(&mut descriptor_set_allocator);
        game_renderer.bloom_extract_material_dyn_set = Some(bloom_extract_material_dyn_set);

        log::trace!("Create VkBloomBlurRenderPass");

        let bloom_blur_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.bloom_blur_material,
            &swapchain_surface_info,
            0,
        );

        game_renderer.bloom_blur_renderpass = Some(VkBloomBlurRenderPass::new(
            device_context,
            swapchain,
            bloom_blur_pipeline_info,
            resource_manager,
            game_renderer.bloom_resources.as_ref().unwrap()
        )?);

        log::trace!("Create VkBloomCombineRenderPass");

        let bloom_combine_layout = resource_manager.get_descriptor_set_info(
            &game_renderer.bloom_combine_material,
            0,
            0
        );

        let bloom_combine_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.bloom_combine_material,
            &swapchain_surface_info,
            0,
        );

        game_renderer.bloom_combine_renderpass = Some(VkBloomCombineRenderPass::new(
            device_context,
            swapchain,
            bloom_combine_pipeline_info,
            game_renderer.bloom_resources.as_ref().unwrap()
        )?);

        let mut bloom_combine_material_dyn_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&bloom_combine_layout.descriptor_set_layout)?;
        bloom_combine_material_dyn_set.set_image_raw(0, game_renderer.bloom_resources.as_ref().unwrap().color_image_view);
        bloom_combine_material_dyn_set.set_image_raw(1, game_renderer.bloom_resources.as_ref().unwrap().bloom_image_views[0]);
        bloom_combine_material_dyn_set.flush(&mut descriptor_set_allocator);
        game_renderer.bloom_combine_material_dyn_set = Some(bloom_combine_material_dyn_set);

        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(())
    }

    fn swapchain_destroyed(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) {
        let mut guard = self.game_renderer.inner.lock().unwrap();
        let mut game_renderer = &mut *guard;

        log::debug!("game renderer swapchain destroyed");

        let swapchain_surface_info = SwapchainSurfaceInfo {
            extents: swapchain.swapchain_info.extents,
            msaa_level: swapchain.swapchain_info.msaa_level,
            surface_format: swapchain.swapchain_info.surface_format,
            color_format: swapchain.color_format,
            depth_format: swapchain.depth_format,

        };

        self.resource_manager
            .remove_swapchain(&swapchain_surface_info);
        game_renderer.imgui_event_listener
            .swapchain_destroyed(device_context, swapchain);

        game_renderer.swapchain_surface_info = None;
    }
}

impl GameRenderer {
    // This is externally exposed, it checks result of the previous frame (which implicitly also
    // waits for the previous frame to complete if it hasn't already
    pub fn begin_render(
        &self,
        resources: &Resources,
        world: &World,
        window: &dyn Window
    ) -> VkResult<()> {
        {
            // This lock will delay until the previous frame completes being submitted to GPU
            let mut result = self.inner.lock().unwrap().previous_frame_result.take();
            if let Some(result) = result {
                if let Err(e) = result {
                    match e {
                        ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                            let mut surface = resources.get_mut::<VkSurface>().unwrap();
                            let mut resource_manager = resources.get_mut::<ResourceManager>().unwrap();
                            let render_registry = resources.get::<RenderRegistry>().unwrap();

                            let mut lifetime_listener = SwapchainLifetimeListener {
                                resources: &resources,
                                resource_manager: &mut *resource_manager,
                                render_registry: &*render_registry,
                                game_renderer: &*self,
                            };

                            surface.rebuild_swapchain(window, &mut Some(&mut lifetime_listener))
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

    pub fn do_begin_render(
        &self,
        resources: &Resources,
        world: &World,
        window: &dyn Window
    ) -> VkResult<()> {
        // Fetch the next swapchain image
        let frame_in_flight = {
            let mut surface = resources.get_mut::<VkSurface>().unwrap();
            surface.acquire_next_swapchain_image(window)?
        };

        // Get command buffers to submit
        self.render(
            world,
            resources,
            window,
            frame_in_flight
        )
    }


    pub fn render(
        &self,
        world: &World,
        resources: &Resources,
        window: &Window,
        frame_in_flight: FrameInFlight
    ) -> VkResult<()> {
        //
        // Fetch resources
        //
        let asset_resource_fetch = resources.get::<AssetResource>().unwrap();
        let asset_resource = &* asset_resource_fetch;

        let time_state_fetch = resources.get::<TimeState>().unwrap();
        let time_state = &* time_state_fetch;

        let static_visibility_node_set_fetch = resources.get::<StaticVisibilityNodeSet>().unwrap();
        let static_visibility_node_set = &* static_visibility_node_set_fetch;

        let dynamic_visibility_node_set_fetch = resources.get::<DynamicVisibilityNodeSet>().unwrap();
        let dynamic_visibility_node_set = &* dynamic_visibility_node_set_fetch;

        let mut debug_draw_3d_line_lists = resources.get_mut::<DebugDraw3DResource>().unwrap().take_line_lists();

        let render_registry = resources.get::<RenderRegistry>().unwrap().clone();
        let device_context = resources.get::<VkDeviceContext>().unwrap().clone();

        let mut resource_manager_fetch = resources.get_mut::<ResourceManager>().unwrap();
        let resource_manager = &mut *resource_manager_fetch;

        // Call this here - represents that the previous frame was completed
        resource_manager.on_frame_complete();

        let mut guard = self.inner.lock().unwrap();


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
            let view = glam::Mat4::look_at_rh(eye, glam::Vec3::new(0.0, 0.0, 0.0), glam::Vec3::new(0.0, 0.0, 1.0));
            let proj = glam::Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, aspect_ratio, 0.01, 20.0);
            let proj = glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0)) * proj;
            let view_proj = proj * view;

            let main_view = render_view_set.create_view(
                eye,
                view,
                proj,
                guard.main_camera_render_phase_mask,
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
        guard.debug_material_per_frame_data.set_buffer_data(0, &view_proj);
        guard.debug_material_per_frame_data.flush(&mut descriptor_set_allocator);
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
                &guard.sprite_material,
                guard.swapchain_surface_info.as_ref().unwrap(),
                0,
            );

            let mesh_pipeline_info = resource_manager.get_pipeline_info(
                &guard.mesh_material,
                guard.swapchain_surface_info.as_ref().unwrap(),
                0,
            );

            let mut extract_job_set = ExtractJobSet::new();

            // Sprites
            extract_job_set.add_job(create_sprite_extract_job(
                device_context.clone(),
                resource_manager.create_descriptor_set_allocator(),
                sprite_pipeline_info,
                &guard.sprite_material,
            ));

            // Meshes
            extract_job_set.add_job(create_mesh_extract_job(
                device_context.clone(),
                resource_manager.create_descriptor_set_allocator(),
                mesh_pipeline_info,
                &guard.mesh_material,
            ));
            extract_job_set
        };

        let mut extract_context = RenderJobExtractContext::new(&world, &resources, resource_manager);
        let prepare_job_set = extract_job_set.extract(
            &mut extract_context,
            &frame_packet,
            &[&main_view]
        );

        let opaque_pipeline_info = resource_manager.get_pipeline_info(
            &guard.sprite_material,
            guard.swapchain_surface_info.as_ref().unwrap(),
            0,
        );

        let debug_pipeline_info = resource_manager.get_pipeline_info(
            &guard.debug_material,
            guard.swapchain_surface_info.as_ref().unwrap(),
            0,
        );

        let dyn_resource_allocator_set = resource_manager.create_dyn_resource_allocator_set();
        let command_buffers = self.render_thread(
            guard,
            prepare_job_set,
            dyn_resource_allocator_set,
            frame_packet,
            main_view,
            render_registry.clone(),
            device_context.clone(),
            opaque_pipeline_info,
            debug_pipeline_info,
            debug_draw_3d_line_lists,
            window.scale_factor(),
            &frame_in_flight
        )?;

        // Submit them - temporary, eventually a job we kick off will do this
        {
            // TODO: Figure out a way to not require fetching the surface
            let mut surface = resources.get_mut::<VkSurface>().unwrap();
            surface.present(frame_in_flight, command_buffers.as_slice())?;
        }

        Ok(())
    }

    fn render_thread(
        &self,
        mut guard: MutexGuard<GameRendererInner>,
        prepare_job_set: PrepareJobSet<RenderJobPrepareContext, RenderJobWriteContext>,
        dyn_resource_allocator_set: DynResourceAllocatorSet,
        frame_packet: FramePacket,
        main_view: RenderView,
        render_registry: RenderRegistry,
        device_context: VkDeviceContext,
        opaque_pipeline_info: PipelineSwapchainInfo,
        debug_pipeline_info: PipelineSwapchainInfo,
        debug_draw_3d_line_lists: Vec<LineList3D>,
        window_scale_factor: f64,
        frame_in_flight: &FrameInFlight,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        //let mut guard = self.inner.lock().unwrap();
        let mut command_buffers = vec![];

        let present_index = frame_in_flight.present_index() as usize;

        //
        // Prepare Jobs - everything beyond this point could be done in parallel with the main thread
        //
        let prepare_context = RenderJobPrepareContext::new(dyn_resource_allocator_set);
        let prepared_render_data = prepare_job_set.prepare(
            &prepare_context,
            &frame_packet,
            &[&main_view],
            &render_registry,
        );

        //
        // Write Jobs - called from within renderpasses for now
        //
        let mut write_context_factory = RenderJobWriteContextFactory::new(
            device_context.clone(),
            prepare_context.dyn_resource_lookups
        );

        //
        // Opaque renderpass
        //
        if let Some(opaque_renderpass) = &mut guard.opaque_renderpass {
            log::trace!("opaque_renderpass update");
            opaque_renderpass.update(
                &opaque_pipeline_info,
                present_index,
                &*prepared_render_data,
                &main_view,
                &write_context_factory
            )?;
            command_buffers.push(opaque_renderpass.command_buffers[present_index].clone());
        }

        //
        // Debug Renderpass
        //
        let descriptor_set_per_pass = guard.debug_material_per_frame_data.descriptor_set().get();
        if let Some(debug_renderpass) = &mut guard.debug_renderpass {
            log::trace!("debug_renderpass update");

            debug_renderpass.update(
                present_index,
                descriptor_set_per_pass,
                debug_draw_3d_line_lists,
            )?;
            command_buffers.push(debug_renderpass.command_buffers[present_index].clone());
        }

        //
        // bloom extract
        //
        let descriptor_set_per_pass = guard.bloom_extract_material_dyn_set.as_ref().unwrap().descriptor_set().get();
        if let Some(bloom_extract_renderpass) = &mut guard.bloom_extract_renderpass {
            log::trace!("bloom_extract_renderpass update");

            bloom_extract_renderpass.update(
                present_index,
                descriptor_set_per_pass
            )?;
            command_buffers.push(bloom_extract_renderpass.command_buffers[present_index].clone());
        }

        //
        // bloom blur
        //
        if let Some(bloom_blur_renderpass) = &mut guard.bloom_blur_renderpass {
            log::trace!("bloom_blur_renderpass update");
            command_buffers.push(bloom_blur_renderpass.command_buffers[0].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[1].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[0].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[1].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[0].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[1].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[0].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[1].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[0].clone());
            command_buffers.push(bloom_blur_renderpass.command_buffers[1].clone());
        }

        //
        // bloom combine
        //
        let descriptor_set_per_pass = guard.bloom_combine_material_dyn_set.as_ref().unwrap().descriptor_set().get();
        if let Some(bloom_combine_renderpass) = &mut guard.bloom_combine_renderpass {
            log::trace!("bloom_combine_renderpass update");

            bloom_combine_renderpass.update(
                present_index,
                descriptor_set_per_pass
            )?;
            command_buffers.push(bloom_combine_renderpass.command_buffers[present_index].clone());
        }

        //
        // imgui
        //
        {
            log::trace!("imgui_event_listener update");
            let mut commands =
                guard.imgui_event_listener
                    .render(&device_context, present_index, window_scale_factor)?;
            command_buffers.append(&mut commands);
        }

        VkResult::Ok(command_buffers)
    }
}
