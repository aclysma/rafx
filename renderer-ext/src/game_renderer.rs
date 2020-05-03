use crate::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener};
//use crate::ResourceManager;
use renderer_shell_vulkan::{VkDevice, VkSwapchain, VkSurfaceEventListener, VkSurface, Window, VkTransferUpload, VkTransferUploadState, VkImage, VkDeviceContext, VkContextBuilder, VkCreateContextError, VkContext};
use ash::prelude::VkResult;
//use crate::features::sprite_renderpass_push_constant::VkSpriteRenderPass;
use crate::renderpass::sprite::{VkSpriteRenderPass/*, LoadingSprite*/};
use crate::renderpass::sprite::VkSpriteResourceManager;
use std::mem::{swap, ManuallyDrop};
use crate::image_utils::{decode_texture, load_images, enqueue_load_images};
use ash::vk;
use crate::time::{ScopeTimer, TimeState};
use std::sync::mpsc::Sender;
use std::ops::Deref;

pub struct GameRenderer {
    time_state: TimeState,
    imgui_event_listener: ImguiRenderEventListener,

    sprite_resource_manager: VkSpriteResourceManager,
    sprite_renderpass: Option<VkSpriteRenderPass>,
}

impl GameRenderer {
    pub fn new(
        window: &dyn Window,
        device_context: &VkDeviceContext,
        imgui_font_atlas: VkImGuiRenderPassFontAtlas,
        time_state: &TimeState
    ) -> VkResult<Self> {

        let imgui_event_listener = ImguiRenderEventListener::new(imgui_font_atlas);
        let sprite_resource_manager = VkSpriteResourceManager::new(device_context, renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32)?;

        Ok(GameRenderer {
            time_state: time_state.clone(),
            imgui_event_listener,
            sprite_resource_manager,
            sprite_renderpass: None,
        })
    }

    pub fn update_time(&mut self, time_state: &TimeState) {
        self.time_state = time_state.clone();
    }

    pub fn sprite_resource_manager(&self) -> &VkSpriteResourceManager {
        &self.sprite_resource_manager
    }

    pub fn sprite_resource_manager_mut(&mut self) -> &mut VkSpriteResourceManager {
        &mut self.sprite_resource_manager
    }
}


impl VkSurfaceEventListener for GameRenderer {
    fn swapchain_created(&mut self, device_context: &VkDeviceContext, swapchain: &VkSwapchain) -> VkResult<()> {
        log::debug!("game renderer swapchain_created called");
        self.imgui_event_listener.swapchain_created(device_context, swapchain)?;

        log::debug!("Create VkSpriteRenderPass");
        self.sprite_renderpass = Some(VkSpriteRenderPass::new(device_context, swapchain, &self.sprite_resource_manager)?);
        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(())
    }

    fn swapchain_destroyed(&mut self) {
        log::debug!("game renderer swapchain destroyed");

        self.sprite_renderpass = None;
        self.imgui_event_listener.swapchain_destroyed();
    }

    fn render(
        &mut self,
        window: &Window,
        device_context: &VkDeviceContext,
        present_index: usize
    ) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        log::trace!("game renderer render");
        let mut command_buffers = vec![];

        self.sprite_resource_manager.update();

        if let Some(sprite_renderpass) = &mut self.sprite_renderpass {
            log::trace!("sprite_renderpass update");
            sprite_renderpass.update(present_index, 1.0, &self.sprite_resource_manager, &self.time_state)?;
            command_buffers.push(sprite_renderpass.command_buffers[present_index].clone());
        }

        {
            log::trace!("imgui_event_listener update");
            let mut commands = self.imgui_event_listener.render(window, device_context, present_index)?;
            command_buffers.append(&mut commands);
        }

        VkResult::Ok(command_buffers)
    }
}

pub struct GameRendererWithContext {
    // Handles setting up device/instance
    context: VkContext,
    game_renderer: ManuallyDrop<GameRenderer>,
    surface: ManuallyDrop<VkSurface>
}

impl GameRendererWithContext {
    pub fn new(
        window: &dyn Window,
        imgui_font_atlas: VkImGuiRenderPassFontAtlas,
        time_state: &TimeState
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
            time_state
        )?;

        let surface = VkSurface::new(&context, window, Some(&mut game_renderer))?;

        Ok(GameRendererWithContext {
            context,
            game_renderer: ManuallyDrop::new(game_renderer),
            surface: ManuallyDrop::new(surface)
        })
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

    pub fn context(&self) -> &VkContext {
        &self.context
    }

    pub fn sprite_resource_manager(&self) -> &VkSpriteResourceManager {
        self.game_renderer.sprite_resource_manager()
    }

    pub fn sprite_resource_manager_mut(&mut self) -> &mut VkSpriteResourceManager {
        self.game_renderer.sprite_resource_manager_mut()
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