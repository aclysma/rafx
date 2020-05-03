use renderer_shell_vulkan::{VkSwapchain, VkDeviceContext};
use renderer_shell_vulkan::VkDevice;
use renderer_shell_vulkan::Window;
use ash::vk;
use super::{VkImGuiRenderPass, VkImGuiRenderPassFontAtlas};
use ash::prelude::VkResult;

pub struct ImguiRenderEventListener {
    // The renderpass, none if it's not created
    imgui_renderpass: Option<VkImGuiRenderPass>,

    // A copy of the font atlas, used so we can recreate the renderpass as needed
    font_atlas: VkImGuiRenderPassFontAtlas,
}

impl ImguiRenderEventListener {
    pub fn new(font_atlas: VkImGuiRenderPassFontAtlas) -> Self {
        ImguiRenderEventListener {
            imgui_renderpass: None,
            font_atlas,
        }
    }
}

impl renderer_shell_vulkan::VkSurfaceEventListener for ImguiRenderEventListener {
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

    fn swapchain_destroyed(&mut self) {
        self.imgui_renderpass = None;
    }

    fn render(
        &mut self,
        window: &dyn Window,
        device_context: &VkDeviceContext,
        present_index: usize,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let draw_data = unsafe { imgui::sys::igGetDrawData() };
        if draw_data.is_null() {
            log::warn!("no draw data available");
            return Err(vk::Result::ERROR_INITIALIZATION_FAILED);
        }

        let draw_data = unsafe { &*(draw_data as *mut imgui::DrawData) };

        let renderpass = self.imgui_renderpass.as_mut().unwrap();

        renderpass.update(
            Some(&draw_data),
            present_index as usize,
            window.scale_factor(),
        )?;

        Ok(vec![renderpass.command_buffers[present_index].clone()])
    }
}
