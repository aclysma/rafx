use crate::asset_resource::AssetResource;
use crate::features::debug3d::create_debug3d_extract_job;
use crate::features::mesh::{
    create_mesh_extract_job, LightId, MeshRenderNodeSet, ShadowMapData, ShadowMapRenderView,
};
use crate::features::sprite::{create_sprite_extract_job, SpriteRenderNodeSet};
use crate::imgui_support::Sdl2ImguiManager;
use crate::phases::TransparentRenderPhase;
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase, UiRenderPhase};
use crate::render_contexts::RenderJobExtractContext;
use crate::time::TimeState;
use ash::prelude::VkResult;
use legion::*;
use rafx::assets::image_utils;
use rafx::assets::AssetManager;
use rafx::nodes::{
    AllRenderNodes, ExtractJobSet, FramePacketBuilder, RenderPhaseMask, RenderPhaseMaskBuilder,
    RenderRegistry, RenderView, RenderViewDepthRange, RenderViewSet, VisibilityResult,
};
use rafx::resources::vk_description as dsc;
use rafx::resources::{ImageViewResource, ResourceArc};
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use rafx::vulkan::{FrameInFlight, VkContext, VkDeviceContext, VkSurface, Window};
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
use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::features::imgui::create_imgui_extract_job;
use arrayvec::ArrayVec;
use fnv::FnvHashMap;
pub use swapchain_handling::SwapchainLifetimeListener;

/// Creates a right-handed perspective projection matrix with [0,1] depth range.
pub fn perspective_rh(
    fov_y_radians: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
) -> glam::Mat4 {
    debug_assert!(z_near > 0.0 && z_far > 0.0);
    let (sin_fov, cos_fov) = (0.5 * fov_y_radians).sin_cos();
    let h = cos_fov / sin_fov;
    let w = h / aspect_ratio;
    let r = z_far / (z_near - z_far);
    glam::Mat4::from_cols(
        glam::Vec4::new(w, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, h, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, r, -1.0),
        glam::Vec4::new(0.0, 0.0, r * z_near, 0.0),
    )
}

pub fn matrix_flip_y(proj: glam::Mat4) -> glam::Mat4 {
    glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0)) * proj
}

// Equivalent to flipping near/far values?
pub fn matrix_reverse_z(proj: glam::Mat4) -> glam::Mat4 {
    let reverse_mat = glam::Mat4::from_cols(
        glam::Vec4::new(1.0, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, 1.0, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, -1.0, 0.0),
        glam::Vec4::new(0.0, 0.0, 1.0, 1.0),
    );
    reverse_mat * proj
}

pub struct GameRendererInner {
    imgui_font_atlas_image_view: ResourceArc<ImageViewResource>,

    // Everything that is loaded all the time
    static_resources: GameRendererStaticResources,

    // Everything that requires being created after the swapchain inits
    swapchain_resources: Option<SwapchainResources>,

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

        log::info!("all waits complete");
        let game_renderer_resources =
            GameRendererStaticResources::new(asset_resource, asset_manager)?;

        let render_thread = RenderThread::start();

