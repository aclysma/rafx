use super::{TextDrawCallMeta, TextDrawCommand, TextVertex};
use crate::assets::font::{font_cooking, FontAsset};
use fnv::{FnvBuildHasher, FnvHashMap, FnvHashSet};
use fontdue::layout::{GlyphPosition, LayoutSettings, TextStyle};
use hydrate_base::LoadHandle;
use rafx::api::{
    RafxBufferDef, RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxTextureDef,
};
use rafx::framework::{BufferResource, DynResourceAllocatorSet, ImageViewResource, ResourceArc};

pub struct FontAtlas {
    image: ResourceArc<ImageViewResource>,
    character_lookup: FnvHashMap<char, FontTextureCharacterRectUv>,
    font_asset: FontAsset,
}

#[derive(Debug, Clone, Copy)]
pub struct FontTextureCharacterRectUv {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Default)]
pub struct TextDrawCallBufferData {
    pub vertices: Vec<TextVertex>,
    pub indices: Vec<u16>,
}

pub struct TextDrawVerticesResult {
    pub draw_call_buffer_data: Vec<TextDrawCallBufferData>,
    pub draw_call_metas: Vec<TextDrawCallMeta>,
    pub font_atlas_images: Vec<ResourceArc<ImageViewResource>>,
    pub image_updates: Vec<TextImageUpdate>,
}

pub struct TextImageUpdate {
    pub upload_buffer: ResourceArc<BufferResource>,
    pub upload_image: ResourceArc<ImageViewResource>,
}

#[derive(Default)]
pub struct FontAtlasCache {
    fonts: FnvHashMap<LoadHandle, FontAtlas>,
}

