use std::sync::Arc;

pub struct EguiDrawData {
    pub vertices: Vec<egui::epaint::Vertex>,
    pub indices: Vec<u16>,
    pub clipped_draw_calls: Vec<EguiClippedDrawCalls>,
    pub font_atlas: Arc<egui::Texture>,
    pub pixels_per_point: f32,
}

pub struct EguiDrawCall {
    pub texture_id: egui::TextureId,
    pub vertex_offset: usize,
    pub index_offset: usize,
    pub index_count: usize,
}

pub struct EguiClippedDrawCalls {
    pub clip_rect: egui::math::Rect,
    pub draw_calls: Vec<EguiDrawCall>,
}

impl EguiDrawData {
    pub fn try_create_new(
        clipped_meshes: Vec<egui::epaint::ClippedMesh>,
        font_atlas: Arc<egui::Texture>,
        pixels_per_point: f32,
    ) -> Option<Self> {
        let mut vertices = Vec::default();
        let mut indices = Vec::default();
        let mut all_clipped_draw_calls = Vec::default();

        for clipped_mesh in clipped_meshes {
            let rect = clipped_mesh.0;
            let meshes = clipped_mesh.1.split_to_u16();

            let mut clipped_draw_calls = EguiClippedDrawCalls {
                clip_rect: rect,
                draw_calls: vec![],
            };

            for mut mesh in meshes {
                let vertex_offset = vertices.len();
                let index_offset = indices.len();
                clipped_draw_calls.draw_calls.push(EguiDrawCall {
                    texture_id: mesh.texture_id,
                    vertex_offset,
                    index_offset,
                    index_count: mesh.indices.len(),
                });
                vertices.append(&mut mesh.vertices);
                indices.append(&mut mesh.indices);
            }

            all_clipped_draw_calls.push(clipped_draw_calls);
        }

        if vertices.len() > 1 {
            Some(EguiDrawData {
                vertices,
                indices,
                clipped_draw_calls: all_clipped_draw_calls,
                font_atlas,
                pixels_per_point,
            })
        } else {
            None
        }
    }

    pub fn draw_lists(&self) -> &[EguiClippedDrawCalls] {
        &self.clipped_draw_calls
    }
}
