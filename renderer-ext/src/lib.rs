pub mod features;
pub mod phases;
pub mod renderpass;

pub mod imgui_support;

mod game_renderer;
pub use game_renderer::GameRenderer;
pub use game_renderer::GameRendererWithContext;

pub mod time;

use legion::prelude::*;
use glam::Vec3;
use features::sprite::SpriteRenderNodeHandle;
use renderer_base::visibility::DynamicAabbVisibilityNodeHandle;
use crate::image_utils::DecodedTexture;

pub mod asset_resource;
pub mod asset_storage;
pub mod image_importer;
pub mod gltf_importer;
pub mod image_utils;

#[derive(Copy, Clone)]
pub struct PositionComponent {
    pub position: Vec3,
}

#[derive(Clone)]
pub struct SpriteComponent {
    pub sprite_handle: SpriteRenderNodeHandle,
    pub visibility_handle: DynamicAabbVisibilityNodeHandle,
    pub alpha: f32,
}

pub struct ExtractSource {
    world: &'static World,
    resources: &'static Resources,
}

impl ExtractSource {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
    ) -> Self {
        unsafe {
            ExtractSource {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
            }
        }
    }
}

pub struct CommandWriter {}

impl CommandWriter {}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}


pub fn test_gltf() {
    //let path = std::path::Path::new("assets/blender/cubic.gltf");
    let path = std::path::Path::new("assets/blender/3objects.gltf");
    let (doc, buffers, images) = gltf::import(path).unwrap();

    let mut decoded_textures = vec![];

    for image in doc.images() {
        let image_data = &images[image.index()];

        // Convert it to standard RGBA format
        use gltf::image::Format;
        use image::buffer::ConvertBuffer;
        let converted_image : image::RgbaImage = match image_data.format {
            Format::R8 => {
                image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R8G8 => {
                image::ImageBuffer::<image::LumaA<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R8G8B8 => {
                image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R8G8B8A8 => {
                image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::B8G8R8 => {
                image::ImageBuffer::<image::Bgr<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::B8G8R8A8 => {
                image::ImageBuffer::<image::Bgra<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R16 => {
                unimplemented!();
            },
            Format::R16G16 => {
                unimplemented!();
            },
            Format::R16G16B16 => {
                unimplemented!();
            },
            Format::R16G16B16A16 => {
                unimplemented!();
            },
        };

        let decoded_texture = DecodedTexture {
            data: converted_image.to_vec(),
            width: image_data.width,
            height: image_data.height
        };

        assert!(image.index() == decoded_textures.len());
        println!("texture {:?} {} {} {} {}", image.name(), image.index(), decoded_texture.width, decoded_texture.height, decoded_texture.data.len());
        decoded_textures.push(decoded_texture);
    }

    for material in doc.materials() {
        let pbr_metallic_roughness = material.pbr_metallic_roughness();
        let base_color = pbr_metallic_roughness.base_color_factor();
        let base_color_texture_index = pbr_metallic_roughness.base_color_texture().map(|base_texture| {
            base_texture.texture().index()
        });

        println!("material name: {:?} base: {:?} texture: {:?}", material.name(), base_color, base_color_texture_index);
    }

    for mesh in doc.meshes() {
        println!("mesh name: {:?}", mesh.name());

        for primitive in mesh.primitives() {
            let indices = primitive.indices().unwrap();

            match (indices.data_type(), indices.dimensions()) {
                (DataType::U16, Dimensions::Scalar) => {
                    // let iter = Iter::<[f32; 3]>::new(accessor, get_buffer_data);
                    // for item in iter {
                    //     println!("{:?}", item);
                    // }
                }
                _ => {
                    unimplemented!();
                },
            }


            let mut positions = None;
            let mut normals = None;
            let mut tex_coords = None;

            use gltf::Semantic;
            use gltf::accessor::{DataType, Dimensions, Iter};
            let get_buffer_data = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|x| &*x.0);
            for (semantic, accessor) in primitive.attributes() {
                match semantic {
                    Semantic::Positions => {
                        match (accessor.data_type(), accessor.dimensions()) {
                            (DataType::F32, Dimensions::Vec3) => {
                                // let iter = Iter::<[f32; 3]>::new(accessor, get_buffer_data);
                                // for item in iter {
                                //     println!("{:?}", item);
                                // }
                            }
                            _ => {
                                unimplemented!();
                            },
                        }

                        positions = Some(accessor);
                    },
                    Semantic::Normals => {
                        match (accessor.data_type(), accessor.dimensions()) {
                            (DataType::F32, Dimensions::Vec3) => {
                                // let iter = Iter::<[f32; 3]>::new(accessor, get_buffer_data);
                                // for item in iter {
                                //     println!("{:?}", item);
                                // }
                            }
                            _ => {
                                unimplemented!();
                            },
                        }

                        normals = Some(accessor);
                    },
                    Semantic::TexCoords(0) => {
                        match (accessor.data_type(), accessor.dimensions()) {
                            (DataType::F32, Dimensions::Vec2) => {
                                // let iter = Iter::<[f32; 3]>::new(accessor, get_buffer_data);
                                // for item in iter {
                                //     println!("{:?}", item);
                                // }
                            }
                            _ => {
                                unimplemented!();
                            },
                        }

                        tex_coords = Some(accessor);
                    },
                    _ => {
                        unimplemented!();
                    }
                }
            }

            println!(
                "primitives with positions {:?} {:?} {:?} normals {:?} {:?} {:?} tex_coords {:?} {:?} {:?}",
                positions.as_ref().map(|x| x.count()),
                positions.as_ref().map(|x| x.data_type()),
                positions.as_ref().map(|x| x.dimensions()),
                normals.as_ref().map(|x| x.count()),
                normals.as_ref().map(|x| x.data_type()),
                normals.as_ref().map(|x| x.dimensions()),
                tex_coords.as_ref().map(|x| x.count()),
                tex_coords.as_ref().map(|x| x.data_type()),
                tex_coords.as_ref().map(|x| x.dimensions()),
            );
        }
    }

    // for scene in doc.scenes() {
    //     println!("begin scene {:?} {:?}", scene.index(), scene.name());
    //     for node in scene.nodes() {
    //
    //     }
    // }


    // Iterate images
    // - Generates an image asset per image
    // - If these were content-addressable we could get rid of duplicate data..

    // Iterate the materials
    // - Generates a material asset per material

    // Iterate the mesh list
    // - Produce material/mesh-part pairs

    // Iterate the nodes
    // - Grab a scene (just choose 0? Or maybe treat these as LODs?)
    // - Iterate the root nodes
    // - Produce a list of meshes and their final transforms by walking the tree




    // Export will produce:
    // - A bunch of images, each their own asset
    // - A bunch of materials, each their own asset
    //   - dependencies on
    // -




    // doc has...
    // Important:
    // - materials
    // - meshes
    // - nodes
    // - samplers
    // - textures
    // - images
    // - accessors (buffer accessors)
    // - views (buffer views)
    // - buffers
    // Other stuff:
    // - cameras
    // - animations
    // - skins
    // - scenes
    //

    // buffers has a bunch of Vec<u8>s

    // images has
    // - pixels in Vec<u8>
    // - formats (R, RG, RGB, RGBA in 8 bits, BGR, BGRA in 8 bits, and R, RG, RGB, RGBA in 16 bit)
    // - width
    // - height

}