impl FontAtlasCache {
    pub fn generate_vertices(
        &mut self,
        text_draw_commands: &[TextDrawCommand],
        font_assets: &FnvHashMap<LoadHandle, FontAsset>,
        dyn_resource_allocator: &DynResourceAllocatorSet,
    ) -> RafxResult<TextDrawVerticesResult> {
        let image_updates =
            self.update_cache(text_draw_commands, font_assets, dyn_resource_allocator)?;

        let mut font_index_lookup =
            FnvHashMap::with_capacity_and_hasher(self.fonts.len(), FnvBuildHasher::default());
        let mut fonts = Vec::with_capacity(self.fonts.len());
        let mut font_atlas_images = Vec::with_capacity(self.fonts.len());
        let mut font_atlases = Vec::with_capacity(self.fonts.len());
        for (&load_handle, atlas) in &self.fonts {
            font_index_lookup.insert(load_handle, fonts.len());
            fonts.push(&atlas.font_asset.inner.font);
            font_atlas_images.push(atlas.image.clone());
            font_atlases.push(atlas)
        }

        let mut draw_call_metas = vec![];
        let mut draw_call_buffer_data = vec![];

        fn append_glyphs(
            draw_call_metas: &mut Vec<TextDrawCallMeta>,
            draw_call_buffer_data: &mut Vec<TextDrawCallBufferData>,
            glyphs: &[GlyphPosition<glam::Vec4>],
            font_atlases: &[&FontAtlas],
            z_position: f32,
        ) {
            if glyphs.is_empty() {
                return;
            }

            let required_vertices = 4 * glyphs.len();
            if draw_call_buffer_data.is_empty()
                || draw_call_buffer_data.last().unwrap().vertices.len()
                    >= (std::u16::MAX as usize - required_vertices)
            {
                draw_call_buffer_data.push(TextDrawCallBufferData::default());
            }

            let buffer_index = draw_call_buffer_data.len() as u32 - 1;
            let draw_call_buffers = draw_call_buffer_data.last_mut().unwrap();

            let mut draw_call_meta = TextDrawCallMeta {
                buffer_index,
                index_offset: draw_call_buffers.indices.len() as u32,
                font_descriptor_index: 0,
                index_count: 0,
                z_position,
            };

            for glyph in glyphs {
                let color = glyph.user_data.into();
                let atlas = font_atlases[glyph.key.font_index];

                let texture_rect = atlas.character_lookup[&glyph.key.c];

                if draw_call_meta.index_count == 0 {
                    draw_call_meta.font_descriptor_index = glyph.key.font_index as u32;
                } else if draw_call_meta.font_descriptor_index != glyph.key.font_index as u32 {
                    draw_call_metas.push(draw_call_meta);
                    draw_call_meta = TextDrawCallMeta {
                        buffer_index,
                        index_offset: draw_call_buffers.indices.len() as u32,
                        font_descriptor_index: 0,
                        index_count: 0,
                        z_position,
                    };
                }

                draw_call_meta.index_count += 6;

                let base_index = draw_call_buffers.vertices.len() as u16;
                draw_call_buffers.vertices.push(TextVertex {
                    position: [
                        glyph.x + glyph.width as f32,
                        glyph.y + glyph.height as f32,
                        z_position,
                    ],
                    uv: [texture_rect.right, texture_rect.bottom],
                    color,
                });
                draw_call_buffers.vertices.push(TextVertex {
                    position: [glyph.x, glyph.y + glyph.height as f32, z_position],
                    uv: [texture_rect.left, texture_rect.bottom],
                    color,
                });
                draw_call_buffers.vertices.push(TextVertex {
                    position: [glyph.x + glyph.width as f32, glyph.y, z_position],
                    uv: [texture_rect.right, texture_rect.top],
                    color,
                });
                draw_call_buffers.vertices.push(TextVertex {
                    position: [glyph.x, glyph.y, z_position],
                    uv: [texture_rect.left, texture_rect.top],
                    color,
                });

                draw_call_buffers.indices.push(base_index + 0);
                draw_call_buffers.indices.push(base_index + 1);
                draw_call_buffers.indices.push(base_index + 2);
                draw_call_buffers.indices.push(base_index + 2);
                draw_call_buffers.indices.push(base_index + 1);
                draw_call_buffers.indices.push(base_index + 3);
            }

            if draw_call_meta.index_count > 0 {
                draw_call_metas.push(draw_call_meta);
            }
        }

        let mut layout = fontdue::layout::Layout::<glam::Vec4>::new(
            fontdue::layout::CoordinateSystem::PositiveYDown,
        );

        for draw in text_draw_commands {
            if !draw.is_append {
                append_glyphs(
                    &mut draw_call_metas,
                    &mut draw_call_buffer_data,
                    layout.glyphs(),
                    &font_atlases,
                    draw.position.z,
                );
                // do something with them

                layout.reset(&LayoutSettings {
                    x: draw.position.x,
                    y: draw.position.y,
                    ..Default::default()
                });
            }

            let font_index = font_index_lookup[&draw.font];
            layout.append(
                &fonts,
                &TextStyle::with_user_data(&draw.text, draw.size, font_index, draw.color),
            );
        }

        // may push a list of no glyphs, not really a problem
        if let Some(draw) = text_draw_commands.last() {
            append_glyphs(
                &mut draw_call_metas,
                &mut draw_call_buffer_data,
                layout.glyphs(),
                &font_atlases,
                draw.position.z,
            );
        }

        Ok(TextDrawVerticesResult {
            draw_call_metas,
            draw_call_buffer_data,
            font_atlas_images,
            image_updates,
        })
    }

