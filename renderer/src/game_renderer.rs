use renderer_ext::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener, Sdl2ImguiManager};
use renderer_shell_vulkan::{VkDevice, VkSwapchain, VkSurface, Window, VkTransferUpload, VkTransferUploadState, VkImage, VkDeviceContext, VkContextBuilder, VkCreateContextError, VkContext, VkSurfaceSwapchainLifetimeListener, MsaaLevel, MAX_FRAMES_IN_FLIGHT, VkBuffer};
use ash::prelude::VkResult;
use renderer_ext::renderpass::{VkSpriteRenderPass, VkMeshRenderPass, StaticMeshInstance, PerFrameDataShaderParam, PerObjectDataShaderParam, VkDebugRenderPass, VkBloomRenderPassResources, VkOpaqueRenderPass};
use std::mem::{ManuallyDrop, swap};
use renderer_ext::image_utils::{decode_texture, enqueue_load_images};
use ash::vk;
use renderer_ext::time::{ScopeTimer, TimeState};
use crossbeam_channel::Sender;
use std::ops::Deref;
// use renderer_ext::resource_managers::{
//     SpriteResourceManager, VkMeshResourceManager, ImageResourceManager,
//     MaterialResourceManager,
// };
//use renderer_ext::renderpass::VkMeshRenderPass;
use renderer_ext::pipeline_description::SwapchainSurfaceInfo;
use renderer_ext::pipeline::pipeline::{MaterialAsset, PipelineAsset, MaterialInstanceAsset};
use atelier_assets::loader::handle::Handle;
use renderer_ext::asset_resource::AssetResource;
//use renderer_ext::upload::UploadQueue;
//use renderer_ext::load_handlers::{ImageLoadHandler, MeshLoadHandler, SpriteLoadHandler, MaterialLoadHandler};
use renderer_ext::pipeline::shader::ShaderAsset;
use renderer_ext::pipeline::image::ImageAsset;
//use renderer_ext::pipeline::gltf::{GltfMaterialAsset, MeshAsset};
//use renderer_ext::pipeline::sprite::SpriteAsset;
use atelier_assets::core::asset_uuid;
use atelier_assets::loader::LoadStatus;
use atelier_assets::loader::handle::AssetHandle;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;
use renderer_ext::resource_managers::{ResourceManager, DynDescriptorSet, DynMaterialInstance, MeshInfo};
use renderer_ext::pipeline::gltf::{MeshAsset, GltfMaterialAsset, GltfMaterialData, GltfMaterialDataShaderParam};
use renderer_ext::pipeline::buffer::BufferAsset;
use renderer_ext::renderpass::debug_renderpass::DebugDraw3DResource;
use renderer_ext::renderpass::VkBloomExtractRenderPass;
use renderer_ext::renderpass::VkBloomBlurRenderPass;
use renderer_ext::renderpass::VkBloomCombineRenderPass;
use renderer_ext::features::sprite::{SpriteRenderNodeSet, SpriteRenderFeature, create_sprite_extract_job};
use renderer_base::visibility::{StaticVisibilityNodeSet, DynamicVisibilityNodeSet};
use renderer_base::{RenderRegistryBuilder, RenderPhaseMaskBuilder, RenderPhaseMask, RenderRegistry, RenderViewSet, AllRenderNodes, FramePacketBuilder, ExtractJobSet};
use renderer_ext::phases::draw_opaque::DrawOpaqueRenderPhase;
use renderer_ext::phases::draw_transparent::DrawTransparentRenderPhase;
use legion::prelude::*;
use renderer_ext::{RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContextFactory};
use renderer_ext::RenderJobWriteContext;
use renderer_shell_vulkan::cleanup::{VkCombinedDropSink, VkResourceDropSinkChannel};

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
    //renderer: &mut GameRenderer,
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

pub struct GameRenderer {
    //time_state: TimeState,
    imgui_event_listener: ImguiRenderEventListener,

    //resource_manager: ResourceManager,

    sprite_material: Handle<MaterialAsset>,
    sprite_material_instance: Handle<MaterialInstanceAsset>,
    sprite_custom_material: Option<DynMaterialInstance>,

    debug_material: Handle<MaterialAsset>,
    debug_material_per_frame_data: DynDescriptorSet,
    debug_draw_3d: DebugDraw3DResource,

