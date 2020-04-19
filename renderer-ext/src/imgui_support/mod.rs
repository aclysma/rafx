use ash::vk;
use ash::prelude::VkResult;

mod imgui_renderpass;
pub use imgui_renderpass::VkImGuiRenderPass;

mod sdl2_imgui_manager;
pub use sdl2_imgui_manager::Sdl2ImguiManager;
pub use sdl2_imgui_manager::init_imgui_manager;

mod imgui_manager;
pub use imgui_manager::ImguiManager;

mod renderer_event_listener;
pub use renderer_event_listener::ImguiRenderEventListener;

pub use imgui;

pub struct VkImGuiRenderPassFontAtlas {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl VkImGuiRenderPassFontAtlas {
    pub fn new(texture: &imgui::FontAtlasTexture) -> Self {
        VkImGuiRenderPassFontAtlas {
            width: texture.width,
            height: texture.height,
            data: texture.data.to_vec(),
        }
    }
}