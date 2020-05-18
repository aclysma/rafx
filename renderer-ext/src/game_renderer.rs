use crate::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener};
use renderer_shell_vulkan::{
    VkDevice, VkSwapchain, VkSurfaceEventListener, VkSurface, Window, VkTransferUpload,
    VkTransferUploadState, VkImage, VkDeviceContext, VkContextBuilder, VkCreateContextError,
    VkContext,
};
use ash::prelude::VkResult;
use crate::renderpass::{VkSpriteRenderPass};
use std::mem::{ManuallyDrop};
use crate::image_utils::{decode_texture, enqueue_load_images};
use ash::vk;
use crate::time::{ScopeTimer, TimeState};
use crossbeam_channel::Sender;
use std::ops::Deref;
use crate::resource_managers::{
    SpriteResourceManager, VkMeshResourceManager, ImageResourceManager,
    MaterialResourceManager, /*, ShaderResourceManager*/
};
use crate::renderpass::VkMeshRenderPass;
//use crate::pipeline_manager::{PipelineManager, ShaderLoadHandler, PipelineLoadHandler, PipelineResourceManager};
use crate::pipeline_description::SwapchainSurfaceInfo;
use crate::pipeline::pipeline::{MaterialAsset2, PipelineAsset2};
use atelier_assets::loader::handle::Handle;
use crate::asset_resource::AssetResource;
use crate::upload::UploadQueue;
use crate::load_handlers::{ImageLoadHandler, MeshLoadHandler, SpriteLoadHandler, MaterialLoadHandler};
use crate::pipeline::shader::ShaderAsset;
use crate::pipeline::image::ImageAsset;
use crate::pipeline::gltf::{MaterialAsset, MeshAsset};
use crate::pipeline::sprite::SpriteAsset;
use atelier_assets::core::asset_uuid;
use atelier_assets::loader::LoadStatus;
use atelier_assets::loader::handle::AssetHandle;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;
use crate::resource_managers::ResourceManager;


