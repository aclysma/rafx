use crate::assets::mesh_basic::{
    MeshBasicAssetData, MeshBasicMaterialData, MeshBasicMaterialDataShaderParam,
    MeshBasicPartAssetData, MeshMaterialBasicAsset, MeshMaterialBasicAssetData,
};
use fnv::FnvHashMap;
use glam::Vec3;
use gltf::buffer::Data as GltfBufferData;
use gltf::image::Data as GltfImageData;
use hydrate_base::handle::Handle;
use itertools::Itertools;
use rafx::api::RafxResourceType;
use rafx::assets::BufferAssetData;
use rafx::assets::ImageAsset;
use rafx::assets::PushBuffer;
use rafx::assets::{ImageAssetColorSpaceConfig, ImageAssetData};
use rafx::assets::{MaterialInstanceAssetData, MaterialInstanceSlotAssignment};
use rafx::rafx_visibility::{PolygonSoup, PolygonSoupIndex, VisibleBounds};
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

//TODO: These are extensions that might be interesting to try supporting. In particular, lights,
// LOD, and clearcoat
// Good explanations of upcoming extensions here: https://medium.com/@babylonjs/gltf-extensions-in-babylon-js-b3fa56de5483
//KHR_materials_clearcoat: https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_materials_clearcoat/README.md
//KHR_materials_pbrSpecularGlossiness: https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_materials_pbrSpecularGlossiness/README.md
//KHR_materials_unlit: https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_materials_unlit/README.md
//KHR_lights_punctual (directional, point, spot): https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_lights_punctual/README.md
//EXT_lights_image_based: https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Vendor/EXT_lights_image_based/README.md
//MSFT_lod: https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Vendor/MSFT_lod/README.md
//MSFT_packing_normalRoughnessMetallic: https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Vendor/MSFT_packing_normalRoughnessMetallic/README.md
// Normal: NG, Roughness: B, Metallic: A
//MSFT_packing_occlusionRoughnessMetallic: https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Vendor/MSFT_packing_occlusionRoughnessMetallic/README.md

#[derive(Debug)]
struct GltfImportError {
    error_message: String,
}

impl GltfImportError {
    pub fn new(error_message: &str) -> Self {
        GltfImportError {
            error_message: error_message.to_string(),
        }
    }
}

impl std::error::Error for GltfImportError {}

impl std::fmt::Display for GltfImportError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.error_message)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum GltfObjectId {
    Name(String),
    Index(usize),
}

struct ImageToImport {
    id: GltfObjectId,
    asset: ImageAssetData,
}

#[derive(Default, Clone)]
pub struct MeshBasicGltfMaterialImportData {
    //pub name: Option<String>,
    pub material_data: MeshBasicMaterialData,

    pub base_color_texture: Option<Handle<ImageAsset>>,
    // metalness in B, roughness in G
    pub metallic_roughness_texture: Option<Handle<ImageAsset>>,
    pub normal_texture: Option<Handle<ImageAsset>>,
    pub occlusion_texture: Option<Handle<ImageAsset>>,
    pub emissive_texture: Option<Handle<ImageAsset>>,
    // We would need to change the pipeline for these
    // double_sided: bool, // defult false
    // alpha_mode: String, // OPAQUE, MASK, BLEND
    // support for points/lines?
}

struct MaterialToImport {
    id: GltfObjectId,
    asset: MeshBasicGltfMaterialImportData,
}

struct MeshToImport {
    id: GltfObjectId,
    asset: MeshBasicAssetData,
}

