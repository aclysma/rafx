mod sdl2_imgui_manager;
pub use sdl2_imgui_manager::Sdl2ImguiManager;
pub use sdl2_imgui_manager::init_imgui_manager;

mod imgui_manager;
pub use imgui_manager::ImguiManager;

pub use imgui;
use imgui::{DrawCmdParams, DrawCmd};

pub struct ImGuiFontAtlas {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImGuiFontAtlas {
    pub fn new(texture: &imgui::FontAtlasTexture) -> Self {
        ImGuiFontAtlas {
            width: texture.width,
            height: texture.height,
            data: texture.data.to_vec(),
        }
    }
}

pub enum ImGuiDrawCmd {
    Elements {
        count: usize,
        cmd_params: DrawCmdParams,
    },
    ResetRenderState,
    //RawCallback is not supported
}

impl From<imgui::DrawCmd> for ImGuiDrawCmd {
    fn from(draw_cmd: DrawCmd) -> Self {
        match draw_cmd {
            imgui::DrawCmd::Elements { count, cmd_params } => {
                ImGuiDrawCmd::Elements { count, cmd_params }
            }
            imgui::DrawCmd::ResetRenderState => ImGuiDrawCmd::ResetRenderState,
            _ => unimplemented!(),
        }
    }
}

pub struct ImGuiDrawList {
    vertex_buffer: Vec<imgui::DrawVert>,
    index_buffer: Vec<imgui::DrawIdx>,
    command_buffer: Vec<ImGuiDrawCmd>,
}

impl ImGuiDrawList {
    pub fn vertex_buffer(&self) -> &[imgui::DrawVert] {
        &self.vertex_buffer
    }
    pub fn index_buffer(&self) -> &[imgui::DrawIdx] {
        &self.index_buffer
    }
    pub fn commands(&self) -> &[ImGuiDrawCmd] {
        &self.command_buffer
    }
}

pub struct ImGuiDrawData {
    draw_lists: Vec<ImGuiDrawList>,
    pub total_idx_count: i32,
    pub total_vtx_count: i32,
    pub display_pos: [f32; 2],
    pub display_size: [f32; 2],
    pub framebuffer_scale: [f32; 2],
}

impl ImGuiDrawData {
    pub fn new(draw_data: &imgui::DrawData) -> Self {
        let draw_lists: Vec<_> = draw_data
            .draw_lists()
            .map(|draw_list| {
                let vertex_buffer: Vec<_> = draw_list.vtx_buffer().iter().copied().collect();
                let index_buffer: Vec<_> = draw_list.idx_buffer().iter().copied().collect();
                let command_buffer: Vec<_> = draw_list.commands().map(|x| x.into()).collect();

                ImGuiDrawList {
                    vertex_buffer,
                    index_buffer,
                    command_buffer,
                }
            })
            .collect();

        ImGuiDrawData {
            draw_lists,
            total_idx_count: draw_data.total_idx_count,
            total_vtx_count: draw_data.total_vtx_count,
            display_pos: draw_data.display_pos,
            display_size: draw_data.display_size,
            framebuffer_scale: draw_data.framebuffer_scale,
        }
    }

    pub fn draw_lists(&self) -> &[ImGuiDrawList] {
        &self.draw_lists
    }
}
