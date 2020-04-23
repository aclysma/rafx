use crate::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener};
use crate::ResourceManager;
use renderer_shell_vulkan::{VkDevice, VkSwapchain, RendererEventListener, RendererBuilder, CreateRendererError, Renderer, Window};
use ash::prelude::VkResult;
use crate::features::sprite_renderpass_push_constant::VkSpriteRenderPass;
use std::mem::swap;

pub struct GameRenderer {
    // Handles uploading resources to GPU
    resource_manager: ResourceManager,

    imgui_event_listener: ImguiRenderEventListener,

    //imgui_font_atlas: VkImGuiRenderPassFontAtlas,
    //imgui_renderpass: Option<VkImGuiRenderPass>,
    sprite_renderpass: Option<VkSpriteRenderPass>

}

impl GameRenderer {
    pub fn new(window: &dyn Window, imgui_font_atlas: VkImGuiRenderPassFontAtlas) -> Self {
        let mut resource_manager = ResourceManager::new();

        let imgui_event_listener = ImguiRenderEventListener::new(imgui_font_atlas);

        GameRenderer {
            //imgui_font_atlas,
            resource_manager,
            imgui_event_listener,
            //imgui_renderpass: None,
            sprite_renderpass: None
        }
    }
}


impl RendererEventListener for GameRenderer {
    fn swapchain_created(&mut self, device: &VkDevice, swapchain: &VkSwapchain) -> VkResult<()> {
        log::debug!("game renderer swapchain created");
        self.resource_manager.swapchain_created(device, swapchain)?;
        self.imgui_event_listener.swapchain_created(device, swapchain)?;

        //self.imgui_renderpass = Some(VkImGuiRenderPass::new(device, swapchain, &self.imgui_font_atlas)?);
        self.sprite_renderpass = Some(VkSpriteRenderPass::new(device, swapchain)?);

        VkResult::Ok(())
    }

    fn swapchain_destroyed(&mut self) {
        log::debug!("game renderer swapchain destroyed");
        self.resource_manager.swapchain_destroyed();
        self.imgui_event_listener.swapchain_destroyed();

        // Dropping these will clean them up
        //self.imgui_renderpass = None;
        self.sprite_renderpass = None;

    }

    fn render(&mut self, window: &Window, device: &VkDevice, present_index: usize) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        log::trace!("game renderer render");
        let mut command_buffers = vec![];

        {
            let mut commands = self.resource_manager.render(window, device, present_index)?;
            command_buffers.append(&mut commands);
        }

        if let Some(sprite_renderpass) = &mut self.sprite_renderpass {
            sprite_renderpass.update(&device.memory_properties, present_index, 1.0)?;
            command_buffers.push(sprite_renderpass.command_buffers[present_index].clone());
        }

        {
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
}

impl Drop for GameRendererWithShell {
    fn drop(&mut self) {
        self.shell.tear_down(Some(&mut self.game_renderer));
    }
}