struct BufferToImport {
    id: GltfObjectId,
    asset: BufferAssetData,
}
//
// // The asset state is stored in this format using Vecs
// #[derive(TypeUuid, Serialize, Deserialize, Default, Clone)]
// #[uuid = "807c83b3-c24c-4123-9580-5f9c426260b4"]
// pub struct MeshBasicGltfImporterStateStable {
//     // Asset UUIDs for imported image by name. We use vecs here so we can sort by UUID for
//     // deterministic output
//     buffer_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
//     image_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
//     material_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
//     material_instance_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
//     mesh_material_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
//     mesh_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
// }
//
// impl From<MeshBasicGltfImporterStateUnstable> for MeshBasicGltfImporterStateStable {
//     fn from(other: MeshBasicGltfImporterStateUnstable) -> Self {
//         let mut stable = MeshBasicGltfImporterStateStable::default();
//         stable.buffer_asset_uuids = other
//             .buffer_asset_uuids
//             .into_iter()
//             .sorted_by_key(|(id, _uuid)| id.clone())
//             .collect();
//         stable.image_asset_uuids = other
//             .image_asset_uuids
//             .into_iter()
//             .sorted_by_key(|(id, _uuid)| id.clone())
//             .collect();
//         stable.material_asset_uuids = other
//             .material_asset_uuids
//             .into_iter()
//             .sorted_by_key(|(id, _uuid)| id.clone())
//             .collect();
//         stable.material_instance_asset_uuids = other
//             .material_instance_asset_uuids
//             .into_iter()
//             .sorted_by_key(|(id, _uuid)| id.clone())
//             .collect();
//         stable.mesh_material_asset_uuids = other
//             .mesh_material_asset_uuids
//             .into_iter()
//             .sorted_by_key(|(id, _uuid)| id.clone())
//             .collect();
//         stable.mesh_asset_uuids = other
//             .mesh_asset_uuids
//             .into_iter()
//             .sorted_by_key(|(id, _uuid)| id.clone())
//             .collect();
//         stable
//     }
// }
//
// #[derive(Default)]
// pub struct MeshBasicGltfImporterStateUnstable {
//     //asset_uuid: Option<AssetUuid>,
//
//     // Asset UUIDs for imported image by name
//     buffer_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
//     image_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
//     material_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
//     material_instance_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
//     mesh_material_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
//     mesh_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
// }
//
// impl From<MeshBasicGltfImporterStateStable> for MeshBasicGltfImporterStateUnstable {
//     fn from(other: MeshBasicGltfImporterStateStable) -> Self {
//         let mut unstable = MeshBasicGltfImporterStateUnstable::default();
//         unstable.buffer_asset_uuids = other.buffer_asset_uuids.into_iter().collect();
//         unstable.image_asset_uuids = other.image_asset_uuids.into_iter().collect();
//         unstable.material_asset_uuids = other.material_asset_uuids.into_iter().collect();
//         unstable.material_instance_asset_uuids =
//             other.material_instance_asset_uuids.into_iter().collect();
//         unstable.mesh_material_asset_uuids = other.mesh_material_asset_uuids.into_iter().collect();
//         unstable.mesh_asset_uuids = other.mesh_asset_uuids.into_iter().collect();
//         unstable
//     }
// }
//
// #[derive(TypeUuid)]
// #[uuid = "fc9ae812-110d-4daf-9223-e87b40966c6b"]
// pub struct MeshBasicGltfImporter;
// impl Importer for MeshBasicGltfImporter {
//     fn version_static() -> u32
//     where
//         Self: Sized,
//     {
//         29
//     }
//
//     fn version(&self) -> u32 {
//         Self::version_static()
//     }
//
//     type Options = ();
//
//     type State = MeshBasicGltfImporterStateStable;
//
//     /// Reads the given bytes and produces assets.
//     fn import(
//         &self,
//         op: &mut ImportOp,
//         source: &mut dyn Read,
//         _options: &Self::Options,
//         stable_state: &mut Self::State,
//     ) -> distill::importer::Result<ImporterValue> {
//         let mut unstable_state: MeshBasicGltfImporterStateUnstable = stable_state.clone().into();
//
//         //
//         // Load the GLTF file
//         //
//         let mut bytes = Vec::new();
//         source.read_to_end(&mut bytes)?;
//         let result = gltf::import_slice(&bytes);
//         if let Err(err) = result {
//             log::error!("GLTF Import error: {:?}", err);
//             return Err(Error::Boxed(Box::new(err)));
//         }
//
//         let (doc, buffers, images) = result.unwrap();
//
//         // Accumulate everything we will import in this list
//         let mut imported_assets = Vec::new();
//
//         let image_color_space_assignments =
//             build_image_color_space_assignments_from_materials(&doc);
//
//         //
//         // Images
//         //
//         let images_to_import =
//             extract_images_to_import(&doc, &buffers, &images, &image_color_space_assignments);
//         let mut image_index_to_handle = vec![];
//         for image_to_import in images_to_import {
//             // Find the UUID associated with this image or create a new one
//             let image_uuid = *unstable_state
//                 .image_asset_uuids
//                 .entry(image_to_import.id.clone())
//                 .or_insert_with(|| op.new_asset_uuid());
//
//             let image_handle = make_handle(image_uuid);
//             // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
//             image_index_to_handle.push(image_handle);
//
//             let mut search_tags: Vec<(String, Option<String>)> = vec![];
//             if let GltfObjectId::Name(name) = &image_to_import.id {
//                 search_tags.push(("name".to_string(), Some(name.clone())));
//             }
//
//             log::debug!("Importing image uuid {:?}", image_uuid);
//
//             // Create the asset
//             imported_assets.push(ImportedAsset {
//                 id: image_uuid,
//                 search_tags,
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(image_to_import.asset),
//             });
//         }
//
//         //
//         // GLTF Material (which we may not end up needing)
//         //
//         let materials_to_import =
//             extract_materials_to_import(&doc, &buffers, &images, &image_index_to_handle);
//
//         let material_handle = make_handle_from_str("92a98639-de0d-40cf-a222-354f616346c3")?;
//
//         let null_image_handle = make_handle_from_str("fc937369-cad2-4a00-bf42-5968f1210784")?;
//
//         //
//         // Material instance
//         //
//         let mut mesh_material_index_to_handle = vec![];
//         for material_to_import in &materials_to_import {
//             //
//             // Push the material instance UUID into the list so that we have an O(1) lookup material index to UUID
//             //
//             let material_instance_uuid = *unstable_state
//                 .material_instance_asset_uuids
//                 .entry(material_to_import.id.clone())
//                 .or_insert_with(|| op.new_asset_uuid());
//
//             let material_instance_handle = make_handle(material_instance_uuid);
//
//             let mut search_tags: Vec<(String, Option<String>)> = vec![];
//             if let GltfObjectId::Name(name) = &material_to_import.id {
//                 search_tags.push(("name".to_string(), Some(name.clone())));
//             }
//
//             //
//             // Create the material instance
//             //
//             let mut slot_assignments = vec![];
//
//             let material_data = &material_to_import.asset.material_data;
//             let material_data_shader_param: MeshBasicMaterialDataShaderParam =
//                 material_data.clone().into();
//             slot_assignments.push(MaterialInstanceSlotAssignment {
//                 slot_name: "per_material_data".to_string(),
//                 array_index: 0,
//                 image: None,
//                 sampler: None,
//                 buffer_data: Some(
//                     rafx::base::memory::any_as_bytes(&material_data_shader_param).into(),
//                 ),
//             });
//
//             fn choose_image_handle(
//                 should_include: bool,
//                 image: &Option<Handle<ImageAsset>>,
//                 default_image: &Handle<ImageAsset>,
//             ) -> Option<Handle<ImageAsset>> {
//                 if should_include {
//                     Some(image.as_ref().map_or(default_image, |x| x).clone())
//                 } else {
//                     Some(default_image.clone())
//                 }
//             }
//
//             fn push_image_slot_assignment(
//                 slot_name: &str,
//                 slot_assignments: &mut Vec<MaterialInstanceSlotAssignment>,
//                 should_include: bool,
//                 image: &Option<Handle<ImageAsset>>,
//                 default_image: &Handle<ImageAsset>,
//             ) {
//                 slot_assignments.push(MaterialInstanceSlotAssignment {
//                     slot_name: slot_name.to_string(),
//                     array_index: 0,
//                     image: choose_image_handle(should_include, image, default_image),
//                     sampler: None,
//                     buffer_data: None,
//                 });
//             }
//
//             push_image_slot_assignment(
//                 "base_color_texture",
//                 &mut slot_assignments,
//                 material_data.has_base_color_texture,
//                 &material_to_import.asset.base_color_texture,
//                 &null_image_handle,
//             );
//             push_image_slot_assignment(
//                 "metallic_roughness_texture",
//                 &mut slot_assignments,
//                 material_data.has_metallic_roughness_texture,
//                 &material_to_import.asset.metallic_roughness_texture,
//                 &null_image_handle,
//             );
//             push_image_slot_assignment(
//                 "normal_texture",
//                 &mut slot_assignments,
//                 material_data.has_normal_texture,
//                 &material_to_import.asset.normal_texture,
//                 &null_image_handle,
//             );
//             push_image_slot_assignment(
//                 "emissive_texture",
//                 &mut slot_assignments,
//                 material_data.has_emissive_texture,
//                 &material_to_import.asset.emissive_texture,
//                 &null_image_handle,
//             );
//
//             let material_instance_asset = MaterialInstanceAssetData {
//                 material: material_handle.clone(),
//                 slot_assignments,
//             };
//
//             log::debug!(
//                 "Importing material instance uuid {:?}",
//                 material_instance_uuid
//             );
//
//             // Create the asset
//             imported_assets.push(ImportedAsset {
//                 id: material_instance_uuid,
//                 search_tags,
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(material_instance_asset),
//             });
//
//             //
//             // Create the mesh material
//             //
//
//             //
//             // Push the mesh material UUID into the list so that we have an O(1) lookup material index to UUID
//             //
//             let mesh_material_uuid = *unstable_state
//                 .mesh_material_asset_uuids
//                 .entry(material_to_import.id.clone())
//                 .or_insert_with(|| op.new_asset_uuid());
//
//             let mesh_material_handle = make_handle(mesh_material_uuid);
//
//             mesh_material_index_to_handle.push(mesh_material_handle);
//
//             let mut search_tags: Vec<(String, Option<String>)> = vec![];
//             if let GltfObjectId::Name(name) = &material_to_import.id {
//                 search_tags.push(("name".to_string(), Some(name.clone())));
//             }
//
//             let mesh_material_asset = MeshMaterialBasicAssetData {
//                 material_data: material_data.clone(),
//                 material_instance: material_instance_handle,
//                 color_texture: choose_image_handle(
//                     material_data.has_base_color_texture,
//                     &material_to_import.asset.base_color_texture,
//                     &null_image_handle,
//                 ),
//                 metallic_roughness_texture: choose_image_handle(
//                     material_data.has_metallic_roughness_texture,
//                     &material_to_import.asset.metallic_roughness_texture,
//                     &null_image_handle,
//                 ),
//                 normal_texture: choose_image_handle(
//                     material_data.has_normal_texture,
//                     &material_to_import.asset.normal_texture,
//                     &null_image_handle,
//                 ),
//                 emissive_texture: choose_image_handle(
//                     material_data.has_emissive_texture,
//                     &material_to_import.asset.emissive_texture,
//                     &null_image_handle,
//                 ),
//             };
//
//             imported_assets.push(ImportedAsset {
//                 id: mesh_material_uuid,
//                 search_tags,
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(mesh_material_asset),
//             });
//         }
//
//         //
//         // Meshes
//         //
//         let (meshes_to_import, buffers_to_import) = extract_meshes_to_import(
//             op,
//             &mut unstable_state,
//             &doc,
//             &buffers,
//             &mesh_material_index_to_handle,
//         )?;
//
//         let mut buffer_index_to_handle = vec![];
//         for buffer_to_import in buffers_to_import {
//             // Find the UUID associated with this image or create a new one
//             let buffer_uuid = *unstable_state
//                 .buffer_asset_uuids
//                 .entry(buffer_to_import.id.clone())
//                 .or_insert_with(|| op.new_asset_uuid());
//
//             let buffer_handle = make_handle::<BufferAssetData>(buffer_uuid);
//
//             // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
//             buffer_index_to_handle.push(buffer_handle);
//
//             log::debug!("Importing buffer uuid {:?}", buffer_uuid);
//
//             // Create the asset
//             imported_assets.push(ImportedAsset {
//                 id: buffer_uuid,
//                 search_tags: vec![],
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(buffer_to_import.asset),
//             });
//         }
//
//         //let mut mesh_index_to_uuid_lookup = vec![];
//         for mesh_to_import in meshes_to_import {
//             // Find the UUID associated with this image or create a new one
//             let mesh_uuid = *unstable_state
//                 .mesh_asset_uuids
//                 .entry(mesh_to_import.id.clone())
//                 .or_insert_with(|| op.new_asset_uuid());
//
//             // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
//             //mesh_index_to_uuid_lookup.push(mesh_uuid.clone());
//
//             let mut search_tags: Vec<(String, Option<String>)> = vec![];
//             if let GltfObjectId::Name(name) = &mesh_to_import.id {
//                 search_tags.push(("name".to_string(), Some(name.clone())));
//             }
//
//             // let mut load_deps = vec![];
//             //
//             // // Vertex buffer dependency
//             // let vertex_buffer_uuid = SerdeContext::with_active(|x, _| {
//             //     x.get_asset_id(mesh_to_import.asset.vertex_buffer.load_handle())
//             // }).unwrap();
//             // load_deps.push(AssetRef::Uuid(vertex_buffer_uuid));
//             //
//             // // Index buffer dependency
//             // let index_buffer_uuid = SerdeContext::with_active(|x, _| {
//             //     x.get_asset_id(mesh_to_import.asset.index_buffer.load_handle())
//             // }).unwrap();
//             // load_deps.push(AssetRef::Uuid(index_buffer_uuid));
//             //
//             // // Materials dependencies
//             // for mesh_part in &mesh_to_import.asset.mesh_parts {
//             //     if let Some(material) = &mesh_part.material {
//             //         let material_uuid = SerdeContext::with_active(|x, _| {
//             //             x.get_asset_id(material.load_handle())
//             //         }).unwrap();
//             //         load_deps.push(AssetRef::Uuid(material_uuid));
//             //     }
//             // }
//
//             log::debug!("Importing mesh uuid {:?}", mesh_uuid);
//
//             // Create the asset
//             imported_assets.push(ImportedAsset {
//                 id: mesh_uuid,
//                 search_tags,
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(mesh_to_import.asset),
//             });
//         }
//
//         *stable_state = unstable_state.into();
//
//         Ok(ImporterValue {
//             assets: imported_assets,
//         })
//     }
// }

