use crate::asset_resource::AssetResource;
use crate::features::debug3d::create_debug3d_extract_job;
use crate::features::mesh::{create_mesh_extract_job, MeshRenderNodeSet};
use crate::features::sprite::{create_sprite_extract_job, SpriteRenderNodeSet};
use crate::imgui_support::Sdl2ImguiManager;
use crate::phases::TransparentRenderPhase;
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase, UiRenderPhase};
use crate::render_contexts::RenderJobExtractContext;
use crate::time::TimeState;
use ash::prelude::VkResult;
use legion::*;
use renderer::assets::AssetManager;
use renderer::nodes::{
    AllRenderNodes, ExtractJobSet, FramePacketBuilder, RenderPhaseMask, RenderPhaseMaskBuilder,
    RenderRegistry, RenderViewSet,
};
use renderer::resources::vk_description as dsc;
use renderer::resources::{ImageViewResource, ResourceArc};
use renderer::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use renderer::vulkan::{FrameInFlight, VkContext, VkDeviceContext, VkSurface, Window};
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

mod static_resources;
use static_resources::GameRendererStaticResources;

mod render_thread;
use render_thread::RenderThread;

mod swapchain_resources;
use swapchain_resources::SwapchainResources;

mod render_frame_job;
use render_frame_job::RenderFrameJob;

mod render_graph;

//TODO: Find a way to not expose this
mod swapchain_handling;
use crate::components::DirectionalLightComponent;
use crate::features::imgui::create_imgui_extract_job;
pub use swapchain_handling::SwapchainLifetimeListener;

pub struct GameRendererInner {
    imgui_font_atlas_image_view: ResourceArc<ImageViewResource>,

    // Everything that is loaded all the time
    static_resources: GameRendererStaticResources,

    // Everything that requires being created after the swapchain inits
    swapchain_resources: Option<SwapchainResources>,

    main_camera_render_phase_mask: RenderPhaseMask,

    render_thread: RenderThread,
}

#[derive(Clone)]
pub struct GameRenderer {
    inner: Arc<Mutex<GameRendererInner>>,
}

impl GameRenderer {
    pub fn new(
        _window: &dyn Window,
        resources: &Resources,
    ) -> VkResult<Self> {
        let mut asset_resource_fetch = resources.get_mut::<AssetResource>().unwrap();
        let asset_resource = &mut *asset_resource_fetch;

        let mut asset_manager_fetch = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_manager = &mut *asset_manager_fetch;

        let vk_context = resources.get_mut::<VkContext>().unwrap();
        let device_context = vk_context.device_context();

        let imgui_font_atlas_image_view = GameRenderer::create_font_atlas_image_view(
            &device_context,
            &mut asset_manager,
            resources,
        )?;

        let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
            .add_render_phase::<OpaqueRenderPhase>()
            .add_render_phase::<ShadowMapRenderPhase>()
            .add_render_phase::<TransparentRenderPhase>()
            .add_render_phase::<UiRenderPhase>()
            .build();

        log::info!("all waits complete");
        let game_renderer_resources =
            GameRendererStaticResources::new(asset_resource, asset_manager)?;

        let render_thread = RenderThread::start();

        let renderer = GameRendererInner {
            imgui_font_atlas_image_view,
            static_resources: game_renderer_resources,
            swapchain_resources: None,

            main_camera_render_phase_mask,

            render_thread,
        };

        Ok(GameRenderer {
            inner: Arc::new(Mutex::new(renderer)),
        })
    }

    fn create_font_atlas_image_view(
        device_context: &VkDeviceContext,
        asset_manager: &mut AssetManager,
        resources: &Resources,
    ) -> VkResult<ResourceArc<ImageViewResource>> {
        //TODO: Simplify this setup code for the imgui font atlas
        let imgui_font_atlas = resources
            .get::<Sdl2ImguiManager>()
            .unwrap()
            .build_font_atlas();

        let imgui_font_atlas = renderer::assets::image_utils::DecodedTexture {
            width: imgui_font_atlas.width,
            height: imgui_font_atlas.height,
            data: imgui_font_atlas.data,
            color_space: renderer::assets::image_utils::ColorSpace::Linear,
            mips: renderer::assets::image_utils::default_mip_settings_for_image(
                imgui_font_atlas.width,
                imgui_font_atlas.height,
            ),
        };

        // Should just be one, so pop/unwrap
        let imgui_font_atlas_image = renderer::assets::image_utils::load_images(
            &device_context,
            device_context
                .queue_family_indices()
                .transfer_queue_family_index,
            &device_context.queues().transfer_queue,
            device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            &device_context.queues().graphics_queue,
            &[imgui_font_atlas],
        )?
        .pop()
        .unwrap();

        let dyn_resource_allocator = asset_manager.create_dyn_resource_allocator_set();
        let image =
            dyn_resource_allocator.insert_image(ManuallyDrop::into_inner(imgui_font_atlas_image));

        let image_view_meta = dsc::ImageViewMeta::default_2d_no_mips_or_layers(
            dsc::Format::R8G8B8A8_UNORM,
            dsc::ImageAspectFlag::Color.into(),
        );

        dyn_resource_allocator.insert_image_view(device_context, &image, image_view_meta)
    }
}

