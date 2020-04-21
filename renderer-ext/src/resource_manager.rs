use renderer_shell_vulkan::{RendererEventListener, VkDevice, VkSwapchain, Window};
use ash::prelude::VkResult;

pub struct ResourceManager {

}

impl ResourceManager {
    pub fn new() -> Self {
        ResourceManager {}
    }
}

impl RendererEventListener for ResourceManager {
    fn swapchain_created(&mut self, device: &VkDevice, swapchain: &VkSwapchain) -> VkResult<()> {
        log::info!("resource manager swapchain created");
        VkResult::Ok(())
    }

    fn swapchain_destroyed(&mut self) {
        log::info!("resource manager swapchain destroyed");
    }

    fn render(&mut self, window: &Window, device: &VkDevice, present_index: usize) -> VkResult<Vec<ash::vk::CommandBuffer>> {
        log::trace!("resource manager render");
        VkResult::Ok(vec![])
    }
}

