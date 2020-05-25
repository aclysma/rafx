use crate::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener};
use renderer_shell_vulkan::{
    VkDevice, VkSwapchain, VkSurfaceEventListener, VkSurface, Window, VkTransferUpload,
    VkTransferUploadState, VkImage, VkDeviceContext, VkContextBuilder, VkCreateContextError,
    VkContext,
};
use ash::prelude::VkResult;
use crate::renderpass::{VkSpriteRenderPass, VkMeshRenderPass};
use std::mem::{ManuallyDrop};
use crate::image_utils::{decode_texture, enqueue_load_images};
use ash::vk;
use crate::time::{ScopeTimer, TimeState};
use crossbeam_channel::Sender;
use std::ops::Deref;
// use crate::resource_managers::{
//     SpriteResourceManager, VkMeshResourceManager, ImageResourceManager,
//     MaterialResourceManager,
// };
//use crate::renderpass::VkMeshRenderPass;
use crate::pipeline_description::SwapchainSurfaceInfo;
use crate::pipeline::pipeline::{MaterialAsset, PipelineAsset, MaterialInstanceAsset};
use atelier_assets::loader::handle::Handle;
use crate::asset_resource::AssetResource;
//use crate::upload::UploadQueue;
//use crate::load_handlers::{ImageLoadHandler, MeshLoadHandler, SpriteLoadHandler, MaterialLoadHandler};
use crate::pipeline::shader::ShaderAsset;
use crate::pipeline::image::ImageAsset;
//use crate::pipeline::gltf::{GltfMaterialAsset, MeshAsset};
//use crate::pipeline::sprite::SpriteAsset;
use atelier_assets::core::asset_uuid;
use atelier_assets::loader::LoadStatus;
use atelier_assets::loader::handle::AssetHandle;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;
use crate::resource_managers::{ResourceManager, DynDescriptorSet, DynMaterialInstance};
use crate::pipeline::gltf::{MeshAsset, GltfMaterialAsset};

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
    renderer: &mut GameRenderer,
) {
    loop {
        asset_resource.update();
        renderer.update_resources(device_context);
        match asset_handle.load_status(asset_resource.loader()) {
            LoadStatus::NotRequested => {
                unreachable!();
            }
            LoadStatus::Loading => {
                log::info!("blocked waiting for asset to load");
                std::thread::sleep(std::time::Duration::from_millis(100));
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
    time_state: TimeState,
    imgui_event_listener: ImguiRenderEventListener,

    resource_manager: ResourceManager,

    sprite_material: Handle<MaterialAsset>,
    sprite_renderpass: Option<VkSpriteRenderPass>,
    sprite_material_instance: Handle<MaterialInstanceAsset>,
    //mesh_renderpass: Option<VkMeshRenderPass>,
    sprite_custom_material: Option<DynMaterialInstance>,
    //mesh: Handle<MeshAsset>,
    //mesh_material: Handle<MaterialAsset>,
    //mesh_material_instance: Handle<MaterialInstanceAsset>,
}

impl GameRenderer {
    pub fn new(
        window: &dyn Window,
        device_context: &VkDeviceContext,
        imgui_font_atlas: VkImGuiRenderPassFontAtlas,
        time_state: &TimeState,
        asset_resource: &mut AssetResource,
    ) -> VkResult<Self> {
        let imgui_event_listener = ImguiRenderEventListener::new(imgui_font_atlas);

        let mut resource_manager = ResourceManager::new(device_context);

        asset_resource.add_storage_with_load_handler::<ShaderAsset, _>(Box::new(
            resource_manager.create_shader_load_handler(),
        ));
        asset_resource.add_storage_with_load_handler::<PipelineAsset, _>(Box::new(
            resource_manager.create_pipeline_load_handler(),
        ));
        asset_resource.add_storage_with_load_handler::<MaterialAsset, _>(Box::new(
            resource_manager.create_material_load_handler(),
        ));
        asset_resource.add_storage_with_load_handler::<MaterialInstanceAsset, _>(Box::new(
            resource_manager.create_material_instance_load_handler(),
        ));
        asset_resource.add_storage_with_load_handler::<ImageAsset, _>(Box::new(
            resource_manager.create_image_load_handler(),
        ));
        //asset_resource.add_storage::<GltfMaterialAsset>();
        //asset_resource.add_storage::<MeshAsset>();
        // asset_resource.add_storage::<SpriteAsset>();

        let sprite_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("f8c4897e-7c1d-4736-93b7-f2deda158ec7"),
            &asset_resource,
        );
        let sprite_material_instance = begin_load_asset::<MaterialInstanceAsset>(
            asset_uuid!("84d66f60-24b2-4eb2-b6ff-8dbc4d69e2c5"),
            &asset_resource,
        );
        let override_image = begin_load_asset::<ImageAsset>(
            asset_uuid!("b7753a66-1b26-4152-ad61-93584f4442aa"),
            &asset_resource,
        );
        /*
        let mesh_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("267e0388-2611-441c-9c78-2d39d1bd3cf1"),
            &asset_resource,
        );
        let mesh_material_instance = begin_load_asset::<MaterialInstanceAsset>(
            asset_uuid!("4101d8ef-7a46-4ab8-970c-2c18a91aff06"),
            &asset_resource,
        );
        */
        // let mesh = begin_load_asset::<MeshAsset>(
        //     asset_uuid!("6b33207a-241c-41ba-9149-3e678557a45c"),
        //     &asset_resource,
        // );

        let mut renderer = GameRenderer {
            time_state: time_state.clone(),
            imgui_event_listener,
            resource_manager,
            sprite_material,
            sprite_material_instance,
            sprite_renderpass: None,
            //mesh_renderpass: None,
            sprite_custom_material: None,
            //mesh,
            //mesh_material,
            //mesh_material_instance
        };

        wait_for_asset_to_load(
            device_context,
            &renderer.sprite_material.clone(),
            asset_resource,
            &mut renderer,
        );

        wait_for_asset_to_load(
            device_context,
            &renderer.sprite_material_instance.clone(),
            asset_resource,
            &mut renderer,
        );
/*
        wait_for_asset_to_load(
            device_context,
            &renderer.mesh_material.clone(),
            asset_resource,
            &mut renderer,
        );

        wait_for_asset_to_load(
            device_context,
            &renderer.mesh_material_instance.clone(),
            asset_resource,
            &mut renderer,
        );
*/
        let image_info = renderer.resource_manager.get_image_info(&override_image);

        let extents_width = 900;
        let extents_height = 600;
        let fov = extents_width as f32 / extents_height as f32;
        let half_width = 400.0;
        let half_height = 400.0 / fov;
        let proj = crate::renderpass::sprite_renderpass::orthographic_rh_gl(
            -half_width,
            half_width,
            -half_height,
            half_height,
            -100.0,
            100.0,
        );

        let mut sprite_custom_material = renderer
            .resource_manager
            .create_dyn_material_instance_from_asset(renderer.sprite_material_instance.clone())?;
        sprite_custom_material.set_image(&"texture".to_string(), &image_info.image_view);
        sprite_custom_material.set_buffer_data(&"view_proj".to_string(), &proj);
        sprite_custom_material.flush();

        renderer.sprite_custom_material = Some(sprite_custom_material);

        Ok(renderer)
    }

    pub fn update_resources(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        self.resource_manager.update_resources();
    }

    pub fn update_time(
        &mut self,
        time_state: &TimeState,
    ) {
        self.time_state = time_state.clone();
    }
}

impl VkSurfaceEventListener for GameRenderer {
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()> {
        log::debug!("game renderer swapchain_created called");
        self.imgui_event_listener
            .swapchain_created(device_context, swapchain)?;

        let swapchain_surface_info = SwapchainSurfaceInfo {
            surface_format: swapchain.swapchain_info.surface_format,
            extents: swapchain.swapchain_info.extents,
        };

        self.resource_manager.add_swapchain(&swapchain_surface_info);

        let sprite_pipeline_info = self.resource_manager.get_pipeline_info(
            &self.sprite_material,
            &swapchain_surface_info,
            0,
        );

        // let mesh_pipeline_info = self.resource_manager.get_pipeline_info(
        //     &self.mesh_material,
        //     &swapchain_surface_info,
        //     0,
        // );

        // Get the pipeline,

        log::trace!("Create VkSpriteRenderPass");
        self.sprite_renderpass = Some(VkSpriteRenderPass::new(
            device_context,
            swapchain,
            sprite_pipeline_info,
        )?);
        log::trace!("Create VkMeshRenderPass");
        // self.mesh_renderpass = Some(VkMeshRenderPass::new(
        //     device_context,
        //     swapchain,
        //     mesh_pipeline_info,
        // )?);
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
            surface_format: swapchain.swapchain_info.surface_format,
            extents: swapchain.swapchain_info.extents,
        };

        self.resource_manager
            .remove_swapchain(&swapchain_surface_info);
        self.imgui_event_listener
            .swapchain_destroyed(device_context, swapchain);
    }

    fn render(
        &mut self,
        window: &Window,
        device_context: &VkDeviceContext,
        present_index: usize,
    ) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        log::trace!("game renderer render");
        let mut command_buffers = vec![];

        //std::thread::sleep(std::time::Duration::from_millis(100));

        // Flush descriptor set changes
        self.resource_manager.on_begin_frame();

        let pass = self.sprite_custom_material.as_ref().unwrap().pass(0);

        // Pass 0 is "global"
        let descriptor_set_per_pass = pass
            .descriptor_set_layout(0)
            .descriptor_set()
            .get_raw_for_gpu_read(&self.resource_manager);

        // Pass 1 is per-object
        let descriptor_set_per_texture = pass.descriptor_set_layout(1).descriptor_set();
        let descriptor_sets_per_texture =
            vec![descriptor_set_per_texture.get_raw_for_gpu_read(&self.resource_manager)];

        //let descriptor_set_per_pass = vec![descriptor_set.get_raw(&self.resource_manager)];

        if let Some(sprite_renderpass) = &mut self.sprite_renderpass {
            log::trace!("sprite_renderpass update");
            sprite_renderpass.update(
                present_index,
                1.0,
                //&self.sprite_resource_manager,
                descriptor_set_per_pass,
                &descriptor_sets_per_texture,
                &self.time_state,
            )?;
            command_buffers.push(sprite_renderpass.command_buffers[present_index].clone());
        }

        // if let Some(mesh_renderpass) = &mut self.mesh_renderpass {
        //     log::trace!("mesh_renderpass update");
        //     mesh_renderpass.update(
        //         present_index,
        //         1.0,
        //         &self.mesh_resource_manager,
        //         &self.sprite_resource_manager,
        //         &self.time_state,
        //     )?;
        //     command_buffers.push(mesh_renderpass.command_buffers[present_index].clone());
        // }

        {
            log::trace!("imgui_event_listener update");
            let mut commands =
                self.imgui_event_listener
                    .render(window, device_context, present_index)?;
            command_buffers.append(&mut commands);
        }

        VkResult::Ok(command_buffers)
    }
}

