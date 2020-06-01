use atelier_assets::core::{AssetUuid, AssetRef};
use atelier_assets::importer::{Error, ImportedAsset, Importer, ImporterValue, SourceFileImporter};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;
use gltf::image::Data as GltfImageData;
use gltf::buffer::Data as GltfBufferData;
use fnv::FnvHashMap;
use atelier_assets::loader::handle::Handle;
use gltf::{Accessor, Gltf};
use gltf::mesh::util::indices::CastingIter;
use crate::pipeline::gltf::{GltfMaterialAsset, MeshAsset, MeshPart, MeshVertex, GltfMaterialData, GltfMaterialDataShaderParam};
use crate::pipeline::image::ImageAsset;
use crate::pipeline::buffer::BufferAsset;
use crate::push_buffer::PushBuffer;
use atelier_assets::loader::handle::SerdeContext;
use atelier_assets::loader::handle::AssetHandle;
use crate::pipeline::pipeline::{MaterialInstanceAsset, MaterialAsset, MaterialInstanceSlotAssignment};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
enum GltfObjectId {
    Name(String),
    Index(usize),
}

struct ImageToImport {
    id: GltfObjectId,
    asset: ImageAsset,
}

struct MaterialToImport {
    id: GltfObjectId,
    asset: GltfMaterialAsset,
}

struct MeshToImport {
    id: GltfObjectId,
    asset: MeshAsset,
}

struct BufferToImport {
    id: GltfObjectId,
    asset: BufferAsset,
}

fn get_or_create_uuid(option_uuid: &mut Option<AssetUuid>) -> AssetUuid {
    let uuid = option_uuid.unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

    *option_uuid = Some(uuid);
    uuid
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "807c83b3-c24c-4123-9580-5f9c426260b4"]
struct GltfImporterState {
    //asset_uuid: Option<AssetUuid>,

    // Asset UUIDs for imported image by name
    buffer_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    image_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    material_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    material_instance_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
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
        22
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
        // let gltf_asset_uuid = state
        //     .asset_uuid
        //     .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

        //
        // state.asset_uuid = Some(gltf_asset_uuid);
        //let gltf_asset_uuid = get_or_create_uuid(&mut state.asset_uuid);

        // let vertex_buffer_uuid = get_or_create_uuid(&mut state.vertex_buffer_uuid);
        // let index_buffer_uuid = get_or_create_uuid(&mut state.index_buffer_uuid);
        //
        // let vertex_buffer_handle = atelier_assets::loader::handle::SerdeContext::with_active(|loader_info_provider, ref_op| {
        //     loader_info_provider.get_load_handle(&AssetRef::Uuid(vertex_buffer_uuid))
        // }).unwrap();
        //
        // let index_buffer_handle = atelier_assets::loader::handle::SerdeContext::with_active(|loader_info_provider, ref_op| {
        //     loader_info_provider.get_load_handle(&AssetRef::Uuid(index_buffer_uuid))
        // }).unwrap();

        //log::info!("Importing mesh {}", gltf_asset_uuid);

        //
        // Load the GLTF file
        //
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        let result = gltf::import_slice(&bytes);
        if let Err(err) = result {
            log::error!("GLTF Import error: {:?}", err);
            return Err(Error::Boxed(Box::new(err)));
        }

        let (doc, buffers, images) = gltf::import_slice(&bytes).unwrap();

        // Accumulate everything we will import in this list
        let mut imported_assets = Vec::new();