    // binding 0, contains info about lights
    mesh_material: Handle<MaterialAsset>,
    mesh_material_per_frame_data: DynDescriptorSet,
    meshes: Vec<StaticMeshInstance>,

    bloom_resources: Option<VkBloomRenderPassResources>,

    bloom_extract_material: Handle<MaterialAsset>,
    bloom_extract_material_dyn_set: Option<DynDescriptorSet>,

    bloom_blur_material: Handle<MaterialAsset>,

    bloom_combine_material: Handle<MaterialAsset>,
    bloom_combine_material_dyn_set: Option<DynDescriptorSet>,

    //drop_sink: VkCombinedDropSink,



    //render_registry: RenderRegistry,
    main_camera_render_phase_mask: RenderPhaseMask,
    //
    // sprite_render_nodes: SpriteRenderNodeSet,
    //
    // static_visibility_node_set: StaticVisibilityNodeSet,
    // dynamic_visibility_node_set: DynamicVisibilityNodeSet,

    sprite_renderpass: Option<VkSpriteRenderPass>,
    opaque_renderpass: Option<VkOpaqueRenderPass>,
    mesh_renderpass: Option<VkMeshRenderPass>,
    debug_renderpass: Option<VkDebugRenderPass>,
    bloom_extract_renderpass: Option<VkBloomExtractRenderPass>,
    bloom_blur_renderpass: Option<VkBloomBlurRenderPass>,
    bloom_combine_renderpass: Option<VkBloomCombineRenderPass>,
    swapchain_surface_info: Option<SwapchainSurfaceInfo>,
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
        let sprite_material_instance = begin_load_asset::<MaterialInstanceAsset>(
            asset_uuid!("84d66f60-24b2-4eb2-b6ff-8dbc4d69e2c5"),
            &asset_resource,
        );
        let override_image = begin_load_asset::<ImageAsset>(
            asset_uuid!("7c42f3bc-e96b-49f6-961b-5bfc799dee50"),
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
        let mesh_material_instance = begin_load_asset::<MaterialInstanceAsset>(
            asset_uuid!("0dc01376-ebfe-4da4-9b3c-05eaf7c848a1"),
            &asset_resource,
        );
        let mesh = begin_load_asset::<MeshAsset>(
            asset_uuid!("ffc9b240-0a17-4ff4-bb7d-72d13cc6e261"),
            &asset_resource,
        );

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
        let light_mesh = begin_load_asset::<MeshAsset>(
            asset_uuid!("eb44a445-2670-42ba-9faa-5fb4ec4a2242"),
            &asset_resource,
        );

        // axis z-up (blender format)
        let axis_mesh = begin_load_asset::<MeshAsset>(
            asset_uuid!("21ba465c-57f7-47de-9dd5-6b22060eaec3"),
            &asset_resource,
        );

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
            &sprite_material_instance,
            asset_resource,
            &mut resource_manager,
            "sprite_material_instance"
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

        wait_for_asset_to_load(
            device_context,
            &mesh_material_instance,
            asset_resource,
            &mut resource_manager,
            "mesh material instance"
        );

        wait_for_asset_to_load(
            device_context,
            &mesh,
            asset_resource,
            &mut resource_manager,
            "mesh"
        );

        wait_for_asset_to_load(
            device_context,
            &light_mesh,
            asset_resource,
            &mut resource_manager,
            "light mesh"
        );

        wait_for_asset_to_load(
            device_context,
            &axis_mesh,
            asset_resource,
            &mut resource_manager,
            "axis"
        );

        println!("all waits complete");

        let debug_per_frame_layout = resource_manager.get_descriptor_set_info(&debug_material, 0, 0);
        let debug_material_per_frame_data = resource_manager.create_dyn_descriptor_set_uninitialized(&debug_per_frame_layout.descriptor_set_layout_def)?;

        let mesh_per_frame_layout = resource_manager.get_descriptor_set_info(&mesh_material, 0, 0);
        let mesh_material_per_frame_data = resource_manager.create_dyn_descriptor_set_uninitialized(&mesh_per_frame_layout.descriptor_set_layout_def)?;
        //
        // let mesh_per_frame_layout = resource_manager.get_descriptor_set_info(mesh_material, 0, 2);
        //
        // let static_mesh_instance = StaticMeshInstance {
        //     mesh_info: resource_manager.get_mesh_info(mesh),
        //     object_descriptor_set:
        // }

        let mesh_instance = StaticMeshInstance::new(&mut resource_manager, &mesh, &mesh_material, glam::Vec3::new(0.0, 0.0, 0.0))?;
        let light_mesh_instance = StaticMeshInstance::new(&mut resource_manager, &light_mesh, &mesh_material, glam::Vec3::new(3.0, 3.0, 3.0))?;
        let axis_instance = StaticMeshInstance::new(&mut resource_manager, &axis_mesh, &mesh_material, glam::Vec3::new(0.0, 0.0, 0.0))?;

        let meshes = vec![
            mesh_instance,
            light_mesh_instance,
            axis_instance
        ];

        let mut renderer = GameRenderer {
            //time_state: time_state.clone(),
            imgui_event_listener,
            //resource_manager,

            sprite_material,
            sprite_material_instance,
            sprite_custom_material: None,

            debug_material,
            debug_material_per_frame_data,
            debug_draw_3d: DebugDraw3DResource::new(),

            mesh_material,
            mesh_material_per_frame_data,
            meshes,

            bloom_resources: None,

            bloom_extract_material,
            bloom_extract_material_dyn_set: None,

            bloom_blur_material,

            bloom_combine_material,
            bloom_combine_material_dyn_set: None,

            // mesh,
            // mesh_material_instance,
            // mesh_custom_material: None,
            //
            // light_mesh,
            // light_mesh_material_instances: vec![],

            //render_registry,
            main_camera_render_phase_mask,

            //drop_sink: VkCombinedDropSink::new(MAX_FRAMES_IN_FLIGHT as u32),

            // sprite_render_nodes: Default::default(),
            //
            // static_visibility_node_set: Default::default(),
            // dynamic_visibility_node_set: Default::default(),

            swapchain_surface_info: None,
            sprite_renderpass: None,
            opaque_renderpass: None,
            mesh_renderpass: None,
            debug_renderpass: None,
            bloom_extract_renderpass: None,
            bloom_blur_renderpass: None,
            bloom_combine_renderpass: None,
        };

        let image_info = resource_manager.get_image_info(&override_image);

        let extents_width = 900;
        let extents_height = 600;
        let aspect_ration = extents_width as f32 / extents_height as f32;
        let half_width = 400.0;
        let half_height = 400.0 / aspect_ration;
        let proj = renderer_ext::renderpass::sprite_renderpass::orthographic_rh_gl(
            -half_width,
            half_width,
            -half_height,
            half_height,
            -100.0,
            100.0,
        );

        let mut sprite_custom_material = resource_manager
            .create_dyn_material_instance_from_asset(renderer.sprite_material_instance.clone())?;
        sprite_custom_material.set_image(&"texture".to_string(), &image_info.image_view);
        sprite_custom_material.set_buffer_data(&"view_proj".to_string(), &proj);
        sprite_custom_material.flush();

        renderer.sprite_custom_material = Some(sprite_custom_material);

        // let mut mesh_custom_material = renderer
        //     .resource_manager
        //     .create_dyn_material_instance_from_asset(renderer.mesh_material_instance.clone())?;
        //
        // renderer.mesh_custom_material = Some(mesh_custom_material);
        //
        // let light_material_instance_asset = renderer.light_mesh.asset(asset_resource.storage()).unwrap().mesh_parts[0].material_instance.as_ref().unwrap().clone();
        // let light_material_instance = renderer.resource_manager.create_dyn_material_instance_from_asset(light_material_instance_asset)?;
        // renderer.light_mesh_material_instances.push(light_material_instance);


        Ok(renderer)
    }

