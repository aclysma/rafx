use atelier_assets::core::{AssetUuid, AssetRef};
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, SourceFileImporter
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;
use gltf::image::Data as GltfImageData;
use gltf::buffer::Data as GltfBufferData;
use fnv::FnvHashMap;
use atelier_assets::loader::handle::Handle;
use gltf::Accessor;
use gltf::mesh::util::indices::CastingIter;
use crate::pipeline::image::ImageAsset;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
enum GltfObjectId {
    Name(String),
    Index(usize)
}

#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "130a91a8-ba80-4cad-9bce-848326b234c7"]
pub struct MaterialAsset {
    pub base_color: [f32;4],
    pub base_color_texture: Option<AssetUuid>
}

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[repr(packed(1))]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Serialize, Deserialize)]
pub struct MeshPart {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u16>,
    pub material: Option<AssetUuid>
}

#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "cf232526-3757-4d94-98d1-c2f7e27c979f"]
pub struct MeshAsset {
    pub mesh_parts: Vec<MeshPart>
}

// //TODO: It might not make practical sense to have an overall GLTF asset in the long run, probably
// // would produce separate image, mesh, prefab assets
// #[derive(TypeUuid, Serialize, Deserialize)]
// #[uuid = "122e4e01-d3d3-4e99-8725-c6fcee30ff1a"]
// pub struct GltfAsset {
//     base_color: [f32;4]
// }

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "807c83b3-c24c-4123-9580-5f9c426260b4"]
struct GltfImporterState {
    asset_uuid: Option<AssetUuid>,

    // Asset UUIDs for imported image by name
    image_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    material_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    mesh_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
}

#[derive(TypeUuid)]
#[uuid = "fc9ae812-110d-4daf-9223-e87b40966c6b"]
struct GltfImporter;
impl Importer for GltfImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        11
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = GltfImporterState;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut Read,
        options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        //
        // Get the asset UUID, or create a new UUID if this is a new gltf file
        //
        let gltf_asset_uuid = state
            .asset_uuid
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

        state.asset_uuid = Some(gltf_asset_uuid);

        log::info!("Importing mesh {}", gltf_asset_uuid);

        //
        // Load the GLTF file
        //
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        //let (doc, buffers, images) = gltf::import_slice(&bytes).unwrap();
        let result = gltf::import_slice(&bytes);
        if let Err(err) = result {
            log::error!("GLTF Import error: {:?}", err);
            return Err(Error::Boxed(Box::new(err)));
        }


        let (doc, buffers, images) = gltf::import_slice(&bytes).unwrap();

        let mut imported_assets = Vec::new();

        //
        // Images
        //
        let images_to_import = extract_images_to_import(&doc, &buffers, &images);
        let mut image_index_to_uuid_lookup = vec![];
        for image_to_import in images_to_import {
            // Find the UUID associated with this image or create a new one
            let image_uuid = *state.image_asset_uuids.entry(image_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            image_index_to_uuid_lookup.push(image_uuid.clone());

            let mut search_tags : Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &image_to_import.id {
                search_tags.push(("image_name".to_string(), Some(name.clone())));
            }

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: image_uuid,
                search_tags,
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(image_to_import.asset),
            });
        }

        //
        // Materials
        //
        let materials_to_import = extract_materials_to_import(&doc, &buffers, &images, &image_index_to_uuid_lookup);
        let mut material_index_to_uuid_lookup = vec![];
        for material_to_import in materials_to_import {
            // Find the UUID associated with this image or create a new one
            let material_uuid = *state.material_asset_uuids.entry(material_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            material_index_to_uuid_lookup.push(material_uuid.clone());

            let mut search_tags : Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &material_to_import.id {
                search_tags.push(("material_name".to_string(), Some(name.clone())));
            }

            let mut load_deps = vec![];
            if let Some(image) = material_to_import.asset.base_color_texture {
                load_deps.push(AssetRef::Uuid(image));
            }

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: material_uuid,
                search_tags,
                build_deps: vec![],
                load_deps,
                build_pipeline: None,
                asset_data: Box::new(material_to_import.asset),
            });
        }

        //
        // Meshes
        //
        let meshes_to_import = extract_meshes_to_import(&doc, &buffers, &images, &material_index_to_uuid_lookup);
        let mut mesh_index_to_uuid_lookup = vec![];
        for mesh_to_import in meshes_to_import {
            // Find the UUID associated with this image or create a new one
            let mesh_uuid = *state.mesh_asset_uuids.entry(mesh_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            mesh_index_to_uuid_lookup.push(mesh_uuid.clone());

            let mut search_tags : Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &mesh_to_import.id {
                search_tags.push(("mesh_name".to_string(), Some(name.clone())));
            }

            let mut load_deps = vec![];
            for mesh_part in &mesh_to_import.asset.mesh_parts {
                if let Some(material) = mesh_part.material {
                    load_deps.push(AssetRef::Uuid(material));
                }
            }

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: mesh_uuid,
                search_tags,
                build_deps: vec![],
                load_deps,
                build_pipeline: None,
                asset_data: Box::new(mesh_to_import.asset),
            });
        }


        // //
        // let material_asset = GltfMaterialAsset {
        //     base_color: [1.0, 1.0, 1.0, 1.0]
        // };

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}