        //
        // Images
        //
        let images_to_import = extract_images_to_import(&doc, &buffers, &images);
        let mut image_index_to_handle = vec![];
        for image_to_import in images_to_import {
            // Find the UUID associated with this image or create a new one
            let image_uuid = *state
                .image_asset_uuids
                .entry(image_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            let image_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
                let load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(image_uuid)).unwrap();
                Handle::<ImageAsset>::new(ref_op_sender.clone(), load_handle)
            });

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            image_index_to_handle.push(image_handle);

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &image_to_import.id {
                search_tags.push(("image_name".to_string(), Some(name.clone())));
            }

            log::debug!("Importing image uuid {:?}", image_uuid);

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
        // GLTF Material (which we may not end up needing)
        //
        let materials_to_import =
            extract_materials_to_import(&doc, &buffers, &images, &image_index_to_handle);
        let mut material_index_to_handle = vec![];
        for material_to_import in &materials_to_import {
            // Find the UUID associated with this image or create a new one
            let material_uuid = *state
                .material_asset_uuids
                .entry(material_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            let material_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
                let load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(material_uuid)).unwrap();
                Handle::<GltfMaterialAsset>::new(ref_op_sender.clone(), load_handle)
            });

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            material_index_to_handle.push(material_handle);

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &material_to_import.id {
                search_tags.push(("material_name".to_string(), Some(name.clone())));
            }

            // let mut load_deps = vec![];
            // if let Some(image) = &material_to_import.asset.base_color_texture {
            //     let image_uuid = SerdeContext::with_active(|x, _| {
            //         x.get_asset_id(image.load_handle())
            //     }).unwrap();
            //
            //     load_deps.push(AssetRef::Uuid(image_uuid));
            // }

            log::debug!("Importing material uuid {:?}", material_uuid);

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: material_uuid,
                search_tags,
                build_deps: vec![],
                //load_deps,
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(material_to_import.asset.clone()),
            });
        }

        let material_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
            let material_uuid_str = "267e0388-2611-441c-9c78-2d39d1bd3cf1";
            let material_uuid = AssetUuid(*uuid::Uuid::from_str(material_uuid_str).unwrap().as_bytes());

            let material_load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(material_uuid)).unwrap();
            Handle::<MaterialAsset>::new(ref_op_sender.clone(), material_load_handle)
        });

        let null_image_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
            let material_uuid_str = "be831a21-f4f6-45d4-b9eb-e1bb6fc19d22";
            let material_uuid = AssetUuid(*uuid::Uuid::from_str(material_uuid_str).unwrap().as_bytes());

            let material_load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(material_uuid)).unwrap();
            Handle::<ImageAsset>::new(ref_op_sender.clone(), material_load_handle)
        });



        //
        // Material instance
        //
        let mut material_instance_index_to_handle = vec![];
        for material_to_import in &materials_to_import {
            let material_instance_uuid = *state
                .material_instance_asset_uuids
                .entry(material_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            let material_instance_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
                let load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(material_instance_uuid)).unwrap();
                Handle::<MaterialInstanceAsset>::new(ref_op_sender.clone(), load_handle)
            });

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            material_instance_index_to_handle.push(material_instance_handle);

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &material_to_import.id {
                search_tags.push(("material_name".to_string(), Some(name.clone())));
            }

            let mut slot_assignments = vec![];

            let material_data_shader_param : GltfMaterialDataShaderParam = material_to_import.asset.material_data.clone().into();
            slot_assignments.push(MaterialInstanceSlotAssignment {
                slot_name: "per_material_data".to_string(),
                image: None,
                sampler: None,
                buffer_data: Some(renderer_shell_vulkan::util::any_as_bytes(&material_data_shader_param).into())
            });

            fn push_image_slot_assignment(
                slot_name: &str,
                slot_assignments: &mut Vec<MaterialInstanceSlotAssignment>,
                image: &Option<Handle<ImageAsset>>,
                default_image: &Handle<ImageAsset>
            ) {
                slot_assignments.push(MaterialInstanceSlotAssignment {
                    slot_name: slot_name.to_string(),
                    image: Some(image.as_ref().map_or(default_image, |x| x).clone()),
                    sampler: None,
                    buffer_data: None
                });
            }

            push_image_slot_assignment(
                "base_color_texture",
                &mut slot_assignments,
                &material_to_import.asset.base_color_texture,
                &null_image_handle
            );
            push_image_slot_assignment(
                "metallic_roughness_texture",
                &mut slot_assignments,
                &material_to_import.asset.metallic_roughness_texture,
                &null_image_handle
            );
            push_image_slot_assignment(
                "normal_texture",
                &mut slot_assignments,
                &material_to_import.asset.normal_texture,
                &null_image_handle
            );
            push_image_slot_assignment(
                "occlusion_texture",
                &mut slot_assignments,
                &material_to_import.asset.occlusion_texture,
                &null_image_handle
            );
            push_image_slot_assignment(
                "emissive_texture",
                &mut slot_assignments,
                &material_to_import.asset.emissive_texture,
                &null_image_handle
            );

            let material_instance_asset = MaterialInstanceAsset {
                material: material_handle.clone(),
                slot_assignments
            };

            log::debug!("Importing material instance uuid {:?}", material_instance_uuid);

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: material_instance_uuid,
                search_tags,
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(material_instance_asset),
            });
        }



        // let mut vertices = PushBuffer::new(16384);
        // let mut indices = PushBuffer::new(16384);

        //
        // Meshes
        //
        let (meshes_to_import, buffers_to_import) =
            extract_meshes_to_import(
                state,
                &doc,
                &buffers,
                //&images,
                &material_index_to_handle,
                &material_instance_index_to_handle
            );

        let mut buffer_index_to_handle = vec![];
        for buffer_to_import in buffers_to_import {
            // Find the UUID associated with this image or create a new one
            let buffer_uuid = *state
                .buffer_asset_uuids
                .entry(buffer_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            let buffer_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
                let load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(buffer_uuid)).unwrap();
                Handle::<GltfMaterialAsset>::new(ref_op_sender.clone(), load_handle)
            });

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            buffer_index_to_handle.push(buffer_handle);

            log::debug!("Importing buffer uuid {:?}", buffer_uuid);

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: buffer_uuid,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(buffer_to_import.asset),
            });
        }

        //let mut mesh_index_to_uuid_lookup = vec![];
        for mesh_to_import in meshes_to_import {
            // Find the UUID associated with this image or create a new one
            let mesh_uuid = *state
                .mesh_asset_uuids
                .entry(mesh_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            //mesh_index_to_uuid_lookup.push(mesh_uuid.clone());

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &mesh_to_import.id {
                search_tags.push(("mesh_name".to_string(), Some(name.clone())));
            }

            // let mut load_deps = vec![];
            //
            // // Vertex buffer dependency
            // let vertex_buffer_uuid = SerdeContext::with_active(|x, _| {
            //     x.get_asset_id(mesh_to_import.asset.vertex_buffer.load_handle())
            // }).unwrap();
            // load_deps.push(AssetRef::Uuid(vertex_buffer_uuid));
            //
            // // Index buffer dependency
            // let index_buffer_uuid = SerdeContext::with_active(|x, _| {
            //     x.get_asset_id(mesh_to_import.asset.index_buffer.load_handle())
            // }).unwrap();
            // load_deps.push(AssetRef::Uuid(index_buffer_uuid));
            //
            // // Materials dependencies
            // for mesh_part in &mesh_to_import.asset.mesh_parts {
            //     if let Some(material) = &mesh_part.material {
            //         let material_uuid = SerdeContext::with_active(|x, _| {
            //             x.get_asset_id(material.load_handle())
            //         }).unwrap();
            //         load_deps.push(AssetRef::Uuid(material_uuid));
            //     }
            // }

            log::debug!("Importing mesh uuid {:?}", mesh_uuid);

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: mesh_uuid,
                search_tags,
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(mesh_to_import.asset),
            });
        }

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}