fn extract_images_to_import(
    doc: &gltf::Document,
    _buffers: &[GltfBufferData],
    images: &[GltfImageData],
    image_color_space_assignments: &FnvHashMap<usize, ImageAssetColorSpaceConfig>,
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

        let color_space = *image_color_space_assignments
            .get(&image.index())
            .unwrap_or(&ImageAssetColorSpaceConfig::Linear);
        log::trace!(
            "Choosing color space {:?} for image index {}",
            color_space,
            image.index()
        );

        let (format, mip_generation) = ImageAssetData::default_format_and_mip_generation();
        let asset_data = ImageAssetData::from_raw_rgba32(
            image_data.width,
            image_data.height,
            color_space,
            format,
            mip_generation,
            RafxResourceType::TEXTURE,
            converted_image.as_raw().as_slice(),
        )
        .unwrap();

        let id = image
            .name()
            .map(|s| GltfObjectId::Name(s.to_string()))
            .unwrap_or_else(|| GltfObjectId::Index(image.index()));

        let image_to_import = ImageToImport {
            id,
            asset: asset_data,
        };

        // Verify that we iterate images in order so that our resulting assets are in order
        assert!(image.index() == images_to_import.len());
        log::debug!(
            "Importing Texture name: {:?} index: {} width: {} height: {}",
            image.name(),
            image.index(),
            image_to_import.asset.width,
            image_to_import.asset.height,
        );

        images_to_import.push(image_to_import);
    }

    images_to_import
}

