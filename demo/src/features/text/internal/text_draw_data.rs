use crate::assets::font::FontAsset;
use distill::loader::handle::Handle;
use distill::loader::LoadHandle;
use fnv::FnvHashMap;

pub struct TextDrawData {
    pub fonts: FnvHashMap<LoadHandle, Handle<FontAsset>>,
    pub text_draw_commands: Vec<TextDrawCommand>,
}

#[derive(Debug)]
pub struct TextDrawCommand {
    pub text: String,
    pub position: glam::Vec3,
    pub font: LoadHandle,
    pub size: f32,
    pub color: glam::Vec4,
    pub is_append: bool,
}
