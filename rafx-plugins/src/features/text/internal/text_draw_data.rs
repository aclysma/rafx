use crate::assets::font::FontAsset;
use fnv::FnvHashMap;
use hydrate_base::handle::Handle;
use hydrate_base::LoadHandle;

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