fn build_image_color_space_assignments_from_materials(
    doc: &gltf::Document
) -> FnvHashMap<usize, ImageAssetColorSpaceConfig> {
    let mut image_color_space_assignments = FnvHashMap::default();

    for material in doc.materials() {
        let pbr_metallic_roughness = &material.pbr_metallic_roughness();

        if let Some(texture) = pbr_metallic_roughness.base_color_texture() {
            image_color_space_assignments.insert(
                texture.texture().source().index(),
                ImageAssetColorSpaceConfig::Srgb,
            );
        }

        if let Some(texture) = pbr_metallic_roughness.metallic_roughness_texture() {
            image_color_space_assignments.insert(
                texture.texture().source().index(),
                ImageAssetColorSpaceConfig::Linear,
            );
        }

        if let Some(texture) = material.normal_texture() {
            image_color_space_assignments.insert(
                texture.texture().source().index(),
                ImageAssetColorSpaceConfig::Linear,
            );
        }

        if let Some(texture) = material.occlusion_texture() {
            image_color_space_assignments.insert(
                texture.texture().source().index(),
                ImageAssetColorSpaceConfig::Srgb,
            );
        }

        if let Some(texture) = material.emissive_texture() {
            image_color_space_assignments.insert(
                texture.texture().source().index(),
                ImageAssetColorSpaceConfig::Srgb,
            );
        }
    }

    image_color_space_assignments
}