        let renderer = GameRendererInner {
            imgui_font_atlas_image_view,
            static_resources: game_renderer_resources,
            swapchain_resources: None,

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

        let imgui_font_atlas = image_utils::DecodedTexture {
            width: imgui_font_atlas.width,
            height: imgui_font_atlas.height,
            data: imgui_font_atlas.data,
            color_space: image_utils::ColorSpace::Linear,
            mips: image_utils::default_mip_settings_for_image(
                imgui_font_atlas.width,
                imgui_font_atlas.height,
            ),
        };

        // Should just be one, so pop/unwrap
        let imgui_font_atlas_image = image_utils::load_images(
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
        let main_view = GameRenderer::calculte_main_view(&render_view_set, window, time_state);

        //
        // Determine shadowmap views
        //
        let (shadow_map_lookup, shadow_map_render_views) =
            GameRenderer::calculate_shadow_map_views(&render_view_set, world);

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

        struct RenderViewVisibility {
            render_view: RenderView,
            static_visibility: VisibilityResult,
            dynamic_visibility: VisibilityResult,
        }

        enum ShadowMapVisibility {
            Single(RenderViewVisibility),
            Cube(ArrayVec<[RenderViewVisibility; 6]>),
        }

        let mut shadow_map_visibility_results = Vec::default();
        for render_view in &shadow_map_render_views {
            fn create_render_view_visibility(
                static_visibility_node_set: &mut StaticVisibilityNodeSet,
                dynamic_visibility_node_set: &mut DynamicVisibilityNodeSet,
                render_view: &RenderView,
            ) -> RenderViewVisibility {
                let static_visibility =
                    static_visibility_node_set.calculate_static_visibility(&render_view);
                let dynamic_visibility =
                    dynamic_visibility_node_set.calculate_dynamic_visibility(&render_view);

                log::trace!(
                    "shadow view static node count: {}",
                    static_visibility.handles.len()
                );

                log::trace!(
                    "shadow view dynamic node count: {}",
                    dynamic_visibility.handles.len()
                );

                RenderViewVisibility {
                    render_view: render_view.clone(),
                    static_visibility,
                    dynamic_visibility,
                }
            }

            match render_view {
                ShadowMapRenderView::Single(view) => shadow_map_visibility_results.push(
                    ShadowMapVisibility::Single(create_render_view_visibility(
                        static_visibility_node_set,
                        dynamic_visibility_node_set,
                        view,
                    )),
                ),
                ShadowMapRenderView::Cube(views) => {
                    shadow_map_visibility_results.push(ShadowMapVisibility::Cube(
                        [
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[0],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[1],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[2],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[3],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[4],
                            ),
                            create_render_view_visibility(
                                static_visibility_node_set,
                                dynamic_visibility_node_set,
                                &views[5],
                            ),
                        ]
                        .into(),
                    ));
                }
            }
        }

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

        for shadow_map_visibility_result in shadow_map_visibility_results {
            match shadow_map_visibility_result {
                ShadowMapVisibility::Single(view) => {
                    frame_packet_builder.add_view(
                        &view.render_view,
                        &[view.static_visibility, view.dynamic_visibility],
                    );
                }
                ShadowMapVisibility::Cube(views) => {
                    for view in views {
                        let static_visibility = view.static_visibility;
                        frame_packet_builder.add_view(
                            &view.render_view,
                            &[static_visibility, view.dynamic_visibility],
                        );
                    }
                }
            }
        }

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
            &shadow_map_render_views,
            bloom_extract_material_pass,
            bloom_blur_material_pass,
            bloom_combine_material_pass,
        )?;

        assert_eq!(
            shadow_map_render_views.len(),
            render_graph.shadow_map_image_views.len()
        );
        let shadow_map_data = ShadowMapData {
            shadow_map_lookup,
            shadow_map_render_views: shadow_map_render_views.clone(),
            shadow_map_image_views: render_graph.shadow_map_image_views.clone(),
        };

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
            extract_job_set.add_job(create_mesh_extract_job(shadow_map_data));

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

            let mut extract_views = Vec::default();
            extract_views.push(&main_view);
            for shadow_map_view in &shadow_map_render_views {
                match shadow_map_view {
                    ShadowMapRenderView::Single(view) => {
                        extract_views.push(view);
                    }
                    ShadowMapRenderView::Cube(views) => {
                        for view in views {
                            extract_views.push(view);
                        }
                    }
                }
            }