struct ImageToImport {
    id: GltfObjectId,
    asset: ImageAsset,
}

fn extract_images_to_import(
    doc: &gltf::Document,
    buffers: &Vec<GltfBufferData>,
    images: &Vec<GltfImageData>
) -> Vec<ImageToImport> {
    let mut images_to_import = Vec::with_capacity(images.len());
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

        let asset = ImageAsset {
            data: converted_image.to_vec(),
            width: image_data.width,
            height: image_data.height
        };
        let id = image.name().map(|s| GltfObjectId::Name(s.to_string())).unwrap_or(GltfObjectId::Index(image.index()));

        let image_to_import = ImageToImport {
            id,
            asset
        };

        // Verify that we iterate images in order so that our resulting assets are in order
        assert!(image.index() == images_to_import.len());
        println!(
            "Importing Texture name: {:?} index: {} width: {} height: {} bytes: {}",
            image.name(),
            image.index(),
            image_to_import.asset.width,
            image_to_import.asset.height,
            image_to_import.asset.data.len()
        );

        images_to_import.push(image_to_import);
    }

    images_to_import
}

struct MaterialToImport {
    id: GltfObjectId,
    asset: MaterialAsset,
}

fn extract_materials_to_import(
    doc: &gltf::Document,
    buffers: &Vec<GltfBufferData>,
    images: &Vec<GltfImageData>,
    image_index_to_uuid_lookup: &[AssetUuid]
) -> Vec<MaterialToImport> {
    let mut materials_to_import = Vec::with_capacity(doc.materials().len());

    for material in doc.materials() {
        let pbr_metallic_roughness = material.pbr_metallic_roughness();
        let base_color = pbr_metallic_roughness.base_color_factor();
        let base_color_texture = pbr_metallic_roughness.base_color_texture().map(|base_texture| {
            image_index_to_uuid_lookup[base_texture.texture().index()]
        });

        let asset = MaterialAsset {
            base_color,
            base_color_texture
        };
        let id = material.name().map(|s| GltfObjectId::Name(s.to_string())).unwrap_or(GltfObjectId::Index(material.index().unwrap()));

        let material_to_import = MaterialToImport {
            id,
            asset
        };

        // Verify that we iterate images in order so that our resulting assets are in order
        assert!(material.index().unwrap() == materials_to_import.len());
        println!(
            "Importing Material name: {:?} index: {} base_color: {:?} texture: {:?}",
            material.name(),
            material.index().unwrap(),
            base_color,
            base_color_texture
        );

        materials_to_import.push(material_to_import);
    }

    materials_to_import
}


struct MeshToImport {
    id: GltfObjectId,
    asset: MeshAsset,
}

// fn read_index_buffer(accessor: Accessor) -> Vec<u16> {
//
//
//     use gltf::accessor::{DataType, Dimensions, Iter};
//     match (accessor.data_type(), accessor.dimensions()) {
//         (DataType::U16, Dimensions::Scalar) => {
//             Vec::with_capacity(accessor.count());
//
//             let iter = Iter::<u16>::new(accessor, )
//
//
//             // let iter = Iter::<[f32; 3]>::new(accessor, get_buffer_data);
//             // for item in iter {
//             //     println!("{:?}", item);
//             // }
//         }
//         _ => {
//             unimplemented!();
//         },
//     }
//
//
// }

