use crate::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener};
//use crate::ResourceManager;
use renderer_shell_vulkan::{VkDevice, VkSwapchain, RendererEventListener, RendererBuilder, CreateRendererError, Renderer, Window, VkTransferUpload, VkTransferUploadState, VkImage, VkDeviceContext};
use ash::prelude::VkResult;
//use crate::features::sprite_renderpass_push_constant::VkSpriteRenderPass;
use crate::renderpass::sprite::{VkSpriteRenderPass/*, LoadingSprite*/};
use crate::renderpass::sprite::VkSpriteResourceManager;
use std::mem::{swap, ManuallyDrop};
use crate::image_utils::{decode_texture, load_images, enqueue_load_images};
use ash::vk;
use crate::time::ScopeTimer;
use std::sync::mpsc::Sender;

pub struct GameRenderer {
    imgui_event_listener: ImguiRenderEventListener,

    sprite_resource_manager: Option<VkSpriteResourceManager>,
    sprite_renderpass: Option<VkSpriteRenderPass>,
}

impl GameRenderer {
    pub fn new(window: &dyn Window, imgui_font_atlas: VkImGuiRenderPassFontAtlas) -> Self {

        let imgui_event_listener = ImguiRenderEventListener::new(imgui_font_atlas);

        GameRenderer {
            imgui_event_listener,

            sprite_resource_manager: None,
            sprite_renderpass: None,
        }
    }

    pub fn sprite_resource_manager(&self) -> Option<&VkSpriteResourceManager> {
        self.sprite_resource_manager.as_ref()
    }

    pub fn sprite_resource_manager_mut(&mut self) -> Option<&mut VkSpriteResourceManager> {
        self.sprite_resource_manager.as_mut()
    }
}


impl RendererEventListener for GameRenderer {
    fn swapchain_created(&mut self, device: &VkDevice, swapchain: &VkSwapchain) -> VkResult<()> {
        log::debug!("game renderer swapchain_created called");
        self.imgui_event_listener.swapchain_created(device, swapchain)?;

        log::debug!("Create VkSpriteResourceManager");
        self.sprite_resource_manager = Some(VkSpriteResourceManager::new(device, swapchain.swapchain_info.clone())?);
        log::debug!("Create VkSpriteRenderPass");
        self.sprite_renderpass = Some(VkSpriteRenderPass::new(device, swapchain, self.sprite_resource_manager.as_ref().unwrap())?);
        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(())
    }

    fn swapchain_destroyed(&mut self) {
        log::debug!("game renderer swapchain destroyed");

        self.sprite_renderpass = None;
        self.sprite_resource_manager = None;
        self.imgui_event_listener.swapchain_destroyed();
    }

    fn render(&mut self, window: &Window, device: &VkDevice, present_index: usize) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        log::trace!("game renderer render");
        let mut command_buffers = vec![];

        if let Some(sprite_resource_manager) = &mut self.sprite_resource_manager {
            sprite_resource_manager.update(device);

            if let Some(sprite_renderpass) = &mut self.sprite_renderpass {
                log::trace!("sprite_renderpass update");
                sprite_renderpass.update(&device.memory_properties, present_index, 1.0, sprite_resource_manager)?;
                command_buffers.push(sprite_renderpass.command_buffers[present_index].clone());
            }
        }

        {
            log::trace!("imgui_event_listener update");
            let mut commands = self.imgui_event_listener.render(window, device, present_index)?;
            command_buffers.append(&mut commands);
        }

        VkResult::Ok(command_buffers)
    }
}

pub struct GameRendererWithShell {
    game_renderer: GameRenderer,

    // Handles setting up device/instance/swapchain and windowing integration
    shell: Renderer
}

impl GameRendererWithShell {
    pub fn new(window: &dyn Window, imgui_font_atlas: VkImGuiRenderPassFontAtlas) -> Result<GameRendererWithShell, CreateRendererError> {
        let mut game_renderer = GameRenderer::new(window, imgui_font_atlas);

        let shell = RendererBuilder::new()
            .use_vulkan_debug_layer(true)
            //.use_vulkan_debug_layer(false)
            .prefer_mailbox_present_mode()
            .build(window, Some(&mut game_renderer))?;

        Ok(GameRendererWithShell {
            game_renderer,
            shell
        })
    }

    pub fn draw(&mut self, window: &dyn Window) -> VkResult<()> {
        self.shell.draw(window, Some(&mut self.game_renderer))
    }

    pub fn dump_stats(&mut self) {
        if let Ok(stats) = self.shell.device_mut().allocator().calculate_stats() {
            println!("{:#?}", stats);
        } else {
            log::error!("failed to calculate stats");
        }
    }

    pub fn shell(&self) -> &Renderer {
        &self.shell
    }

    pub fn sprite_resource_manager(&self) -> Option<&VkSpriteResourceManager> {
        self.game_renderer.sprite_resource_manager()
    }

    pub fn sprite_resource_manager_mut(&mut self) -> Option<&mut VkSpriteResourceManager> {
        self.game_renderer.sprite_resource_manager_mut()
    }
}

impl Drop for GameRendererWithShell {
    fn drop(&mut self) {
        self.shell.tear_down(Some(&mut self.game_renderer));
    }
}