fn load_asset<T>(
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

    //shader_resource_manager: ShaderResourceManager,
    image_resource_manager: ImageResourceManager,
    material_resource_manager: MaterialResourceManager,
    sprite_resource_manager: SpriteResourceManager,
    mesh_resource_manager: VkMeshResourceManager,

    upload_queue: UploadQueue,

    resource_manager: ResourceManager,

    sprite_material: Handle<MaterialAsset2>,
    sprite_renderpass: Option<VkSpriteRenderPass>,

    mesh_renderpass: Option<VkMeshRenderPass>,
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

        let mut upload_queue = UploadQueue::new(device_context);

        // let shader_resource_manager = ShaderResourceManager::new(
        //     device_context,
        //     renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
        // )?;
        let image_resource_manager = ImageResourceManager::new(
            device_context,
            renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
        )?;
        let material_resource_manager = MaterialResourceManager::new(
            device_context,
            renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
        )?;
        let sprite_resource_manager = SpriteResourceManager::new(
            device_context,
            renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
            &image_resource_manager,
        )?;
        let mesh_resource_manager = VkMeshResourceManager::new(
            device_context,
            renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
        )?;

        // let pipeline_manager = PipelineManager::new(
        //     device_context
        // );
        let resource_manager = ResourceManager::new(device_context);

        asset_resource.add_storage_with_load_handler::<ShaderAsset, _>(Box::new(
            resource_manager.create_shader_load_handler(),
        ));
        asset_resource.add_storage_with_load_handler::<PipelineAsset2, _>(Box::new(
            resource_manager.create_pipeline2_load_handler(),
        ));
        asset_resource.add_storage_with_load_handler::<MaterialAsset2, _>(Box::new(
            resource_manager.create_material_load_handler(),
        ));
        //asset_resource.add_storage::<ShaderAsset>();
        asset_resource.add_storage_with_load_handler::<ImageAsset, ImageLoadHandler>(Box::new(
            ImageLoadHandler::new(
                upload_queue.pending_image_tx().clone(),
                image_resource_manager.image_update_tx().clone(),
                sprite_resource_manager.sprite_update_tx().clone(),
            ),
        ));
        asset_resource.add_storage_with_load_handler::<MaterialAsset, MaterialLoadHandler>(
            Box::new(MaterialLoadHandler::new(
                material_resource_manager.material_update_tx().clone(),
            )),
        );
        asset_resource.add_storage_with_load_handler::<MeshAsset, MeshLoadHandler>(Box::new(
            MeshLoadHandler::new(
                upload_queue.pending_buffer_tx().clone(),
                mesh_resource_manager.mesh_update_tx().clone(),
            ),
        ));
        asset_resource.add_storage_with_load_handler::<SpriteAsset, SpriteLoadHandler>(Box::new(
            SpriteLoadHandler::new(sprite_resource_manager.sprite_update_tx().clone()),
        ));

        let sprite_material = load_asset::<MaterialAsset2>(
            asset_uuid!("f8c4897e-7c1d-4736-93b7-f2deda158ec7"),
            &asset_resource
        );

        let mut renderer = GameRenderer {
            time_state: time_state.clone(),
            imgui_event_listener,
            //shader_resource_manager,
            image_resource_manager,
            material_resource_manager,
            sprite_resource_manager,
            mesh_resource_manager,
            upload_queue,
            //pipeline_manager,
            resource_manager,
            //sprite_renderpass_pipeline,
            sprite_material,
            sprite_renderpass: None,
            mesh_renderpass: None,
        };

        // wait_for_asset_to_load(
        //     device_context,
        //     &renderer.sprite_renderpass_pipeline.clone(),
        //     asset_resource,
        //     &mut renderer,
        // );
        wait_for_asset_to_load(
            device_context,
            &renderer.sprite_material.clone(),
            asset_resource,
            &mut renderer,
        );
        //wait_for_asset_to_load(&pipeline_variant, asset_resource, &mut renderer);

        // let sprite_renderpass_asset = renderer.sprite_renderpass_pipeline.asset(asset_resource.loader()).unwrap();
        // renderer.pipeline_manager.load_graphics_pipeline(renderer.sprite_renderpass_pipeline.load_handle(), sprite_renderpass_asset);

        Ok(renderer)
    }

    pub fn update_resources(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        //self.pipeline_manager.update();
        self.resource_manager.update();
        self.upload_queue.update(device_context);

        //self.shader_resource_manager.update();
        self.image_resource_manager.update();
        self.material_resource_manager
            .update(&self.image_resource_manager);
        self.sprite_resource_manager
            .update(&self.image_resource_manager);
        self.mesh_resource_manager
            .update(&self.material_resource_manager);
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

        //self.pipeline_manager.add_swapchain(&swapchain_surface_info);
        self.resource_manager.add_swapchain(&swapchain_surface_info);

        //self.pipeline_manager.
        //let pipeline_info = self.pipeline_manager.get_pipeline_info(&self.sprite_renderpass_pipeline, &swapchain_surface_info);

        // let sprite_pipeline_info = self
        //     .resource_manager
        //     .get_pipeline_info(&self.sprite_renderpass_pipeline, &swapchain_surface_info);

        let sprite_pipeline_info = self
            .resource_manager
            .get_pipeline_info(&self.sprite_material, &swapchain_surface_info, 0);

        // Get the pipeline,

        log::debug!("Create VkSpriteRenderPass");
        self.sprite_renderpass = Some(VkSpriteRenderPass::new(
            device_context,
            swapchain,
            sprite_pipeline_info,
            //&mut self.pipeline_manager,
            &self.sprite_resource_manager,
            &swapchain_surface_info,
        )?);
        log::debug!("Create VkMeshRenderPass");
        self.mesh_renderpass = Some(VkMeshRenderPass::new(
            device_context,
            swapchain,
            &self.mesh_resource_manager,
            &self.sprite_resource_manager,
        )?);
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

        self.sprite_renderpass = None;
        self.mesh_renderpass = None;
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

        if let Some(sprite_renderpass) = &mut self.sprite_renderpass {
            log::trace!("sprite_renderpass update");
            sprite_renderpass.update(
                present_index,
                1.0,
                &self.sprite_resource_manager,
                &self.time_state,
            )?;
            command_buffers.push(sprite_renderpass.command_buffers[present_index].clone());
        }

        if let Some(mesh_renderpass) = &mut self.mesh_renderpass {
            log::trace!("mesh_renderpass update");
            mesh_renderpass.update(
                present_index,
                1.0,
                &self.mesh_resource_manager,
                &self.sprite_resource_manager,
                &self.time_state,
            )?;
            command_buffers.push(mesh_renderpass.command_buffers[present_index].clone());
        }

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