fn extract_materials_to_import(
    doc: &gltf::Document,
    _buffers: &[GltfBufferData],
    _images: &[GltfImageData],
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
        let mut material_asset = MeshBasicGltfMaterialImportData::default();

        let pbr_metallic_roughness = &material.pbr_metallic_roughness();
        material_asset.material_data.base_color_factor = pbr_metallic_roughness.base_color_factor();
        material_asset.material_data.emissive_factor = material.emissive_factor();
        material_asset.material_data.metallic_factor = pbr_metallic_roughness.metallic_factor();
        material_asset.material_data.roughness_factor = pbr_metallic_roughness.roughness_factor();
        material_asset.material_data.normal_texture_scale =
            material.normal_texture().map_or(1.0, |x| x.scale());
        // Default is 0.5 per GLTF specification
        material_asset.material_data.alpha_threshold = material.alpha_cutoff().unwrap_or(0.5);

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

        material_asset.material_data.has_base_color_texture =
            material_asset.base_color_texture.is_some();
        material_asset.material_data.has_metallic_roughness_texture =
            material_asset.metallic_roughness_texture.is_some();
        material_asset.material_data.has_normal_texture = material_asset.normal_texture.is_some();
        material_asset.material_data.has_emissive_texture =
            material_asset.emissive_texture.is_some();

        // pub base_color_texture: Option<Handle<ImageAsset>>,
        // // metalness in B, roughness in G
        // pub metallic_roughness_texture: Option<Handle<ImageAsset>>,
        // pub normal_texture: Option<Handle<ImageAsset>>,
        // pub occlusion_texture: Option<Handle<ImageAsset>>,
        // pub emissive_texture: Option<Handle<ImageAsset>>,

        let id = material
            .name()
            .map(|s| GltfObjectId::Name(s.to_string()))
            .unwrap_or_else(|| GltfObjectId::Index(material.index().unwrap()));

        let material_to_import = MaterialToImport {
            id,
            asset: material_asset,
        };

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

fn extract_meshes_to_import(
    op: &mut ImportOp,
    state: &mut MeshBasicGltfImporterStateUnstable,
    doc: &gltf::Document,
    buffers: &[GltfBufferData],
    mesh_material_index_to_handle: &[Handle<MeshMaterialBasicAsset>],
) -> distill::importer::Result<(Vec<MeshToImport>, Vec<BufferToImport>)> {
    let mut meshes_to_import = Vec::with_capacity(doc.meshes().len());
    let mut buffers_to_import = Vec::with_capacity(doc.meshes().len() * 2);

    for mesh in doc.meshes() {
        let mut all_positions = Vec::with_capacity(1024);
        let mut all_position_indices = Vec::with_capacity(8192);

        let mut all_vertices_full = PushBuffer::new(16384);
        let mut all_vertices_position = PushBuffer::new(16384);
        let mut all_indices = PushBuffer::new(16384);

        let mut mesh_parts: Vec<MeshBasicPartAssetData> =
            Vec::with_capacity(mesh.primitives().len());

        //
        // Iterate all mesh parts, building a single vertex and index buffer. Each MeshPart will
        // hold offsets/lengths to their sections in the vertex/index buffers
        //
        for primitive in mesh.primitives() {
            let mesh_part = {
                let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|x| &**x));

                let positions = reader.read_positions();
                let normals = reader.read_normals();
                let tex_coords = reader.read_tex_coords(0);
                let indices = reader.read_indices();

                if let (Some(indices), Some(positions), Some(normals), Some(tex_coords)) =
                    (indices, positions, normals, tex_coords)
                {
                    let part_indices: Vec<u32> = indices.into_u32().collect();

                    let positions: Vec<_> = positions.collect();
                    let normals: Vec<_> = normals.collect();
                    let tex_coords: Vec<_> = tex_coords.into_f32().collect();

                    let part_data = super::util::process_mesh_part(
                        &part_indices,
                        &positions,
                        &normals,
                        &tex_coords,
                        &mut all_vertices_full,
                        &mut all_vertices_position,
                        &mut all_indices,
                    );

                    //
                    // Positions and indices for the visibility system
                    //
                    for index in part_indices {
                        all_position_indices.push(index as u32);
                    }

                    for i in 0..positions.len() {
                        all_positions.push(Vec3::new(
                            positions[i][0],
                            positions[i][1],
                            positions[i][2],
                        ));
                    }

                    let mesh_material = if let Some(material_index) = primitive.material().index() {
                        mesh_material_index_to_handle[material_index].clone()
                    } else {
                        return Err(distill::importer::Error::Boxed(Box::new(
                            GltfImportError::new("A mesh primitive did not have a material"),
                        )));
                    };

                    Some(MeshBasicPartAssetData {
                        mesh_material,
                        vertex_full_buffer_offset_in_bytes: part_data
                            .vertex_full_buffer_offset_in_bytes,
                        vertex_full_buffer_size_in_bytes: part_data
                            .vertex_full_buffer_size_in_bytes,
                        vertex_position_buffer_offset_in_bytes: part_data
                            .vertex_position_buffer_offset_in_bytes,
                        vertex_position_buffer_size_in_bytes: part_data
                            .vertex_position_buffer_size_in_bytes,
                        index_buffer_offset_in_bytes: part_data.index_buffer_offset_in_bytes,
                        index_buffer_size_in_bytes: part_data.index_buffer_size_in_bytes,
                        index_type: part_data.index_type,
                    })
                } else {
                    log::error!(
                        "Mesh primitives must specify indices, positions, normals, tangents, and tex_coords"
                    );

                    return Err(distill::importer::Error::Boxed(Box::new(
                        GltfImportError::new("Mesh primitives must specify indices, positions, normals, tangents, and tex_coords"),
                    )));
                }
            };

            if let Some(mesh_part) = mesh_part {
                mesh_parts.push(mesh_part);
            }
        }

        //
        // Vertex Full Buffer
        //
        let vertex_full_buffer_asset = BufferAssetData {
            resource_type: RafxResourceType::VERTEX_BUFFER,
            data: all_vertices_full.into_data(),
        };

        let vertex_full_buffer_id = GltfObjectId::Index(buffers_to_import.len());
        let vertex_full_buffer_to_import = BufferToImport {
            asset: vertex_full_buffer_asset,
            id: vertex_full_buffer_id.clone(),
        };

        let vertex_full_buffer_uuid = *state
            .buffer_asset_uuids
            .entry(vertex_full_buffer_id)
            .or_insert_with(|| op.new_asset_uuid());

        buffers_to_import.push(vertex_full_buffer_to_import);

        let vertex_full_buffer_handle = make_handle(vertex_full_buffer_uuid);

        //
        // Vertex Position Buffer
        //
        let vertex_position_buffer_asset = BufferAssetData {
            resource_type: RafxResourceType::VERTEX_BUFFER,
            data: all_vertices_position.into_data(),
        };

        let vertex_position_buffer_id = GltfObjectId::Index(buffers_to_import.len());
        let vertex_position_buffer_to_import = BufferToImport {
            asset: vertex_position_buffer_asset,
            id: vertex_position_buffer_id.clone(),
        };

        let vertex_position_buffer_uuid = *state
            .buffer_asset_uuids
            .entry(vertex_position_buffer_id)
            .or_insert_with(|| op.new_asset_uuid());

        buffers_to_import.push(vertex_position_buffer_to_import);

        let vertex_position_buffer_handle = make_handle(vertex_position_buffer_uuid);

        //
        // Index Buffer
        //
        let index_buffer_asset = BufferAssetData {
            resource_type: RafxResourceType::INDEX_BUFFER,
            data: all_indices.into_data(),
        };

        let index_buffer_id = GltfObjectId::Index(buffers_to_import.len());
        let index_buffer_to_import = BufferToImport {
            asset: index_buffer_asset,
            id: index_buffer_id.clone(),
        };

        let index_buffer_uuid = *state
            .buffer_asset_uuids
            .entry(index_buffer_id)
            .or_insert_with(|| op.new_asset_uuid());

        buffers_to_import.push(index_buffer_to_import);

        let index_buffer_handle = make_handle(index_buffer_uuid);

        let mesh_data = PolygonSoup {
            vertex_positions: all_positions,
            index: PolygonSoupIndex::Indexed32(all_position_indices),
        };

        let asset = MeshBasicAssetData {
            mesh_parts,
            vertex_full_buffer: vertex_full_buffer_handle,
            vertex_position_buffer: vertex_position_buffer_handle,
            index_buffer: index_buffer_handle,
            visible_bounds: VisibleBounds::from(mesh_data),
        };

        let mesh_id = mesh
            .name()
            .map(|s| GltfObjectId::Name(s.to_string()))
            .unwrap_or_else(|| GltfObjectId::Index(mesh.index()));

        let mesh_to_import = MeshToImport { id: mesh_id, asset };

        // Verify that we iterate meshes in order so that our resulting assets are in order
        assert!(mesh.index() == meshes_to_import.len());
        log::trace!(
            "Importing Mesh name: {:?} index: {} mesh_parts count: {}",
            mesh.name(),
            mesh.index(),
            mesh_to_import.asset.mesh_parts.len()
        );

        meshes_to_import.push(mesh_to_import);
    }

    Ok((meshes_to_import, buffers_to_import))
}
