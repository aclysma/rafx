use crate::features::debug3d::create_debug3d_extract_job;
use crate::features::mesh::{
    create_mesh_extract_job, LightId, MeshRenderNodeSet, ShadowMapData, ShadowMapRenderView,
};
use crate::features::sprite::{create_sprite_extract_job, SpriteRenderNodeSet};
use crate::phases::TransparentRenderPhase;
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase, UiRenderPhase};
use crate::render_contexts::RenderJobExtractContext;
use crate::time::TimeState;
use legion::*;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{image_upload, GpuImageDataColorSpace};
use rafx::assets::{AssetManager, GpuImageData};
use rafx::framework::{DynResourceAllocatorSet, RenderResources};
use rafx::framework::{ImageViewResource, ResourceArc};
use rafx::nodes::{
    AllRenderNodes, ExtractJobSet, FramePacketBuilder, RenderPhaseMask, RenderPhaseMaskBuilder,
    RenderRegistry, RenderView, RenderViewDepthRange, RenderViewSet, VisibilityResult,
};
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use std::sync::{Arc, Mutex};

mod static_resources;
pub use static_resources::GameRendererStaticResources;

mod render_thread;
use render_thread::RenderThread;

mod swapchain_resources;
use swapchain_resources::SwapchainResources;

mod render_frame_job;
use render_frame_job::RenderFrameJob;

mod render_graph;

//TODO: Find a way to not expose this
mod swapchain_handling;
pub use swapchain_handling::SwapchainHandler;

use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::RenderOptions;
use arrayvec::ArrayVec;
use fnv::FnvHashMap;
use rafx::api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx::api::{
    RafxApi, RafxDeviceContext, RafxError, RafxPresentableFrame, RafxQueue, RafxResourceType,
    RafxResult, RafxSampleCount,
};
use rafx::assets::image_upload::ImageUploadParams;

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

#[cfg(feature = "use-imgui")]
#[derive(Clone)]
pub struct ImguiFontAtlas(pub ResourceArc<ImageViewResource>);

#[derive(Clone)]
pub struct InvalidResources {
    pub invalid_image: ResourceArc<ImageViewResource>,
    pub invalid_cube_map_image: ResourceArc<ImageViewResource>,
}

pub struct GameRendererInner {
    #[cfg(feature = "use-imgui")]
    imgui_font_atlas_image_view: ImguiFontAtlas,
    invalid_resources: InvalidResources,

    // Everything that is loaded all the time
    static_resources: GameRendererStaticResources,

    // Everything that requires being created after the swapchain inits
    swapchain_resources: Option<SwapchainResources>,

    render_thread: RenderThread,
}

#[derive(Clone)]
pub struct GameRenderer {
    inner: Arc<Mutex<GameRendererInner>>,
    graphics_queue: RafxQueue,
    transfer_queue: RafxQueue,
}