// use std::convert::TryFrom;
// fn convert_to_u16_indices(indices: &gltf::mesh::util::ReadIndices) -> Result<Vec<u16>, <u32 as TryFrom<u16>>::Error> {
//     indices.into_u32().map(|x| {
//         u16::try_from(x)?
//     }).collect()
// }

// fn convert_to_u16_indices(read_indices: &gltf::mesh::util::ReadIndices) -> Option<Vec<u16>> {
//     use gltf::mesh::util::ReadIndices;
//     match read_indices {
//         ReadIndices::U8(values) => {
//             Some(values.collect::<Vec<u8>>().map(|x| x as u16).collect())
//         },
//         ReadIndices::U16(values) => {
//             Some(values.map(|x| x).collect())
//         },
//         ReadIndices::U32(values) => {
//             unimplemented!();
//         }
//     }
// }

//TODO: This feels kind of dumb..
fn convert_to_u16_indices(read_indices: gltf::mesh::util::ReadIndices) -> Result<Vec<u16>, std::num::TryFromIntError> {
    use std::convert::TryFrom;
    let indices_u32 : Vec<u32> = read_indices.into_u32().collect();
    let mut indices_u16 : Vec<u16> = Vec::with_capacity(indices_u32.len());
    for index in indices_u32 {
        indices_u16.push(index.try_into()?);
    }

    Ok(indices_u16)
}

fn extract_meshes_to_import(
    doc: &gltf::Document,
    buffers: &Vec<GltfBufferData>,
    images: &Vec<GltfImageData>,
    material_index_to_uuid_lookup: &[AssetUuid]
) -> Vec<MeshToImport> {
    let mut meshes_to_import = Vec::with_capacity(doc.meshes().len());

    for mesh in doc.meshes() {
        let mut mesh_parts : Vec<MeshPart> = Vec::with_capacity(mesh.primitives().len());

        for primitive in mesh.primitives() {
            let mesh_part = {
                let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|x| &**x));
                let positions = reader.read_positions();
                let normals = reader.read_normals();
                let tex_coords = reader.read_tex_coords(0);
                let indices = reader.read_indices();

                if let (Some(indices), Some(positions), Some(normals), Some(tex_coords)) = (indices, positions, normals, tex_coords) {
                    let indices = convert_to_u16_indices(indices);

                    if let Ok(indices) = indices {
                        let positions : Vec<_> = positions.collect();
                        let normals : Vec<_> = normals.collect();
                        let tex_coords : Vec<_> = tex_coords.into_f32().collect();

                        let mut vertices = Vec::with_capacity(positions.len());
                        for i in 0..positions.len() {
                            vertices.push(MeshVertex {
                                position: positions[i],
                                normal: normals[i],
                                tex_coord: tex_coords[i]
                            });
                        }

                        let material = if let Some(material_index) = primitive.material().index() {
                            Some(material_index_to_uuid_lookup[material_index])
                        } else {
                            None
                        };

                        Some(MeshPart {
                            vertices,
                            indices,
                            material
                        })
                    } else {
                        log::error!("indices must fit in u16");
                        None
                    }
                } else {
                    log::error!("Mesh primitives must specify indices, positions, normals, and tex_coords");
                    None
                }
            };

            mesh_parts.push(mesh_part.unwrap());
        }


        let asset = MeshAsset {
            mesh_parts
        };
        let id = mesh.name().map(|s| GltfObjectId::Name(s.to_string())).unwrap_or(GltfObjectId::Index(mesh.index()));

        let mesh_to_import = MeshToImport {
            id,
            asset
        };

        // Verify that we iterate images in order so that our resulting assets are in order
        assert!(mesh.index() == meshes_to_import.len());
        println!(
            "Importing Mesh name: {:?} index: {} mesh_parts count: {}",
            mesh.name(),
            mesh.index(),
            mesh_to_import.asset.mesh_parts.len()
        );

        meshes_to_import.push(mesh_to_import);
    }

    meshes_to_import
}

// make a macro to reduce duplication here :)
inventory::submit!(SourceFileImporter {
    extension: "gltf",
    instantiator: || Box::new(GltfImporter {}),
});
