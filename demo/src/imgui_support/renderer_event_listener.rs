use renderer::vulkan::{VkSwapchain, VkDeviceContext};
use renderer::vulkan::VkDevice;
use renderer::vulkan::Window;
use ash::vk;
use super::{VkImGuiRenderPass, ImGuiFontAtlas};
use ash::prelude::VkResult;
use crate::imgui_support::ImGuiDrawData;

pub struct ImguiRenderEventListener {
    // The renderpass, none if it's not created
    imgui_renderpass: Option<VkImGuiRenderPass>,

    // A copy of the font atlas, used so we can recreate the renderpass as needed
    font_atlas: ImGuiFontAtlas,
}

impl ImguiRenderEventListener {
    pub fn new(font_atlas: ImGuiFontAtlas) -> Self {
        ImguiRenderEventListener {
            imgui_renderpass: None,
            font_atlas,
        }
    }
}

impl renderer::vulkan::VkSurfaceSwapchainLifetimeListener for ImguiRenderEventListener {
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()> {
        self.imgui_renderpass = Some(VkImGuiRenderPass::new(
            device_context,
            swapchain,
            &self.font_atlas,
        )?);
        Ok(())
    }

    fn swapchain_destroyed(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) {
        self.imgui_renderpass = None;
    }
}

impl ImguiRenderEventListener {
    pub fn render(
        &mut self,
        present_index: usize,
        draw_data: Option<&ImGuiDrawData>,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let renderpass = self.imgui_renderpass.as_mut().unwrap();

        renderpass.update(draw_data, present_index as usize)?;

        Ok(vec![renderpass.command_buffers[present_index].clone()])
    }
}