    // pub fn update_resources(
    //     &mut self,
    //     device_context: &VkDeviceContext,
    // ) {
    //     self.resource_manager.update_resources();
    // }

    // pub fn update_time(
    //     &mut self,
    //     time_state: &TimeState,
    // ) {
    //     self.time_state = time_state.clone();
    // }
}


pub struct SwapchainLifetimeListener<'a> {
    pub resources: &'a Resources,
    pub resource_manager: &'a mut ResourceManager,
    pub render_registry: &'a RenderRegistry,
    pub game_renderer: &'a mut GameRenderer
}

impl<'a> VkSurfaceSwapchainLifetimeListener for SwapchainLifetimeListener<'a> {
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()> {
        //let mut resource_manager_fetch = self.resources.get_mut::<ResourceManager>().unwrap();
        //let resource_manager = &mut *resource_manager_fetch;
        let mut resource_manager = &mut self.resource_manager;

        log::debug!("game renderer swapchain_created called");
        self.game_renderer.imgui_event_listener
            .swapchain_created(device_context, swapchain)?;

        let swapchain_surface_info = SwapchainSurfaceInfo {
            extents: swapchain.swapchain_info.extents,
            msaa_level: swapchain.swapchain_info.msaa_level,
            surface_format: swapchain.swapchain_info.surface_format,
            color_format: swapchain.color_format,
            depth_format: swapchain.depth_format,
        };

        self.game_renderer.swapchain_surface_info = Some(swapchain_surface_info.clone());
        resource_manager.add_swapchain(&swapchain_surface_info);

        log::trace!("Create VkSpriteRenderPass");
        let sprite_pipeline_info = resource_manager.get_pipeline_info(
            &self.game_renderer.sprite_material,
            &swapchain_surface_info,
            0,
        );

        self.game_renderer.sprite_renderpass = Some(VkSpriteRenderPass::new(
            device_context,
            swapchain,
            sprite_pipeline_info,
        )?);

        log::trace!("Create VkOpaqueRenderPass");
        //TODO: We probably want to move to just using a pipeline here and not a specific material
        let opaque_pipeline_info = resource_manager.get_pipeline_info(
            &self.game_renderer.mesh_material,
            &swapchain_surface_info,
            0,
        );

        self.game_renderer.opaque_renderpass = Some(VkOpaqueRenderPass::new(
            device_context,
            swapchain,
            opaque_pipeline_info,
        )?);

        log::trace!("Create VkMeshRenderPass");
        let mesh_pipeline_info = resource_manager.get_pipeline_info(
            &self.game_renderer.mesh_material,
            &swapchain_surface_info,
            0,
        );

        self.game_renderer.mesh_renderpass = Some(VkMeshRenderPass::new(
            device_context,
            swapchain,
            mesh_pipeline_info,
        )?);

        log::trace!("Create VkDebugRenderPass");
        let debug_pipeline_info = resource_manager.get_pipeline_info(
            &self.game_renderer.debug_material,
            &swapchain_surface_info,
            0,
        );

        self.game_renderer.debug_renderpass = Some(VkDebugRenderPass::new(
            device_context,
            swapchain,
            debug_pipeline_info,
        )?);

        log::trace!("Create VkBloomExtractRenderPass");

        self.game_renderer.bloom_resources = Some(VkBloomRenderPassResources::new(
            device_context,
            swapchain,
            resource_manager,
            self.game_renderer.bloom_blur_material.clone()
        )?);

        // HACK HACK HACK - VkBloomRenderPassResources writes descriptors that VkBloomBlurRenderPass
        // immediately uses to record command buffers. Flush now so that the descriptors get
        // initialized
        resource_manager.on_begin_frame();

        let bloom_extract_layout = resource_manager.get_descriptor_set_info(
            &self.game_renderer.bloom_extract_material,
            0,
            0
        );

        let bloom_extract_pipeline_info = resource_manager.get_pipeline_info(
            &self.game_renderer.bloom_extract_material,
            &swapchain_surface_info,
            0,
        );

        self.game_renderer.bloom_extract_renderpass = Some(VkBloomExtractRenderPass::new(
            device_context,
            swapchain,
            bloom_extract_pipeline_info,
            self.game_renderer.bloom_resources.as_ref().unwrap()
        )?);

        let mut bloom_extract_material_dyn_set = resource_manager.create_dyn_descriptor_set_uninitialized(&bloom_extract_layout.descriptor_set_layout_def)?;
        bloom_extract_material_dyn_set.set_image_raw(0, swapchain.color_attachment.resolved_image_view());
        bloom_extract_material_dyn_set.flush();
        self.game_renderer.bloom_extract_material_dyn_set = Some(bloom_extract_material_dyn_set);

        log::trace!("Create VkBloomBlurRenderPass");

        // let bloom_blur_layout = resource_manager.get_descriptor_set_info(
        //     &self.bloom_blur_material,
        //     0,
        //     0
        // );

        let bloom_blur_pipeline_info = resource_manager.get_pipeline_info(
            &self.game_renderer.bloom_blur_material,
            &swapchain_surface_info,
            0,
        );

        self.game_renderer.bloom_blur_renderpass = Some(VkBloomBlurRenderPass::new(
            device_context,
            swapchain,
            bloom_blur_pipeline_info,
            resource_manager,
            self.game_renderer.bloom_resources.as_ref().unwrap()
        )?);

        log::trace!("Create VkBloomCombineRenderPass");

        let bloom_combine_layout = resource_manager.get_descriptor_set_info(
            &self.game_renderer.bloom_combine_material,
            0,
            0
        );

        let bloom_combine_pipeline_info = resource_manager.get_pipeline_info(
            &self.game_renderer.bloom_combine_material,
            &swapchain_surface_info,
            0,
        );

        self.game_renderer.bloom_combine_renderpass = Some(VkBloomCombineRenderPass::new(
            device_context,
            swapchain,
            bloom_combine_pipeline_info,
            self.game_renderer.bloom_resources.as_ref().unwrap()
        )?);

        let mut bloom_combine_material_dyn_set = resource_manager.create_dyn_descriptor_set_uninitialized(&bloom_combine_layout.descriptor_set_layout_def)?;
        bloom_combine_material_dyn_set.set_image_raw(0, self.game_renderer.bloom_resources.as_ref().unwrap().color_image_view);
        bloom_combine_material_dyn_set.set_image_raw(1, self.game_renderer.bloom_resources.as_ref().unwrap().bloom_image_views[0]);
        bloom_combine_material_dyn_set.flush();
        self.game_renderer.bloom_combine_material_dyn_set = Some(bloom_combine_material_dyn_set);

        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(())
    }