fn extract_images_to_import(
    doc: &gltf::Document,
    buffers: &Vec<GltfBufferData>,
    images: &Vec<GltfImageData>,
) -> Vec<ImageToImport> {
    let mut images_to_import = Vec::with_capacity(images.len());
    for image in doc.images() {
        let image_data = &images[image.index()];

        // Convert it to standard RGBA format
        use gltf::image::Format;
        use image::buffer::ConvertBuffer;
        let converted_image: image::RgbaImage = match image_data.format {
            Format::R8 => image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_vec(
                image_data.width,
                image_data.height,
                image_data.pixels.clone(),
            )
            .unwrap()
            .convert(),
            Format::R8G8 => image::ImageBuffer::<image::LumaA<u8>, Vec<u8>>::from_vec(
                image_data.width,
                image_data.height,
                image_data.pixels.clone(),
            )
            .unwrap()
            .convert(),
            Format::R8G8B8 => image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_vec(
                image_data.width,
                image_data.height,
                image_data.pixels.clone(),
            )
            .unwrap()
            .convert(),
            Format::R8G8B8A8 => image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
                image_data.width,
                image_data.height,
                image_data.pixels.clone(),
            )
            .unwrap()
            .convert(),
            Format::B8G8R8 => image::ImageBuffer::<image::Bgr<u8>, Vec<u8>>::from_vec(
                image_data.width,
                image_data.height,
                image_data.pixels.clone(),
            )
            .unwrap()
            .convert(),
            Format::B8G8R8A8 => image::ImageBuffer::<image::Bgra<u8>, Vec<u8>>::from_vec(
                image_data.width,
                image_data.height,
                image_data.pixels.clone(),
            )
            .unwrap()
            .convert(),
            Format::R16 => {
                unimplemented!();
            }
            Format::R16G16 => {
                unimplemented!();
            }
            Format::R16G16B16 => {
                unimplemented!();
            }
            Format::R16G16B16A16 => {
                unimplemented!();
            }
        };

        let asset = ImageAsset {
            data: converted_image.to_vec(),
            width: image_data.width,
            height: image_data.height,
        };
        let id = image
            .name()
            .map(|s| GltfObjectId::Name(s.to_string()))
            .unwrap_or(GltfObjectId::Index(image.index()));

        let image_to_import = ImageToImport { id, asset };

        // Verify that we iterate images in order so that our resulting assets are in order
        assert!(image.index() == images_to_import.len());
        log::debug!(
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

fn extract_materials_to_import(
    doc: &gltf::Document,
    buffers: &Vec<GltfBufferData>,
    images: &Vec<GltfImageData>,
    image_index_to_handle: &[Handle<ImageAsset>],
) -> Vec<MaterialToImport> {
    let mut materials_to_import = Vec::with_capacity(doc.materials().len());

    for material in doc.materials() {
/*
        let mut material_data = GltfMaterialData {
            base_color_factor: [f32; 4], // default: 1,1,1,1
            emissive_factor: [f32; 3],
            metallic_factor: f32, //default: 1,
            roughness_factor: f32, // default: 1,
            normal_texture_scale: f32, // default: 1
            occlusion_texture_strength: f32, // default 1
            alpha_cutoff: f32, // default 0.5
        }

        let material_asset = GltfMaterialAsset {
            material_data,
            base_color_factor: base_color,
            base_color_texture: base_color_texture.clone(),
            metallic_roughness_texture: None,
            normal_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
        };
*/
        let mut material_asset = GltfMaterialAsset::default();

        let pbr_metallic_roughness = &material.pbr_metallic_roughness();
        material_asset.material_data.base_color_factor = pbr_metallic_roughness.base_color_factor();
        material_asset.material_data.emissive_factor = material.emissive_factor();
        material_asset.material_data.metallic_factor = pbr_metallic_roughness.metallic_factor();
        material_asset.material_data.roughness_factor = pbr_metallic_roughness.roughness_factor();
        material_asset.material_data.normal_texture_scale = material.normal_texture().map_or(1.0, |x| x.scale());
        material_asset.material_data.occlusion_texture_strength = material.occlusion_texture().map_or(1.0, |x| x.strength());
        material_asset.material_data.alpha_cutoff = material.alpha_cutoff();

        material_asset.base_color_texture = pbr_metallic_roughness
            .base_color_texture()
            .map(|texture| image_index_to_handle[texture.texture().source().index()].clone());
        material_asset.metallic_roughness_texture = pbr_metallic_roughness
            .metallic_roughness_texture()
            .map(|texture| image_index_to_handle[texture.texture().source().index()].clone());
        material_asset.normal_texture = material
            .normal_texture()
            .map(|texture| image_index_to_handle[texture.texture().source().index()].clone());
        material_asset.occlusion_texture = material
            .occlusion_texture()
            .map(|texture| image_index_to_handle[texture.texture().source().index()].clone());
        material_asset.emissive_texture = material
            .emissive_texture()
            .map(|texture| image_index_to_handle[texture.texture().source().index()].clone());

        material_asset.material_data.has_base_color_texture = material_asset.base_color_texture.is_some();
        material_asset.material_data.has_metallic_roughness_texture = material_asset.metallic_roughness_texture.is_some();
        material_asset.material_data.has_normal_texture = material_asset.normal_texture.is_some();
        material_asset.material_data.has_occlusion_texture = material_asset.occlusion_texture.is_some();
        material_asset.material_data.has_emissive_texture = material_asset.emissive_texture.is_some();



        // pub base_color_texture: Option<Handle<ImageAsset>>,
        // // metalness in B, roughness in G
        // pub metallic_roughness_texture: Option<Handle<ImageAsset>>,
        // pub normal_texture: Option<Handle<ImageAsset>>,
        // pub occlusion_texture: Option<Handle<ImageAsset>>,
        // pub emissive_texture: Option<Handle<ImageAsset>>,


        let id = material
            .name()
            .map(|s| GltfObjectId::Name(s.to_string()))
            .unwrap_or(GltfObjectId::Index(material.index().unwrap()));

        let material_to_import = MaterialToImport { id, asset: material_asset };

        // Verify that we iterate images in order so that our resulting assets are in order
        assert!(material.index().unwrap() == materials_to_import.len());
        log::debug!(
            "Importing Material name: {:?} index: {}",
            material.name(),
            material.index().unwrap(),
        );

        materials_to_import.push(material_to_import);
    }

    materials_to_import
}

//TODO: This feels kind of dumb..
fn convert_to_u16_indices(
    read_indices: gltf::mesh::util::ReadIndices
) -> Result<Vec<u16>, std::num::TryFromIntError> {
    use std::convert::TryFrom;
    let indices_u32: Vec<u32> = read_indices.into_u32().collect();
    let mut indices_u16: Vec<u16> = Vec::with_capacity(indices_u32.len());
    for index in indices_u32 {
        indices_u16.push(index.try_into()?);
    }

    Ok(indices_u16)
}

fn extract_meshes_to_import(
    state: &mut GltfImporterState,
    doc: &gltf::Document,
    buffers: &Vec<GltfBufferData>,
    //images: &Vec<GltfImageData>,
    material_index_to_handle: &[Handle<GltfMaterialAsset>],
    material_instance_index_to_handle: &[Handle<MaterialInstanceAsset>],
) -> (Vec<MeshToImport>, Vec<BufferToImport>) {
    let mut meshes_to_import = Vec::with_capacity(doc.meshes().len());
    let mut buffers_to_import = Vec::with_capacity(doc.meshes().len() * 2);

    for mesh in doc.meshes() {
        let mut all_vertices = PushBuffer::new(16384);
        let mut all_indices = PushBuffer::new(16384);

        let mut mesh_parts: Vec<MeshPart> = Vec::with_capacity(mesh.primitives().len());

        //
        // Iterate all mesh parts, building a single vertex and index buffer. Each MeshPart will
        // hold offsets/lengths to their sections in the vertex/index buffers
        //
        for primitive in mesh.primitives() {
            let mesh_part = {
                let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|x| &**x));

                let positions = reader.read_positions();
                let normals = reader.read_normals();
                let tangents = reader.read_tangents();
                //let colors = reader.read_colors();
                let tex_coords = reader.read_tex_coords(0);
                let indices = reader.read_indices();

                if let (Some(indices), Some(positions), Some(normals), Some(tangents), Some(tex_coords)) =
                    (indices, positions, normals, tangents, tex_coords)
                {
                    let part_indices = convert_to_u16_indices(indices);

                    if let Ok(part_indices) = part_indices {
                        //TODO: Consider computing binormal (bitangent) here
                        let positions: Vec<_> = positions.collect();
                        let normals: Vec<_> = normals.collect();
                        let tangents: Vec<_> = tangents.collect();
                        let tex_coords: Vec<_> = tex_coords.into_f32().collect();

                        let vertex_offset = all_vertices.len();
                        let indices_offset = all_indices.len();

                        for i in 0..positions.len() {
                            all_vertices.push(&[MeshVertex {
                                position: positions[i],
                                normal: normals[i],
                                tangent: tangents[i],
                                tex_coord: tex_coords[i],
                            }],
                            1);
                        }

                        all_indices.push(&part_indices, 1);

                        let vertex_size = all_vertices.len() - vertex_offset;
                        let indices_size = all_indices.len() - indices_offset;

                        let material = if let Some(material_index) = primitive.material().index() {
                            Some(material_index_to_handle[material_index].clone())
                        } else {
                            None
                        };

                        let material_instance = if let Some(material_index) = primitive.material().index() {
                            Some(material_instance_index_to_handle[material_index].clone())
                        } else {
                            None
                        };

                        Some(MeshPart {
                            material,
                            material_instance,
                            vertex_buffer_offset_in_bytes: vertex_offset as u32,
                            vertex_buffer_size_in_bytes: vertex_size as u32,
                            index_buffer_offset_in_bytes: indices_offset as u32,
                            index_buffer_size_in_bytes: indices_size as u32
                        })
                    } else {
                        log::error!("indices must fit in u16");
                        None
                    }
                } else {
                    log::error!(
                        "Mesh primitives must specify indices, positions, normals, tangents, and tex_coords"
                    );
                    None
                }
            };

            if let Some(mesh_part) = mesh_part {
                mesh_parts.push(mesh_part);
            }
        }

        //
        // Vertex Buffer
        //
        let vertex_buffer_asset = BufferAsset {
            data: all_vertices.into_data()
        };

        let vertex_buffer_id = GltfObjectId::Index(buffers_to_import.len());
        let vertex_buffer_to_import = BufferToImport {
            asset: vertex_buffer_asset,
            id: vertex_buffer_id.clone()
        };

        let vertex_buffer_uuid = *state
            .buffer_asset_uuids
            .entry(vertex_buffer_id)
            .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

        buffers_to_import.push(vertex_buffer_to_import);

        let vertex_buffer_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
            let load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(vertex_buffer_uuid)).unwrap();
            Handle::<BufferAsset>::new(ref_op_sender.clone(), load_handle)
        });


        //
        // Index Buffer
        //
        let index_buffer_asset = BufferAsset {
            data: all_indices.into_data()
        };

        let index_buffer_id = GltfObjectId::Index(buffers_to_import.len());
        let index_buffer_to_import = BufferToImport {
            asset: index_buffer_asset,
            id: index_buffer_id.clone()
        };

        let index_buffer_uuid = *state
            .buffer_asset_uuids
            .entry(index_buffer_id)
            .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

        buffers_to_import.push(index_buffer_to_import);

        let index_buffer_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
            let load_handle = loader_info_provider.get_load_handle(&AssetRef::Uuid(index_buffer_uuid)).unwrap();
            Handle::<BufferAsset>::new(ref_op_sender.clone(), load_handle)
        });



        let asset = MeshAsset {
            mesh_parts,
            vertex_buffer: vertex_buffer_handle,
            index_buffer: index_buffer_handle,
        };

        let mesh_id = mesh
            .name()
            .map(|s| GltfObjectId::Name(s.to_string()))
            .unwrap_or(GltfObjectId::Index(mesh.index()));

        let mesh_to_import = MeshToImport {
            id: mesh_id,
            asset
        };

        // Verify that we iterate meshes in order so that our resulting assets are in order
        assert!(mesh.index() == meshes_to_import.len());
        log::debug!(
            "Importing Mesh name: {:?} index: {} mesh_parts count: {}",
            mesh.name(),
            mesh.index(),
            mesh_to_import.asset.mesh_parts.len()
        );

        meshes_to_import.push(mesh_to_import);
    }

    (meshes_to_import, buffers_to_import)
}

// make a macro to reduce duplication here :)
inventory::submit!(SourceFileImporter {
    extension: "gltf",
    instantiator: || Box::new(GltfImporter {}),
});

// make a macro to reduce duplication here :)
inventory::submit!(SourceFileImporter {
    extension: "glb",
    instantiator: || Box::new(GltfImporter {}),
});