impl GameRenderer {
    // This is externally exposed, it checks result of the previous frame (which implicitly also
    // waits for the previous frame to complete if it hasn't already)
    #[profiling::function]
    pub fn start_rendering_next_frame(
        &self,
        resources: &Resources,
        world: &World,
        window: &dyn Window,
    ) -> VkResult<()> {
        //
        // Block until the previous frame completes being submitted to GPU
        //
        let t0 = std::time::Instant::now();
        let previous_frame_job_result = resources
            .get_mut::<VkSurface>()
            .unwrap()
            .wait_until_frame_not_in_flight();
        let t1 = std::time::Instant::now();
        log::trace!(
            "[main] wait for previous frame present {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        //
        // Check the result of the previous frame. Three outcomes:
        //  - Previous frame was successful: immediately try rendering again with the same swapchain
        //  - Previous frame failed but resolvable by rebuilding the swapchain - skip trying to
        //    render again with the same swapchain
        //  - Previous frame failed with unrecoverable error: bail
        //
        let rebuild_swapchain = match &previous_frame_job_result {
            Ok(_) => Ok(false),
            Err(ash::vk::Result::SUCCESS) => Ok(false),
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true),
            Err(ash::vk::Result::SUBOPTIMAL_KHR) => Ok(true),
            Err(e) => Err(*e),
        }?;

        //
        // If the previous frame rendered properly, try to render immediately with the same
        // swapchain as last time
        //
        let previous_frame_job_result = if !rebuild_swapchain {
            self.aquire_swapchain_image_and_render(resources, world, window)
        } else {
            previous_frame_job_result
        };

        //
        // Rebuild the swapchain if needed
        //
        if let Err(e) = previous_frame_job_result {
            match e {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    log::info!("  ERROR_OUT_OF_DATE_KHR");
                    SwapchainLifetimeListener::rebuild_swapchain(resources, window, self)
                }
                ash::vk::Result::SUCCESS => Ok(()),
                ash::vk::Result::SUBOPTIMAL_KHR => Ok(()),
                _ => {
                    log::warn!("Unexpected rendering error {:?}", e);
                    return Err(e);
                }
            }?;

            // If we fail again immediately, bail
            self.aquire_swapchain_image_and_render(resources, world, window)?
        }