    fn swapchain_destroyed(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) {
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
        self.game_renderer.imgui_event_listener
            .swapchain_destroyed(device_context, swapchain);

        self.game_renderer.swapchain_surface_info = None;
    }
}

impl GameRenderer {
    pub fn render(
        &mut self,
        world: &World,
        resources: &Resources,
        resource_manager: &mut ResourceManager,
        render_registry: &RenderRegistry,
        window: &Window,
        device_context: &VkDeviceContext,
        present_index: usize,
    ) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        resource_manager.on_frame_complete();

        // let mut resource_manager_fetch = resources.get_mut::<ResourceManager>().unwrap();
        // let mut resource_manager = &mut *resource_manager_fetch;
        //
        // let mut render_registry_fetch = resources.get_mut::<RenderRegistry>().unwrap();
        // let mut render_registry = &mut *render_registry_fetch;

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

        let view = glam::Mat4::look_at_rh(eye, glam::Vec3::new(0.0, 0.0, 0.0), glam::Vec3::new(0.0, 0.0, 1.0));
        let proj = glam::Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, aspect_ratio, 0.01, 20.0);
        let proj = glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0)) * proj;
        let view_proj = proj * view;

        let render_view_set = RenderViewSet::default();
        let main_view = render_view_set.create_view(
            eye,
            view_proj,
            self.main_camera_render_phase_mask,
            "main".to_string(),
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

        let sprite_render_nodes = resources.get::<SpriteRenderNodeSet>().unwrap();
        let mut all_render_nodes = AllRenderNodes::new();
        all_render_nodes.add_render_nodes(&*sprite_render_nodes);

        let frame_packet_builder = FramePacketBuilder::new(&all_render_nodes);

        // After these jobs end, user calls functions to start jobs that extract data
        frame_packet_builder.add_view(
            &main_view,
            &[
                main_view_static_visibility_result,
                main_view_dynamic_visibility_result,
            ],
        );

        //
        // Gather lighting data
        //
        log::trace!("game renderer render");
        let mut command_buffers = vec![];

        //
        // Push latest light/camera info into the mesh material
        //
        let mut per_frame_data = PerFrameDataShaderParam::default();
        per_frame_data.ambient_light = glam::Vec4::new(0.03, 0.03, 0.03, 1.0);
        per_frame_data.directional_light_count = 0;
        per_frame_data.point_light_count = 2;
        per_frame_data.spot_light_count = 1;


        let light_from = glam::Vec3::new(5.0, 5.0, 5.0);
        let light_from_vs = (view * light_from.extend(1.0)).truncate();
        let light_to = glam::Vec3::new(0.0, 0.0, 0.0);
        let light_to_vs = (view * light_to.extend(1.0)).truncate();
        let light_direction = (light_to - light_from).normalize();
        let light_direction_vs = (light_to_vs - light_from_vs).normalize();
        per_frame_data.directional_lights[0].direction_ws = light_direction;
        per_frame_data.directional_lights[0].direction_vs = light_direction_vs;
        per_frame_data.directional_lights[0].intensity = 5.0;
        per_frame_data.directional_lights[0].color = glam::Vec4::new(1.0, 1.0, 1.0, 1.0);

        self.debug_draw_3d.add_line(light_from, light_to, glam::Vec4::new(1.0, 1.0, 1.0, 1.0));

        let light_position = glam::Vec3::new(-3.0, -3.0, 3.0);
        let light_position_vs = (view * light_position.extend(1.0)).truncate();
        per_frame_data.point_lights[0].position_ws = light_position.into();
        per_frame_data.point_lights[0].position_vs = light_position_vs.into();
        per_frame_data.point_lights[0].range = 25.0;
        per_frame_data.point_lights[0].color = [1.0, 1.0, 1.0, 1.0].into();
        per_frame_data.point_lights[0].intensity = 130.0;

        let light_position = glam::Vec3::new(-3.0, 3.0, 3.0);
        let light_position_vs = (view * light_position.extend(1.0)).truncate();
        per_frame_data.point_lights[1].position_ws = light_position.into();
        per_frame_data.point_lights[1].position_vs = light_position_vs.into();
        per_frame_data.point_lights[1].range = 100.0;
        per_frame_data.point_lights[1].color = [1.0, 1.0, 1.0, 1.0].into();
        per_frame_data.point_lights[1].intensity = 130.0;


        let light_position = glam::Vec3::new(-3.0, 0.1, -1.0);
        let light_position_vs = (view * light_position.extend(1.0)).truncate();
        per_frame_data.point_lights[2].position_ws = light_position.into();
        per_frame_data.point_lights[2].position_vs = light_position_vs.into();
        per_frame_data.point_lights[2].range = 25.0;
        per_frame_data.point_lights[2].color = [1.0, 1.0, 1.0, 1.0].into();
        per_frame_data.point_lights[2].intensity = 250.0;

        let light_from = glam::Vec3::new(-3.0, -3.0, 0.0);
        let light_from_vs = (view * light_from.extend(1.0)).truncate();
        let light_to = glam::Vec3::new(0.0, 0.0, 0.0);
        let light_to_vs = (view * light_to.extend(1.0)).truncate();

        let light_direction = (light_to - light_from).normalize();
        let light_direction_vs = (light_to_vs - light_from_vs).normalize();

        per_frame_data.spot_lights[0].position_ws = light_from.into();
        per_frame_data.spot_lights[0].position_vs = light_from_vs.into();
        per_frame_data.spot_lights[0].direction_ws = light_direction.into();
        per_frame_data.spot_lights[0].direction_vs = light_direction_vs.into();
        per_frame_data.spot_lights[0].spotlight_half_angle = 10.0 * (std::f32::consts::PI / 180.0);
        per_frame_data.spot_lights[0].range = 8.0;
        per_frame_data.spot_lights[0].color = [1.0, 1.0, 1.0, 1.0].into();
        per_frame_data.spot_lights[0].intensity = 1000.0;


        for i in 0..per_frame_data.point_light_count {
            let light = &per_frame_data.point_lights[i as usize];
            self.debug_draw_3d.add_sphere(
                light.position_ws,
                0.25,
                light.color,
                12
            );
        }

        for i in 0..per_frame_data.spot_light_count {
            let light = &per_frame_data.spot_lights[i as usize];
            self.debug_draw_3d.add_cone(
                light.position_ws,
                light.position_ws + (light.range * light.direction_ws),
                light.range * light.spotlight_half_angle.tan(),
                light.color,
                8
            );
        }

        self.mesh_material_per_frame_data.set_buffer_data(0, &per_frame_data);
        self.mesh_material_per_frame_data.flush();

        for mesh in &mut self.meshes {
            mesh.set_view_proj(view, proj);
        }

        self.debug_material_per_frame_data.set_buffer_data(0, &view_proj);
        self.debug_material_per_frame_data.flush();


        //
        // let gpu_context = GpuContext {
        //     device_context: device_context.clone()
        // }

        //
        // Extract Jobs
        //


        let frame_packet = frame_packet_builder.build();
        let extract_job_set = {
            let sprite_pipeline_info = resource_manager.get_pipeline_info(
                &self.sprite_material,
                self.swapchain_surface_info.as_ref().unwrap(),
                0,
            );

            let dyn_pass_material_instance = self.sprite_custom_material.as_ref().unwrap().pass(0);
            let descriptor_set_per_pass = dyn_pass_material_instance
                .descriptor_set_layout(0)
                .descriptor_set()
                .get_raw_for_gpu_read(resource_manager);

            let descriptor_set_per_texture = dyn_pass_material_instance
                .descriptor_set_layout(1)
                .descriptor_set()
                .get_raw_for_gpu_read(resource_manager);

            let mut extract_job_set = ExtractJobSet::new();
            extract_job_set.add_job(create_sprite_extract_job(
                device_context.clone(),
                sprite_pipeline_info,
                &self.sprite_material,
                descriptor_set_per_pass,
            ));
            extract_job_set
        };



        let mut extract_context = RenderJobExtractContext::new(&world, &resources, resource_manager);
        let prepare_job_set = extract_job_set.extract(
            &mut extract_context,
            &frame_packet,
            &[&main_view]
        );

        //let buffer_drop_sink = VkResourceDropSinkChannel::<ManuallyDrop<VkBuffer>>::new();
        let prepare_context = RenderJobPrepareContext::new(resource_manager.create_dyn_resource_allocator_set());

        //
        // Prepare Jobs
        //
        let prepared_render_data = prepare_job_set.prepare(
            &prepare_context,
            &frame_packet,
            &[&main_view],
            render_registry,
        );

        //
        // Update Resources and flush descriptor set changes
        //
        resource_manager.on_begin_frame();

        //
        // Write Jobs - called from within renderpasses for now
        //
        let mut write_context_factory = RenderJobWriteContextFactory::new(
            device_context.clone(),
            prepare_context.dyn_resource_lookups
        );


        // prepared_render_data
        //     .write_view_phase::<DrawOpaqueRenderPhase>(&main_view, &mut write_context);
        // prepared_render_data
        //     .write_view_phase::<DrawTransparentRenderPhase>(&main_view, &mut write_context);

        //
        // Sprite renderpass
        //
        if let Some(sprite_renderpass) = &mut self.sprite_renderpass {
            log::trace!("sprite_renderpass update");
            let dyn_pass_material_instance = self.sprite_custom_material.as_ref().unwrap().pass(0);
            let static_pass_material_instance = resource_manager.get_material_instance_descriptor_sets_for_current_frame(&self.sprite_material_instance, 0);

            //let pass = self.sprite_material_instance.asset.as_ref().unwrap().pass(0);

            // Pass 0 is "global"
            let descriptor_set_per_pass = dyn_pass_material_instance
                .descriptor_set_layout(0)
                .descriptor_set()
                .get_raw_for_gpu_read(resource_manager);

            // Pass 1 is per-object
            let descriptor_set_per_texture = dyn_pass_material_instance
                .descriptor_set_layout(1)
                .descriptor_set()
                .get_raw_for_gpu_read(resource_manager);
            //let descriptor_set_per_texture = static_pass_material_instance.descriptor_sets[1];

            sprite_renderpass.update(
                present_index,
                1.0,
                //&self.sprite_resource_manager,
                descriptor_set_per_pass,
                &[descriptor_set_per_texture],
                time_state,
            )?;

            command_buffers.push(sprite_renderpass.command_buffers[present_index].clone());
        }

        //
        // Opaque renderpass
        //
        if let Some(opaque_renderpass) = &mut self.opaque_renderpass {
            log::trace!("opaque_renderpass update");
            let opaque_pipeline_info = resource_manager.get_pipeline_info(
                &self.mesh_material,
                self.swapchain_surface_info.as_ref().unwrap(),
                0,
            );

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
        // Mesh renderpass
        //
        if let Some(mesh_renderpass) = &mut self.mesh_renderpass {
            log::trace!("mesh_renderpass update");
            let mesh_pipeline_info = resource_manager.get_pipeline_info(
                &self.mesh_material,
                self.swapchain_surface_info.as_ref().unwrap(),
                0,
            );

            let descriptor_set_per_pass = self.mesh_material_per_frame_data.descriptor_set().get_raw_for_gpu_read(resource_manager);

            mesh_renderpass.update(
                &mesh_pipeline_info,
                present_index,
                descriptor_set_per_pass,
                &self.meshes,
                asset_resource,
                resource_manager
            )?;
            command_buffers.push(mesh_renderpass.command_buffers[present_index].clone());
        }

        //
        // Debug Renderpass
        //
        if let Some(debug_renderpass) = &mut self.debug_renderpass {
            log::trace!("debug_renderpass update");
            let debug_pipeline_info = resource_manager.get_pipeline_info(
                &self.debug_material,
                self.swapchain_surface_info.as_ref().unwrap(),
                0,
            );

            let descriptor_set_per_pass = self.debug_material_per_frame_data.descriptor_set().get_raw_for_gpu_read(resource_manager);

            debug_renderpass.update(
                present_index,
                descriptor_set_per_pass,
                self.debug_draw_3d.take_line_lists(),
            )?;
            command_buffers.push(debug_renderpass.command_buffers[present_index].clone());
        }

        //
        // bloom extract
        //
        if let Some(bloom_extract_renderpass) = &mut self.bloom_extract_renderpass {
            log::trace!("bloom_extract_renderpass update");
            let descriptor_set_per_pass = self.bloom_extract_material_dyn_set.as_ref().unwrap().descriptor_set().get_raw_for_gpu_read(resource_manager);

            bloom_extract_renderpass.update(
                present_index,
                descriptor_set_per_pass
            )?;
            command_buffers.push(bloom_extract_renderpass.command_buffers[present_index].clone());
        }

        //
        // bloom blur
        //
        if let Some(bloom_blur_renderpass) = &mut self.bloom_blur_renderpass {
            log::trace!("bloom_blur_renderpass update");
            //let descriptor_set_per_pass0 = self.bloom_resources.as_ref().unwrap().bloom_image_descriptor_sets[0].descriptor_set().get_raw_for_gpu_read(resource_manager);
            //let descriptor_set_per_pass1 = self.bloom_resources.as_ref().unwrap().bloom_image_descriptor_sets[1].descriptor_set().get_raw_for_gpu_read(resource_manager);

            // bloom_blur_renderpass.update(
            //     present_index,
            //     //descriptor_set_per_pass,
            //     resource_manager,
            //     self.bloom_resources.as_ref().unwrap()
            // )?;
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
        if let Some(bloom_combine_renderpass) = &mut self.bloom_combine_renderpass {
            log::trace!("bloom_combine_renderpass update");
            let descriptor_set_per_pass = self.bloom_combine_material_dyn_set.as_ref().unwrap().descriptor_set().get_raw_for_gpu_read(resource_manager);

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
                self.imgui_event_listener
                    .render(window, device_context, present_index)?;
            command_buffers.append(&mut commands);
        }

        self.debug_draw_3d.clear();
        VkResult::Ok(command_buffers)
    }
}

impl Drop for GameRenderer {
    fn drop(&mut self) {
        self.sprite_renderpass = None;
        self.opaque_renderpass = None;
        self.mesh_renderpass = None;
        self.sprite_custom_material = None;
        self.debug_renderpass = None;
        self.bloom_extract_renderpass = None;
        self.bloom_blur_renderpass = None;
        self.bloom_combine_renderpass = None;
        self.meshes.clear();
        //self.mesh_custom_material = None;
        //self.light_mesh_material_instances.clear();
        //self.mesh_renderpass = None;
    }
}