    pub fn update_cache(
        &mut self,
        text_draw_commands: &[TextDrawCommand],
        font_assets: &FnvHashMap<LoadHandle, FontAsset>,
        dyn_resource_allocator: &DynResourceAllocatorSet,
    ) -> RafxResult<Vec<TextImageUpdate>> {
        let mut image_updates = Vec::default();

        // Accumulate all the characters
        let mut atlas_updates = FnvHashMap::<LoadHandle, FnvHashSet<char>>::default();

        //
        // Check all the text we are about to draw and add any missing characters to atlas_updates
        //
        for draw_command in text_draw_commands {
            let missing_chars_for_font = atlas_updates.entry(draw_command.font).or_default();
            if let Some(font_atlas) = self.fonts.get(&draw_command.font) {
                // Only insert the characters into missing_chars_for_font if existing atlas doesn't have them
                for c in draw_command.text.chars() {
                    if !font_atlas.character_lookup.contains_key(&c) {
                        missing_chars_for_font.insert(c);
                    }
                }
            } else {
                // no existing atlas, will need to create it
                let missing_chars = atlas_updates.entry(draw_command.font).or_default();
                for c in draw_command.text.chars() {
                    missing_chars.insert(c);
                }
            }
        }

        //
        // Rebuild any atlases that are missing characters
        //
        for (font, mut missing_chars) in atlas_updates {
            if missing_chars.is_empty() {
                continue;
            }

            let font_asset = &font_assets[&font];

            if let Some(existing_atlas) = self.fonts.get(&font) {
                for &c in existing_atlas.character_lookup.keys() {
                    missing_chars.insert(c);
                }
            }

            log::debug!("rebuild font atlas with {} chars", missing_chars.len());

            let font_texture = font_cooking::create_font_texture_with_characters(
                &font_asset.inner.font,
                missing_chars.iter(),
                font_asset.inner.scale,
                2,
            )
            .unwrap();

            let extents = RafxExtents3D {
                width: font_texture.font_texture.image_width,
                height: font_texture.font_texture.image_height,
                depth: 1,
            };

            let buffer = dyn_resource_allocator.device_context.create_buffer(
                &RafxBufferDef::for_staging_buffer_data(
                    &font_texture.font_texture.image_data,
                    RafxResourceType::BUFFER,
                ),
            )?;
            buffer
                .copy_to_host_visible_buffer(&font_texture.font_texture.image_data)
                .unwrap();
            let buffer = dyn_resource_allocator.insert_buffer(buffer);

            //DX12TODO: Fix mipmap code to work with this
            let mip_count = if dyn_resource_allocator.device_context.is_dx12() {
                1
            } else {
                rafx::api::extra::mipmaps::mip_level_max_count_for_image_size(
                    extents.width,
                    extents.height,
                )
            };

            let texture =
                dyn_resource_allocator
                    .device_context
                    .create_texture(&RafxTextureDef {
                        extents,
                        format: RafxFormat::R8_UNORM,
                        mip_count,
                        ..Default::default()
                    })?;
            if dyn_resource_allocator
                .device_context
                .device_info()
                .debug_names_enabled
            {
                texture.set_debug_name(format!("Font Atlas Texture {:?}", font));
            }

            let image = dyn_resource_allocator.insert_texture(texture);
            let image_view = dyn_resource_allocator.insert_image_view(&image, None)?;

            image_updates.push(TextImageUpdate {
                upload_buffer: buffer,
                upload_image: image_view.clone(),
            });

            let mut character_lookup = FnvHashMap::with_capacity_and_hasher(
                font_texture.characters.len(),
                FnvBuildHasher::default(),
            );
            for character in font_texture.characters {
                let old = character_lookup.insert(
                    character.character,
                    FontTextureCharacterRectUv {
                        left: character.rect.x as f32 / extents.width as f32,
                        right: (character.rect.x + character.rect.w) as f32 / extents.width as f32,
                        top: character.rect.y as f32 / extents.height as f32,
                        bottom: (character.rect.y + character.rect.h) as f32
                            / extents.height as f32,
                    },
                );
                assert!(old.is_none());
            }

            self.fonts.insert(
                font,
                FontAtlas {
                    character_lookup,
                    image: image_view,
                    font_asset: font_asset.clone(),
                },
            );
        }

        Ok(image_updates)
    }
}
