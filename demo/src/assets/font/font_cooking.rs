use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::BuildHasherDefault;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FontTextureCharacterMeta {
    pub character: char,
    pub rect: FontTextureCharacterRect,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FontTextureCharacterRect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontTexture {
    #[serde(with = "serde_bytes")]
    pub image_data: Vec<u8>,
    pub image_width: u32,
    pub image_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontTextureWithMeta {
    pub font_texture: FontTexture,
    pub characters: Vec<FontTextureCharacterMeta>,
}

pub struct FontTextureWithLookup {
    pub font_texture: FontTexture,
    pub character_lookup: FnvHashMap<char, FontTextureCharacterRect>,
}

impl FontTextureWithLookup {
    pub fn to_font_texture_with_meta(self) -> FontTextureWithMeta {
        let mut characters: Vec<_> = self
            .character_lookup
            .iter()
            .map(|(&c, &r)| FontTextureCharacterMeta {
                character: c,
                rect: r,
            })
            .collect();
        characters.sort_by_key(|x| x.character);

        FontTextureWithMeta {
            font_texture: self.font_texture,
            characters,
        }
    }

    pub fn from_font_texture_with_meta(font_texture_with_meta: FontTextureWithMeta) -> Self {
        let mut character_lookup = FnvHashMap::with_capacity_and_hasher(
            font_texture_with_meta.characters.len(),
            BuildHasherDefault::default(),
        );

        for c in font_texture_with_meta.characters {
            character_lookup.insert(c.character, c.rect);
        }

        FontTextureWithLookup {
            character_lookup,
            font_texture: font_texture_with_meta.font_texture,
        }
    }
}

pub fn create_font_texture_with_ranges(
    font_data: &[u8],
    character_ranges_to_include: &[(u32, u32)],
    size: f32,
    margin: u32,
) -> Option<FontTextureWithMeta> {
    // let character_ranges_to_include = vec![
    //     (32, 128),
    //     //(0x4e00, 0x5FCC)
    // ];

    let mut characters_to_include = vec![];

    //
    // Iterate codepoints in the font and find the characters within the given ranges
    //
    let face = ttf_parser::Face::from_slice(font_data, 0).unwrap();

    for subtable in face.character_mapping_subtables() {
        subtable.codepoints(|codepoint| {
            for range in character_ranges_to_include {
                if codepoint >= range.0 && codepoint <= range.1 {
                    if let Some(_) = subtable.glyph_index(codepoint) {
                        characters_to_include.push(std::char::from_u32(codepoint).unwrap());
                    }
                }
            }
        });
    }

    //
    // Rasterize the characters to a bunch of tiny u8 bitmaps. Also create a list of regions for
    // rectangle_pack to place
    //
    let settings = fontdue::FontSettings {
        scale: size,
        ..fontdue::FontSettings::default()
    };
    let font = fontdue::Font::from_bytes(font_data, settings).unwrap();

    create_font_texture_with_characters(&font, characters_to_include.iter(), size, margin)
}

pub fn create_font_texture_with_characters<'a, IterT: Iterator<Item = &'a char>>(
    font: &fontdue::Font,
    characters: IterT,
    size: f32,
    margin: u32,
) -> Option<FontTextureWithMeta> {
    let mut rasterized_data = FnvHashMap::default();
    let mut rects_to_place = rectangle_pack::GroupedRectsToPlace::<char, ()>::new();

    for &c in characters {
        let (metrics, data) = font.rasterize(c, size);
        rects_to_place.push_rect(
            c,
            None,
            rectangle_pack::RectToInsert::new(
                metrics.width as u32 + (margin * 2),
                metrics.height as u32 + (margin * 2),
                1,
            ),
        );
        rasterized_data.insert(c, (metrics, data));
    }

    //
    // Try packing in progressively larger textures (128x128, 256x256, 512x512, ... 4096x4096)
    //
    let mut texture_dimensions = 128;
    let result = loop {
        let mut target_bins = BTreeMap::new();
        target_bins.insert(
            0,
            rectangle_pack::TargetBin::new(texture_dimensions, texture_dimensions, 1),
        );

        let pack_result = rectangle_pack::pack_rects(
            &rects_to_place,
            target_bins,
            &rectangle_pack::volume_heuristic,
            &rectangle_pack::contains_smallest_box,
        );

        if let Ok(rectangle_placements) = pack_result {
            break Some((texture_dimensions, rectangle_placements));
        }

        texture_dimensions *= 2;
        if texture_dimensions > 4096 {
            break None;
        }
    };

    if result.is_none() {
        eprintln!("Too much data, requires more than a 4k texture to store");
        return None;
    }

    //
    // Create the texture and copy the per-character bitmaps into it
    //
    let (texture_dimensions, placement) = result.unwrap();
    let mut image_data = vec![0; texture_dimensions as usize * texture_dimensions as usize];
    let mut character_meta = Vec::with_capacity(placement.packed_locations().len());

    for (&c, (_, location)) in placement.packed_locations() {
        let (metrics, src_data) = &rasterized_data[&c];
        assert_eq!(metrics.width as u32, location.width() - (2 * margin));
        assert_eq!(metrics.height as u32, location.height() - (2 * margin));

        for src_x in 0..metrics.width {
            for src_y in 0..metrics.height {
                let src_i = metrics.width * src_y + src_x;

                let dst_x = location.x() + src_x as u32 + margin;
                let dst_y = location.y() + src_y as u32 + margin;
                let dst_i = texture_dimensions * dst_y + dst_x;

                image_data[dst_i as usize] = src_data[src_i as usize];
            }
        }

        character_meta.push(FontTextureCharacterMeta {
            character: c,
            rect: FontTextureCharacterRect {
                x: (location.x() + margin) as u16,
                y: (location.y() + margin) as u16,
                w: metrics.width as u16,
                h: metrics.height as u16,
            },
        });
    }

    Some(FontTextureWithMeta {
        font_texture: FontTexture {
            image_data,
            image_width: texture_dimensions,
            image_height: texture_dimensions,
        },
        characters: character_meta,
    })
}