            extract_job_set.extract(&extract_context, &frame_packet, &extract_views)
        };

        let game_renderer = game_renderer.clone();

        let prepared_frame = RenderFrameJob {
            game_renderer,
            prepare_job_set,
            render_graph: render_graph.executor,
            resource_context,
            frame_packet,
            main_view,
            shadow_map_render_views,
            render_registry,
            device_context,
        };

        Ok(prepared_frame)
    }

    #[profiling::function]
    fn calculte_main_view(
        render_view_set: &RenderViewSet,
        window: &dyn Window,
        time_state: &TimeState,
    ) -> RenderView {
        let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
            .add_render_phase::<OpaqueRenderPhase>()
            .add_render_phase::<TransparentRenderPhase>()
            .add_render_phase::<UiRenderPhase>()
            .build();

        const CAMERA_XY_DISTANCE: f32 = 12.0;
        const CAMERA_Z: f32 = 6.0;
        const CAMERA_ROTATE_SPEED: f32 = -0.10;
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

        let view = glam::Mat4::look_at_rh(eye, glam::Vec3::zero(), glam::Vec3::new(0.0, 0.0, 1.0));

        let near_plane = 0.01;
        let proj = glam::Mat4::perspective_infinite_reverse_rh(
            std::f32::consts::FRAC_PI_4,
            aspect_ratio,
            near_plane,
        );
        // Flip it on Y
        let proj = glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0)) * proj;

        render_view_set.create_view(
            eye,
            view,
            proj,
            (extents_width, extents_height),
            RenderViewDepthRange::new_infinite_reverse(near_plane),
            main_camera_render_phase_mask,
            "main".to_string(),
        )
    }

    #[profiling::function]
    fn calculate_shadow_map_views(
        render_view_set: &RenderViewSet,
        world: &World,
    ) -> (FnvHashMap<LightId, usize>, Vec<ShadowMapRenderView>) {
        let mut shadow_map_render_views = Vec::default();
        let mut shadow_map_lookup = FnvHashMap::default();

        let shadow_map_phase_mask = RenderPhaseMaskBuilder::default()
            .add_render_phase::<ShadowMapRenderPhase>()
            .build();

        //TODO: The look-at calls in this fn will fail if the light is pointed straight down

        const SHADOW_MAP_RESOLUTION: u32 = 1024;

        let mut query = <(Entity, Read<SpotLightComponent>, Read<PositionComponent>)>::query();
        for (entity, light, position) in query.iter(world) {
            //TODO: Transform direction by rotation
            let eye_position = position.position;
            let light_to = position.position + light.direction;

            let view =
                glam::Mat4::look_at_rh(eye_position, light_to, glam::Vec3::new(0.0, 0.0, 1.0));

            let near_plane = 0.25;
            let far_plane = 100.0;
            let proj = perspective_rh(light.spotlight_half_angle * 2.0, 1.0, far_plane, near_plane);
            let proj = matrix_flip_y(proj);

            let view = render_view_set.create_view(
                eye_position,
                view,
                proj,
                (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
                RenderViewDepthRange::new_reverse(near_plane, far_plane),
                shadow_map_phase_mask,
                "shadow_map".to_string(),
            );

            let index = shadow_map_render_views.len();
            shadow_map_render_views.push(ShadowMapRenderView::Single(view));
            let old = shadow_map_lookup.insert(LightId::SpotLight(*entity), index);
            assert!(old.is_none());
        }

        let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
        for (entity, light) in query.iter(world) {
            let eye_position = light.direction * -40.0;
            let view = glam::Mat4::look_at_rh(
                eye_position,
                glam::Vec3::zero(),
                glam::Vec3::new(0.0, 0.0, 1.0),
            );

            let near_plane = 0.25;
            let far_plane = 100.0;
            let ortho_projection_size = 10.0;
            let proj = glam::Mat4::orthographic_rh(
                -ortho_projection_size,
                ortho_projection_size,
                ortho_projection_size,
                -ortho_projection_size,
                far_plane,
                near_plane,
            );

            let view = render_view_set.create_view(
                eye_position,
                view,
                proj,
                (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
                RenderViewDepthRange::new_reverse(near_plane, far_plane),
                shadow_map_phase_mask,
                "shadow_map".to_string(),
            );

            let index = shadow_map_render_views.len();
            shadow_map_render_views.push(ShadowMapRenderView::Single(view));
            let old = shadow_map_lookup.insert(LightId::DirectionalLight(*entity), index);
            assert!(old.is_none());
        }

        #[rustfmt::skip]
        // The eye offset and up vector. The directions are per the specification of cubemaps
        let cube_map_view_directions = [
            (glam::Vec3::unit_x(), glam::Vec3::unit_y()),
            (glam::Vec3::unit_x() * -1.0, glam::Vec3::unit_y()),
            (glam::Vec3::unit_y(), glam::Vec3::unit_z() * -1.0),
            (glam::Vec3::unit_y() * -1.0, glam::Vec3::unit_z()),
            (glam::Vec3::unit_z(), glam::Vec3::unit_y()),
            (glam::Vec3::unit_z() * -1.0, glam::Vec3::unit_y()),
        ];

        let mut query = <(Entity, Read<PointLightComponent>, Read<PositionComponent>)>::query();
        for (entity, light, position) in query.iter(world) {
            fn cubemap_face(
                phase_mask: RenderPhaseMask,
                render_view_set: &RenderViewSet,
                light: &PointLightComponent,
                position: glam::Vec3,
                cube_map_view_directions: &(glam::Vec3, glam::Vec3),
            ) -> RenderView {
                //NOTE: Cubemaps always use LH
                let view = glam::Mat4::look_at_lh(
                    position,
                    position + cube_map_view_directions.0,
                    cube_map_view_directions.1,
                );

                let near = 0.25;
                let far = light.range;
                let proj = glam::Mat4::perspective_lh(std::f32::consts::FRAC_PI_2, 1.0, far, near);
                let proj = matrix_flip_y(proj);

                render_view_set.create_view(
                    position,
                    view,
                    proj,
                    (SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION),
                    RenderViewDepthRange::new_reverse(near, far),
                    phase_mask,
                    "shadow_map".to_string(),
                )
            }

            #[rustfmt::skip]
            let cube_map_views = [
                cubemap_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[0]),
                cubemap_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[1]),
                cubemap_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[2]),
                cubemap_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[3]),
                cubemap_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[4]),
                cubemap_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[5]),
            ];

            let index = shadow_map_render_views.len();
            shadow_map_render_views.push(ShadowMapRenderView::Cube(cube_map_views));
            let old = shadow_map_lookup.insert(LightId::PointLight(*entity), index);
            assert!(old.is_none());
        }

        (shadow_map_lookup, shadow_map_render_views)
    }
}