impl Drop for GameRenderer {
    fn drop(&mut self) {
        self.sprite_renderpass = None;
        //self.mesh_renderpass = None;
        self.sprite_custom_material = None;
        //self.mesh_renderpass = None;
    }
}

// The context is separate from the renderer so that we can create it before creating the swapchain.
// This is required because the API design is for VkSurface to be passed temporary borrows to the
// renderer rather than owning the renderer
pub struct GameRendererWithContext {
    // Handles setting up device/instance
    context: VkContext,
    game_renderer: ManuallyDrop<GameRenderer>,
    surface: ManuallyDrop<VkSurface>,
}

impl GameRendererWithContext {
    pub fn new(
        window: &dyn Window,
        imgui_font_atlas: VkImGuiRenderPassFontAtlas,
        time_state: &TimeState,
        asset_resource: &mut AssetResource,
    ) -> Result<GameRendererWithContext, VkCreateContextError> {
        let context = VkContextBuilder::new()
            .use_vulkan_debug_layer(true)
            //.use_vulkan_debug_layer(false)
            .prefer_mailbox_present_mode()
            .build(window)?;

        let mut game_renderer = GameRenderer::new(
            window,
            &context.device().device_context,
            imgui_font_atlas,
            time_state,
            asset_resource,
        )?;

        let surface = VkSurface::new(&context, window, Some(&mut game_renderer))?;

        Ok(GameRendererWithContext {
            context,
            game_renderer: ManuallyDrop::new(game_renderer),
            surface: ManuallyDrop::new(surface),
        })
    }

    pub fn update_resources(&mut self) {
        self.game_renderer
            .update_resources(self.context.device_context());
    }

    pub fn draw(
        &mut self,
        window: &dyn Window,
        time_state: &TimeState,
    ) -> VkResult<()> {
        self.game_renderer.update_time(time_state);
        self.surface.draw(window, Some(&mut *self.game_renderer))
    }

    pub fn dump_stats(&mut self) {
        if let Ok(stats) = self.context.device().allocator().calculate_stats() {
            println!("{:#?}", stats);
        } else {
            log::error!("failed to calculate stats");
        }
    }
}

impl Drop for GameRendererWithContext {
    fn drop(&mut self) {
        self.surface.tear_down(Some(&mut *self.game_renderer));
        unsafe {
            ManuallyDrop::drop(&mut self.surface);
            ManuallyDrop::drop(&mut self.game_renderer);
        }
    }
}
