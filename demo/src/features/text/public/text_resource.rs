use super::super::{TextDrawCommand, TextDrawData};
use crate::assets::font::FontAsset;
use fnv::FnvHashMap;
use rafx::distill::loader::handle::AssetHandle;
use rafx::distill::loader::handle::Handle;
use rafx::distill::loader::LoadHandle;

pub struct AppendText<'a>(&'a mut TextResource, glam::Vec3);

impl<'a> AppendText<'a> {
    pub fn append(
        self,
        text: String,
        font: &Handle<FontAsset>,
        size: f32,
        color: glam::Vec4,
    ) -> AppendText<'a> {
        self.0.do_add_text(text, self.1, font, size, color, true)
    }
}

pub struct TextResource {
    fonts: FnvHashMap<LoadHandle, Handle<FontAsset>>,
    text_draw_commands: Vec<TextDrawCommand>,
}

impl TextResource {
    pub fn new() -> Self {
        TextResource {
            fonts: Default::default(),
            text_draw_commands: Default::default(),
        }
    }

    pub fn add_text(
        &mut self,
        text: String,
        position: glam::Vec3,
        font: &Handle<FontAsset>,
        size: f32,
        color: glam::Vec4,
    ) -> AppendText {
        self.do_add_text(text, position, font, size, color, false)
    }

    pub(super) fn do_add_text(
        &mut self,
        text: String,
        position: glam::Vec3,
        font: &Handle<FontAsset>,
        size: f32,
        color: glam::Vec4,
        is_append: bool,
    ) -> AppendText {
        let font = self.fonts.entry(font.load_handle()).or_insert(font.clone());

        self.text_draw_commands.push(TextDrawCommand {
            text,
            position,
            font: font.load_handle(),
            size,
            color,
            is_append,
        });

        AppendText(self, position)
    }

    // Returns the draw data, leaving this object in an empty state
    pub(in super::super) fn take_text_draw_data(&mut self) -> TextDrawData {
        TextDrawData {
            fonts: std::mem::take(&mut self.fonts),
            text_draw_commands: std::mem::take(&mut self.text_draw_commands),
        }
    }

    // Recommended to call every frame to ensure that this doesn't grow unbounded
    pub fn clear(&mut self) {
        self.take_text_draw_data();
    }
}