impl GameRenderer {
    pub fn new(
        resources: &Resources,
        graphics_queue: &RafxQueue,
        transfer_queue: &RafxQueue,
    ) -> RafxResult<Self> {
        let mut asset_resource_fetch = resources.get_mut::<AssetResource>().unwrap();
        let asset_resource = &mut *asset_resource_fetch;

        let mut asset_manager_fetch = resources.get_mut::<AssetManager>().unwrap();
        let asset_manager = &mut *asset_manager_fetch;

        let rafx_api = resources.get_mut::<RafxApi>().unwrap();
        let device_context = rafx_api.device_context();

        let dyn_resource_allocator = asset_manager.create_dyn_resource_allocator_set();

        let mut upload = RafxTransferUpload::new(
            &device_context,
            asset_manager.transfer_queue(),
            asset_manager.graphics_queue(),
            16 * 1024 * 1024,
        )?;

        #[cfg(feature = "use-imgui")]
        let imgui_font_atlas_image_view = GameRenderer::create_font_atlas_image_view(
            resources,
            &device_context,
            &mut upload,
            &dyn_resource_allocator,
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

        upload.block_until_upload_complete()?;

        log::info!("all waits complete");
        let game_renderer_resources =
            GameRendererStaticResources::new(asset_resource, asset_manager)?;

        let render_thread = RenderThread::start();

        let renderer = GameRendererInner {
            #[cfg(feature = "use-imgui")]
            imgui_font_atlas_image_view: ImguiFontAtlas(imgui_font_atlas_image_view),
            invalid_resources: InvalidResources {
                invalid_image,
                invalid_cube_map_image,
            },
            static_resources: game_renderer_resources,
            swapchain_resources: None,

            render_thread,
        };

        Ok(GameRenderer {
            inner: Arc::new(Mutex::new(renderer)),
            graphics_queue: graphics_queue.clone(),
            transfer_queue: transfer_queue.clone(),
        })
    }

    fn graphics_queue(&self) -> &RafxQueue {
        &self.graphics_queue
    }

    fn transfer_queue(&self) -> &RafxQueue {
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

    #[cfg(feature = "use-imgui")]
    fn create_font_atlas_image_view(
        resources: &Resources,
        device_context: &RafxDeviceContext,
        upload: &mut RafxTransferUpload,
        dyn_resource_allocator: &DynResourceAllocatorSet,
    ) -> RafxResult<ResourceArc<ImageViewResource>> {
        use crate::imgui_support::Sdl2ImguiManager;

        //TODO: Simplify this setup code for the imgui font atlas
        let imgui_font_atlas = resources
            .get::<Sdl2ImguiManager>()
            .unwrap()
            .build_font_atlas();

        let imgui_font_atlas = GpuImageData::new_simple(
            imgui_font_atlas.width,
            imgui_font_atlas.height,
            GpuImageDataColorSpace::Linear.rgba8(),
            imgui_font_atlas.data,
        );

        Self::upload_image_data(
            device_context,
            upload,
            dyn_resource_allocator,
            &imgui_font_atlas,
            ImageUploadParams {
                generate_mips: false,
                ..Default::default()
            },
        )
        .map_err(|x| Into::<RafxError>::into(x))
    }

    // This is externally exposed, it checks result of the previous frame (which implicitly also
    // waits for the previous frame to complete if it hasn't already)
    #[profiling::function]
    pub fn start_rendering_next_frame(
        &self,
        resources: &Resources,
        world: &World,
        window_width: u32,
        window_height: u32,
    ) -> RafxResult<()> {
        //
        // Block until the previous frame completes being submitted to GPU
        //
        let t0 = std::time::Instant::now();
        //let mut swapchain_helper = resources.get_mut::<RafxSwapchainHelper>().unwrap();

        let presentable_frame =
            SwapchainHandler::acquire_next_image(resources, window_width, window_height, self)?;

        //let presentable_frame = swapchain_helper.acquire_next_image(window, window_width, window_height, )?;
        let t1 = std::time::Instant::now();
        log::trace!(
            "[main] wait for previous frame present {} ms",
            (t1 - t0).as_secs_f32() * 1000.0
        );

        Self::create_and_start_render_job(
            self,
            world,
            resources,
            window_width,
            window_height,
            presentable_frame,
        );

        Ok(())
    }

    fn create_and_start_render_job(
        game_renderer: &GameRenderer,
        world: &World,
        resources: &Resources,
        window_width: u32,
        window_height: u32,
        presentable_frame: RafxPresentableFrame,
    ) {
        let result = Self::try_create_render_job(
            &game_renderer,
            world,
            resources,
            window_width,
            window_height,
            &presentable_frame,
        );

        match result {
            Ok(prepared_frame) => {
                let mut guard = game_renderer.inner.lock().unwrap();
                let game_renderer_inner = &mut *guard;
                game_renderer_inner
                    .render_thread
                    .render(prepared_frame, presentable_frame)
            }
            Err(e) => {
                let graphics_queue = game_renderer.graphics_queue();
                presentable_frame.present_with_error(graphics_queue, e)
            }
        };
    }

    fn try_create_render_job(
        game_renderer: &GameRenderer,
        world: &World,
        resources: &Resources,
        window_width: u32,
        window_height: u32,
        presentable_frame: &RafxPresentableFrame,
    ) -> RafxResult<RenderFrameJob> {
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

        let render_options = resources.get::<RenderOptions>().unwrap().clone();

        let render_registry = resources.get::<RenderRegistry>().unwrap().clone();
        let device_context = resources.get::<RafxDeviceContext>().unwrap().clone();

        let mut asset_manager_fetch = resources.get_mut::<AssetManager>().unwrap();
        let mut render_resources = RenderResources::new();
        let asset_manager = &mut *asset_manager_fetch;

        //
        // Mark the previous frame as completed
        //
        asset_manager.on_frame_complete()?;

        let resource_context = asset_manager.resource_manager().resource_context();

        let mut guard = game_renderer.inner.lock().unwrap();
        let game_renderer_inner = &mut *guard;

        let static_resources = &game_renderer_inner.static_resources;
        render_resources.insert(static_resources.clone());
        render_resources.insert(game_renderer_inner.invalid_resources.clone());
        #[cfg(feature = "use-imgui")]
        render_resources.insert(game_renderer_inner.imgui_font_atlas_image_view.clone());

        //
        // Swapchain Status
        //
        let swapchain_resources = game_renderer_inner.swapchain_resources.as_mut().unwrap();

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

        let swapchain_surface_info = swapchain_resources.swapchain_surface_info.clone();
        render_resources.insert(swapchain_surface_info.clone());

        let render_view_set = RenderViewSet::default();

        //
        // Determine Camera Location
        //
        let main_view = GameRenderer::calculate_main_view(
            &render_view_set,
            window_width,
            window_height,
            time_state,
        );

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

        let graph_config = {
            let swapchain_format = swapchain_surface_info.format;
            let sample_count = if render_options.enable_msaa {
                RafxSampleCount::SampleCount4
            } else {
                RafxSampleCount::SampleCount1
            };

            let color_format = if render_options.enable_hdr {
                swapchain_resources.default_color_format_hdr
            } else {
                swapchain_resources.default_color_format_sdr
            };

            render_graph::RenderGraphConfig {
                color_format,
                depth_format: swapchain_resources.default_depth_format,
                samples: sample_count,
                enable_hdr: render_options.enable_hdr,
                swapchain_format,
                enable_bloom: render_options.enable_bloom,
                blur_pass_count: render_options.blur_pass_count,
            }
        };

        //
        // Extract Jobs
        //
        let extract_job_set = {
            let mut extract_job_set = ExtractJobSet::new();

            //TODO: Is it possible to know up front what extract jobs aren't necessary based on
            // render phases?

            // Sprites
            extract_job_set.add_job(create_sprite_extract_job());

            // Meshes
            extract_job_set.add_job(create_mesh_extract_job());

            // Debug 3D
            extract_job_set.add_job(create_debug3d_extract_job());

            #[cfg(feature = "use-imgui")]
            {
                use crate::features::imgui::create_imgui_extract_job;
                extract_job_set.add_job(create_imgui_extract_job());
            }

            extract_job_set
        };

        let frame_packet = frame_packet_builder.build();
        let prepare_job_set = {
            profiling::scope!("renderer extract");
            let extract_context =
                RenderJobExtractContext::new(&world, &resources, &render_resources, asset_manager);

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

        //TODO: This is now possible to run on the render thread
        let render_graph = render_graph::build_render_graph(
            &device_context,
            &resource_context,
            asset_manager,
            &graph_config,
            swapchain_image,
            main_view.clone(),
            &shadow_map_render_views,
            swapchain_resources,
            static_resources,
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
        render_resources.insert(shadow_map_data);

        let game_renderer = game_renderer.clone();
        let graphics_queue = game_renderer.graphics_queue.clone();

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
            render_resources,
            graphics_queue,
        };

        Ok(prepared_frame)
    }

    #[profiling::function]
    fn calculate_main_view(
        render_view_set: &RenderViewSet,
        window_width: u32,
        window_height: u32,
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

        let aspect_ratio = window_width as f32 / window_height as f32;

        let view = glam::Mat4::look_at_rh(eye, glam::Vec3::zero(), glam::Vec3::new(0.0, 0.0, 1.0));

        let near_plane = 0.01;
        let proj = glam::Mat4::perspective_infinite_reverse_rh(
            std::f32::consts::FRAC_PI_4,
            aspect_ratio,
            near_plane,
        );

        render_view_set.create_view(
            eye,
            view,
            proj,
            (window_width, window_height),
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
                -ortho_projection_size,
                ortho_projection_size,
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
            fn cube_map_face(
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
                cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[0]),
                cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[1]),
                cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[2]),
                cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[3]),
                cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[4]),
                cube_map_face(shadow_map_phase_mask, &render_view_set, light, position.position, &cube_map_view_directions[5]),
            ];

            let index = shadow_map_render_views.len();
            shadow_map_render_views.push(ShadowMapRenderView::Cube(cube_map_views));
            let old = shadow_map_lookup.insert(LightId::PointLight(*entity), index);
            assert!(old.is_none());
        }

        (shadow_map_lookup, shadow_map_render_views)
    }
}