        Ok(())
    }

    //TODO: In a failure, return the frame_in_flight and cancel the render. This will make
    // previous_frame_result unnecessary
    fn aquire_swapchain_image_and_render(
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
            log::trace!(
                "[main] wait for swapchain image took {} ms",
                (t1 - t0).as_secs_f32() * 1000.0
            );
            result?
        };

        // After this point, any failures will be deferred to handle in next frame
        Self::create_and_start_render_job(self, world, resources, window, frame_in_flight);
        Ok(())
    }

    fn create_and_start_render_job(
        game_renderer: &GameRenderer,
        world: &World,
        resources: &Resources,
        window: &dyn Window,
        frame_in_flight: FrameInFlight,
    ) {
        let result =
            Self::try_create_render_job(&game_renderer, world, resources, window, &frame_in_flight);

        match result {
            Ok(prepared_frame) => {
                let mut guard = game_renderer.inner.lock().unwrap();
                let game_renderer_inner = &mut *guard;
                game_renderer_inner
                    .render_thread
                    .render(prepared_frame, frame_in_flight)
            }
            Err(e) => frame_in_flight.cancel_present(Err(e)),
        };
    }

    fn try_create_render_job(
        game_renderer: &GameRenderer,
        world: &World,
        resources: &Resources,
        window: &dyn Window,
        frame_in_flight: &FrameInFlight,
    ) -> VkResult<RenderFrameJob> {
        //
        // Fetch resources
        //
        let time_state_fetch = resources.get::<TimeState>().unwrap();
        let time_state = &*time_state_fetch;

        let mut static_visibility_node_set_fetch =
            resources.get_mut::<StaticVisibilityNodeSet>().unwrap();
        let static_visibility_node_set = &mut *static_visibility_node_set_fetch;

        let mut dynamic_visibility_node_set_fetch =
            resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();
        let dynamic_visibility_node_set = &mut *dynamic_visibility_node_set_fetch;

        let render_registry = resources.get::<RenderRegistry>().unwrap().clone();
        let device_context = resources.get::<VkDeviceContext>().unwrap().clone();

        let mut asset_manager_fetch = resources.get_mut::<AssetManager>().unwrap();
        let asset_manager = &mut *asset_manager_fetch;

        //
        // Mark the previous frame as completed
        //
        asset_manager.on_frame_complete()?;

        let resource_context = asset_manager.resource_manager().resource_context();

        let mut guard = game_renderer.inner.lock().unwrap();
        let game_renderer_inner = &mut *guard;

        let main_camera_render_phase_mask = game_renderer_inner.main_camera_render_phase_mask;

        let static_resources = &game_renderer_inner.static_resources;

        //
        // Swapchain Status
        //
        let swapchain_resources = game_renderer_inner.swapchain_resources.as_mut().unwrap();
        let swapchain_image =
            swapchain_resources.swapchain_images[frame_in_flight.present_index() as usize].clone();
        let swapchain_surface_info = swapchain_resources.swapchain_surface_info.clone();
        let swapchain_info = swapchain_resources.swapchain_info.clone();

        let render_view_set = RenderViewSet::default();

        //
        // Determine Camera Location
        //
        let main_view = {
            const CAMERA_XY_DISTANCE: f32 = 12.0;
            const CAMERA_Z: f32 = 5.0;
            const CAMERA_ROTATE_SPEED: f32 = -0.00;
            const CAMERA_LOOP_OFFSET: f32 = -0.3;
            let loop_time = time_state.total_time().as_secs_f32();
            let eye = glam::Vec3::new(
                CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
                CAMERA_XY_DISTANCE * f32::sin(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
                CAMERA_Z,
            );

            let extents = window.logical_size();
            let extents_width = extents.width.max(1);
            let extents_height = extents.height.max(1);
            let aspect_ratio = extents_width as f32 / extents_height as f32;

            let view =
                glam::Mat4::look_at_rh(eye, glam::Vec3::zero(), glam::Vec3::new(0.0, 0.0, 1.0));
            let proj = glam::Mat4::perspective_infinite_reverse_rh(
                std::f32::consts::FRAC_PI_4,
                aspect_ratio,
                0.01,
            );
            let proj = glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0)) * proj;

            render_view_set.create_view(
                eye,
                view,
                proj,
                main_camera_render_phase_mask,
                "main".to_string(),
            )
        };

        //
        // Determine shadowmap views
        //
        let mut directional_light: Option<DirectionalLightComponent> = None;
        let mut query = <Read<DirectionalLightComponent>>::query();
        for light in query.iter(world) {
            directional_light = Some(light.clone());
        }

        // Temporarily assume we have a light
        let directional_light = directional_light.unwrap();
        let directional_light_view = {
            let eye_position = directional_light.direction * -40.0;
            let view = glam::Mat4::look_at_rh(
                eye_position,
                glam::Vec3::zero(),
                glam::Vec3::new(0.0, 0.0, 1.0),
            );

            let ortho_projection_size = 10.0;
            let proj = glam::Mat4::orthographic_rh(
                -ortho_projection_size,
                ortho_projection_size,
                ortho_projection_size,
                -ortho_projection_size,
                100.0,
                0.01,
            );

            //NOTE: This would be the correct way to do perspective projection in our coordinate system
            // let proj = glam::Mat4::perspective_infinite_reverse_rh(
            //     std::f32::consts::FRAC_PI_4,
            //     aspect_ratio,
            //     0.01,
            // );
            // let proj = glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0)) * proj;

            let view = render_view_set.create_view(
                eye_position,
                view,
                proj,
                main_camera_render_phase_mask,
                "shadow_map".to_string(),
            );

            //println!("VIEW: {:?} {:?}", (glam::Vec3::zero() - (directional_light.direction * -40.0)).normalize(), view.view_dir());

            view
        };

        //
        // Visibility
        //
        let main_view_static_visibility_result =
            static_visibility_node_set.calculate_static_visibility(&main_view);
        let main_view_dynamic_visibility_result =
            dynamic_visibility_node_set.calculate_dynamic_visibility(&main_view);

        let directional_light_view_view_static_visibility_result =
            static_visibility_node_set.calculate_static_visibility(&directional_light_view);
        let directional_light_view_view_dynamic_visibility_result =
            dynamic_visibility_node_set.calculate_dynamic_visibility(&directional_light_view);

        log::trace!(
            "main view static node count: {}",
            main_view_static_visibility_result.handles.len()
        );

        log::trace!(
            "main view dynamic node count: {}",
            main_view_dynamic_visibility_result.handles.len()
        );

        //
        // Build the frame packet - this takes the views and visibility results and creates a
        // structure that's used during the extract/prepare/write phases
        //
        let frame_packet_builder = {
            let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
            sprite_render_nodes.update();
            let mut mesh_render_nodes = resources.get_mut::<MeshRenderNodeSet>().unwrap();
            mesh_render_nodes.update();
            let mut all_render_nodes = AllRenderNodes::default();
            all_render_nodes.add_render_nodes(&*sprite_render_nodes);
            all_render_nodes.add_render_nodes(&*mesh_render_nodes);

            FramePacketBuilder::new(&all_render_nodes)
        };

        // After these jobs end, user calls functions to start jobs that extract data
        frame_packet_builder.add_view(
            &main_view,
            &[
                main_view_static_visibility_result,
                main_view_dynamic_visibility_result,
            ],
        );

        frame_packet_builder.add_view(
            &directional_light_view,
            &[
                directional_light_view_view_static_visibility_result,
                directional_light_view_view_dynamic_visibility_result,
            ],
        );

        //
        // Update Resources and flush descriptor set changes
        //
        asset_manager.on_begin_frame()?;

        //
        // Render Graph, this is needed now as some of the outputs from the graph may be used in
        // the extract phase
        //
        let bloom_extract_material_pass = asset_manager
            .get_material_pass_by_index(&static_resources.bloom_extract_material, 0)
            .unwrap();

        let bloom_blur_material_pass = asset_manager
            .get_material_pass_by_index(&static_resources.bloom_blur_material, 0)
            .unwrap();

        let bloom_combine_material_pass = asset_manager
            .get_material_pass_by_index(&static_resources.bloom_combine_material, 0)
            .unwrap();

        //TODO: This is now possible to run on the render thread
        let render_graph = render_graph::build_render_graph(
            &device_context,
            &resource_context,
            &swapchain_surface_info,
            &swapchain_info,
            swapchain_image,
            main_view.clone(),
            directional_light_view.clone(),
            bloom_extract_material_pass,
            bloom_blur_material_pass,
            bloom_combine_material_pass,
        )?;

        //
        // Extract Jobs
        //
        let frame_packet = frame_packet_builder.build();
        let extract_job_set = {
            let mut extract_job_set = ExtractJobSet::new();

            //TODO: Is it possible to know up front what extract jobs aren't necessary based on
            // renderphases?

            // Sprites
            extract_job_set.add_job(create_sprite_extract_job(
                guard.static_resources.sprite_material.clone(),
            ));

            // Meshes
            extract_job_set.add_job(create_mesh_extract_job(
                render_graph.shadow_map,
                directional_light_view.clone(),
            ));

            // Debug 3D
            extract_job_set.add_job(create_debug3d_extract_job(
                &guard.static_resources.debug3d_material,
            ));

            extract_job_set.add_job(create_imgui_extract_job(
                swapchain_surface_info.extents,
                &guard.static_resources.imgui_material,
                guard.imgui_font_atlas_image_view.clone(),
            ));

            extract_job_set
        };

        let prepare_job_set = {
            profiling::scope!("renderer extract");
            let extract_context = RenderJobExtractContext::new(&world, &resources, asset_manager);
            extract_job_set.extract(
                &extract_context,
                &frame_packet,
                &[&main_view, &directional_light_view],
            )
        };

        let game_renderer = game_renderer.clone();

        let prepared_frame = RenderFrameJob {
            game_renderer,
            prepare_job_set,
            render_graph: render_graph.executor,
            resource_context,
            frame_packet,
            main_view,
            directional_light_view,
            render_registry,
            device_context,
        };

        Ok(prepared_frame)
    }
}
