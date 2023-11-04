use crate::assets::mesh_adv::{
    HydrateMeshAdvAssetData, MeshAdvAssetData, MeshAdvBufferAssetData, MeshAdvMaterialData,
    MeshAdvPartAssetData, MeshMaterialAdvAsset, MeshMaterialAdvAssetData,
};
use crate::features::mesh_adv::{MeshVertexFull, MeshVertexPosition};
use crate::schema::{
    MeshAdvMaterialAssetRecord, MeshAdvMeshAssetRecord, MeshAdvMeshImportedDataRecord,
};
use distill::core::AssetUuid;
use distill::importer::{Error, ImportOp, ImportedAsset, Importer, ImporterValue};
use distill::loader::handle::Handle;
use distill::{make_handle, make_handle_from_str};
use fnv::FnvHashMap;
use glam::Vec3;
use gltf::buffer::Data as GltfBufferData;
use gltf::image::Data as GltfImageData;
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{DataContainerMut, Record, SchemaSet, SingleObject};
use hydrate_model::{
    AssetPlugin, BuilderRegistryBuilder, ImportableObject, ImportedImportable, ImporterRegistry,
    ImporterRegistryBuilder, JobProcessorRegistryBuilder, ScannedImportable, SchemaLinker,
};
use itertools::Itertools;
use rafx::api::RafxResourceType;
use rafx::assets::schema::{GpuImageAssetRecord, GpuImageImportedDataRecord};
use rafx::assets::PushBuffer;
use rafx::assets::{GpuImageImporterSimple, ImageAsset, ImageImporterOptions};
use rafx::assets::{ImageAssetColorSpaceConfig, ImageAssetData};
use rafx::rafx_visibility::{PolygonSoup, PolygonSoupIndex, VisibleBounds};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
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
pub struct MeshAdvGltfMaterialImportData {
    //pub name: Option<String>,
    pub material_data: MeshAdvMaterialData,

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
    asset: MeshAdvGltfMaterialImportData,
}

struct MeshToImport {
    id: GltfObjectId,
    asset: MeshAdvAssetData,
}

struct HydrateMeshToImport {
    id: GltfObjectId,
    asset: HydrateMeshAdvAssetData,
}

struct BufferToImport {
    id: GltfObjectId,
    asset: MeshAdvBufferAssetData,
}

// The asset state is stored in this format using Vecs
#[derive(TypeUuid, Serialize, Deserialize, Default, Clone)]
#[uuid = "980d063c-1923-42f8-b4fa-b819fbfd8a5e"]
pub struct MeshAdvGltfImporterStateStable {
    // Asset UUIDs for imported image by name. We use vecs here so we can sort by UUID for
    // deterministic output
    buffer_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
    image_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
    material_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
    material_instance_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
    mesh_material_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
    mesh_asset_uuids: Vec<(GltfObjectId, AssetUuid)>,
}

impl From<MeshAdvGltfImporterStateUnstable> for MeshAdvGltfImporterStateStable {
    fn from(other: MeshAdvGltfImporterStateUnstable) -> Self {
        let mut stable = MeshAdvGltfImporterStateStable::default();
        stable.buffer_asset_uuids = other
            .buffer_asset_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.image_asset_uuids = other
            .image_asset_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.material_asset_uuids = other
            .material_asset_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.material_instance_asset_uuids = other
            .material_instance_asset_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.mesh_material_asset_uuids = other
            .mesh_material_asset_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.mesh_asset_uuids = other
            .mesh_asset_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable
    }
}

#[derive(Default)]
pub struct MeshAdvGltfImporterStateUnstable {
    //asset_uuid: Option<AssetUuid>,

    // Asset UUIDs for imported image by name
    buffer_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    image_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    material_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    material_instance_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    mesh_material_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
    mesh_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
}

impl From<MeshAdvGltfImporterStateStable> for MeshAdvGltfImporterStateUnstable {
    fn from(other: MeshAdvGltfImporterStateStable) -> Self {
        let mut unstable = MeshAdvGltfImporterStateUnstable::default();
        unstable.buffer_asset_uuids = other.buffer_asset_uuids.into_iter().collect();
        unstable.image_asset_uuids = other.image_asset_uuids.into_iter().collect();
        unstable.material_asset_uuids = other.material_asset_uuids.into_iter().collect();
        unstable.material_instance_asset_uuids =
            other.material_instance_asset_uuids.into_iter().collect();
        unstable.mesh_material_asset_uuids = other.mesh_material_asset_uuids.into_iter().collect();
        unstable.mesh_asset_uuids = other.mesh_asset_uuids.into_iter().collect();
        unstable
    }
}

#[derive(TypeUuid)]
#[uuid = "75a6991e-5c0a-4038-960a-6a391eeba766"]
pub struct MeshAdvGltfImporter;
impl Importer for MeshAdvGltfImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        29
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MeshAdvGltfImporterStateStable;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        stable_state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let mut unstable_state: MeshAdvGltfImporterStateUnstable = stable_state.clone().into();

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

        let (doc, buffers, images) = result.unwrap();

        // Accumulate everything we will import in this list
        let mut imported_assets = Vec::new();

        let image_color_space_assignments =
            build_image_color_space_assignments_from_materials(&doc);

        //
        // Images
        //
        let images_to_import =
            extract_images_to_import(&doc, &buffers, &images, &image_color_space_assignments);
        let mut image_index_to_handle = vec![];
        for image_to_import in images_to_import {
            // Find the UUID associated with this image or create a new one
            let image_uuid = *unstable_state
                .image_asset_uuids
                .entry(image_to_import.id.clone())
                .or_insert_with(|| op.new_asset_uuid());

            let image_handle = make_handle(image_uuid);
            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            image_index_to_handle.push(image_handle);

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &image_to_import.id {
                search_tags.push(("name".to_string(), Some(name.clone())));
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

        let material_handle = make_handle_from_str("680c6edd-8bed-407b-aea0-d0f6056093d6")?;

        //
        // Material instance
        //
        let mut mesh_material_index_to_handle = vec![];
        for material_to_import in &materials_to_import {
            //
            // Push the material instance UUID into the list so that we have an O(1) lookup material index to UUID
            //
            let _material_instance_uuid = *unstable_state
                .material_instance_asset_uuids
                .entry(material_to_import.id.clone())
                .or_insert_with(|| op.new_asset_uuid());

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &material_to_import.id {
                search_tags.push(("name".to_string(), Some(name.clone())));
            }

            //
            // Create the material instance
            //
            let material_data = &material_to_import.asset.material_data;

            //
            // Create the mesh material
            //

            //
            // Push the mesh material UUID into the list so that we have an O(1) lookup material index to UUID
            //
            let mesh_material_uuid = *unstable_state
                .mesh_material_asset_uuids
                .entry(material_to_import.id.clone())
                .or_insert_with(|| op.new_asset_uuid());

            let mesh_material_handle = make_handle(mesh_material_uuid);

            mesh_material_index_to_handle.push(mesh_material_handle);

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &material_to_import.id {
                search_tags.push(("name".to_string(), Some(name.clone())));
            }

            let mesh_material_asset = MeshMaterialAdvAssetData {
                material_data: material_data.clone(),
                material_asset: material_handle.clone(),
                color_texture: material_to_import.asset.base_color_texture.clone(),
                metallic_roughness_texture: material_to_import
                    .asset
                    .metallic_roughness_texture
                    .clone(),
                normal_texture: material_to_import.asset.normal_texture.clone(),
                emissive_texture: material_to_import.asset.emissive_texture.clone(),
            };

            imported_assets.push(ImportedAsset {
                id: mesh_material_uuid,
                search_tags,
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(mesh_material_asset),
            });
        }

        //
        // Meshes
        //
        let (meshes_to_import, buffers_to_import) = extract_meshes_to_import(
            op,
            &mut unstable_state,
            &doc,
            &buffers,
            &mesh_material_index_to_handle,
        )?;

        let mut buffer_index_to_handle = vec![];
        for buffer_to_import in buffers_to_import {
            // Find the UUID associated with this image or create a new one
            let buffer_uuid = *unstable_state
                .buffer_asset_uuids
                .entry(buffer_to_import.id.clone())
                .or_insert_with(|| op.new_asset_uuid());

            let buffer_handle = make_handle::<MeshAdvBufferAssetData>(buffer_uuid);

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
            let mesh_uuid = *unstable_state
                .mesh_asset_uuids
                .entry(mesh_to_import.id.clone())
                .or_insert_with(|| op.new_asset_uuid());

            let mut search_tags: Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &mesh_to_import.id {
                search_tags.push(("name".to_string(), Some(name.clone())));
            }

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

        *stable_state = unstable_state.into();

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}

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
        let mut material_asset = MeshAdvGltfMaterialImportData::default();

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
    state: &mut MeshAdvGltfImporterStateUnstable,
    doc: &gltf::Document,
    buffers: &[GltfBufferData],
    mesh_material_index_to_handle: &[Handle<MeshMaterialAdvAsset>],
) -> distill::importer::Result<(Vec<MeshToImport>, Vec<BufferToImport>)> {
    let mut meshes_to_import = Vec::with_capacity(doc.meshes().len());
    let mut buffers_to_import = Vec::with_capacity(doc.meshes().len() * 2);

    for mesh in doc.meshes() {
        let mut all_positions = Vec::with_capacity(1024);
        let mut all_position_indices = Vec::with_capacity(8192);

        let mut all_vertices_full = PushBuffer::new(16384);
        let mut all_vertices_position = PushBuffer::new(16384);
        let mut all_indices = PushBuffer::new(16384);

        let mut mesh_parts: Vec<MeshAdvPartAssetData> = Vec::with_capacity(mesh.primitives().len());

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

                    let part_data = super::mesh_util::process_mesh_part(
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

                    Some(MeshAdvPartAssetData {
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
        let vertex_full_buffer_asset = MeshAdvBufferAssetData {
            resource_type: RafxResourceType::VERTEX_BUFFER,
            alignment: std::mem::size_of::<MeshVertexFull>() as u32,
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
        let vertex_position_buffer_asset = MeshAdvBufferAssetData {
            resource_type: RafxResourceType::VERTEX_BUFFER,
            alignment: std::mem::size_of::<MeshVertexPosition>() as u32,
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
        let index_buffer_asset = MeshAdvBufferAssetData {
            resource_type: RafxResourceType::INDEX_BUFFER,
            alignment: std::mem::size_of::<u32>() as u32,
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

        let asset = MeshAdvAssetData {
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
        log::debug!(
            "Importing Mesh name: {:?} index: {} mesh_parts count: {}",
            mesh.name(),
            mesh.index(),
            mesh_to_import.asset.mesh_parts.len()
        );

        meshes_to_import.push(mesh_to_import);
    }

    Ok((meshes_to_import, buffers_to_import))
}

fn hydrate_import_image(
    asset_name: &str,
    schema_set: &SchemaSet,
    image: &gltf::Image,
    images: &Vec<gltf::image::Data>,
    imported_objects: &mut HashMap<Option<String>, ImportedImportable>,
    image_color_space_assignments: &FnvHashMap<usize, ImageAssetColorSpaceConfig>,
) -> distill::importer::Result<()> {
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

    let (data_format, mip_generation) = ImageAssetData::default_format_and_mip_generation();
    let default_settings = ImageImporterOptions {
        mip_generation,
        color_space,
        data_format,
    };

    //
    // Create the default asset
    //
    let default_asset = {
        let mut default_asset_object = GpuImageAssetRecord::new_single_object(schema_set).unwrap();
        let mut default_asset_data_container =
            DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
        let x = GpuImageAssetRecord::default();

        GpuImageImporterSimple::set_default_asset_properties(
            &default_settings,
            &mut default_asset_data_container,
            &x,
        );

        default_asset_object
    };

    let import_data = {
        let mut import_data = GpuImageImportedDataRecord::new_single_object(schema_set).unwrap();
        let mut import_data_container =
            DataContainerMut::new_single_object(&mut import_data, schema_set);
        let x = GpuImageImportedDataRecord::default();

        x.image_bytes()
            .set(&mut import_data_container, converted_image.to_vec())
            .unwrap();
        x.width()
            .set(&mut import_data_container, converted_image.width())
            .unwrap();
        x.height()
            .set(&mut import_data_container, converted_image.height())
            .unwrap();

        import_data
    };

    //
    // Return the created objects
    //
    imported_objects.insert(
        Some(asset_name.to_string()),
        ImportedImportable {
            file_references: Default::default(),
            import_data: Some(import_data),
            default_asset: Some(default_asset),
        },
    );

    Ok(())
}

fn hydrate_import_material(
    asset_name: &str,
    schema_set: &SchemaSet,
    material: &gltf::Material,
    imported_objects: &mut HashMap<Option<String>, ImportedImportable>,
    image_object_ids: &HashMap<usize, ObjectId>,
) -> distill::importer::Result<()> {
    //
    // Create the default asset
    //
    let default_asset = {
        let mut default_asset_object =
            MeshAdvMaterialAssetRecord::new_single_object(schema_set).unwrap();
        let mut default_asset_data_container =
            DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
        let x = MeshAdvMaterialAssetRecord::default();
        x.base_color_factor()
            .set_vec4(
                &mut default_asset_data_container,
                material.pbr_metallic_roughness().base_color_factor(),
            )
            .unwrap();
        x.emissive_factor()
            .set_vec3(
                &mut default_asset_data_container,
                material.emissive_factor(),
            )
            .unwrap();
        x.metallic_factor()
            .set(
                &mut default_asset_data_container,
                material.pbr_metallic_roughness().metallic_factor(),
            )
            .unwrap();
        x.roughness_factor()
            .set(
                &mut default_asset_data_container,
                material.pbr_metallic_roughness().roughness_factor(),
            )
            .unwrap();
        x.normal_texture_scale()
            .set(
                &mut default_asset_data_container,
                material.normal_texture().map_or(1.0, |x| x.scale()),
            )
            .unwrap();

        if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
            let texture_index = texture.texture().index();
            let texture_object_id = image_object_ids[&texture_index];
            x.color_texture()
                .set(&mut default_asset_data_container, texture_object_id)
                .unwrap();
        }

        if let Some(texture) = material
            .pbr_metallic_roughness()
            .metallic_roughness_texture()
        {
            let texture_index = texture.texture().index();
            let texture_object_id = image_object_ids[&texture_index];
            x.metallic_roughness_texture()
                .set(&mut default_asset_data_container, texture_object_id)
                .unwrap();
        }

        if let Some(texture) = material.normal_texture() {
            let texture_index = texture.texture().index();
            let texture_object_id = image_object_ids[&texture_index];
            x.normal_texture()
                .set(&mut default_asset_data_container, texture_object_id)
                .unwrap();
        }

        if let Some(texture) = material.emissive_texture() {
            let texture_index = texture.texture().index();
            let texture_object_id = image_object_ids[&texture_index];
            x.emissive_texture()
                .set(&mut default_asset_data_container, texture_object_id)
                .unwrap();
        }

        if let Some(texture) = material.occlusion_texture() {
            let texture_index = texture.texture().index();
            let texture_object_id = image_object_ids[&texture_index];
            x.occlusion_texture()
                .set(&mut default_asset_data_container, texture_object_id)
                .unwrap();
        }

        //x.shadow_method()

        //x.shadow_method().set(&mut default_asset_data_container, shadow_method).unwrap();
        //x.blend_method().set(&mut default_asset_data_container, blend_method).unwrap();
        x.alpha_threshold()
            .set(
                &mut default_asset_data_container,
                material.alpha_cutoff().unwrap_or(0.5),
            )
            .unwrap();
        x.backface_culling()
            .set(&mut default_asset_data_container, false)
            .unwrap();
        //TODO: Does this incorrectly write older enum string names when code is older than schema file?
        x.color_texture_has_alpha_channel()
            .set(&mut default_asset_data_container, false)
            .unwrap();
        default_asset_object
    };

    //
    // Return the created objects
    //
    imported_objects.insert(
        Some(asset_name.to_string()),
        ImportedImportable {
            file_references: Default::default(),
            import_data: None,
            default_asset: Some(default_asset),
        },
    );

    Ok(())
}

fn hydrate_import_mesh(
    asset_name: &str,
    buffers: &[GltfBufferData],
    schema_set: &SchemaSet,
    mesh: &gltf::Mesh,
    imported_objects: &mut HashMap<Option<String>, ImportedImportable>,
    material_index_to_object_id: &HashMap<Option<usize>, ObjectId>,
) -> distill::importer::Result<()> {
    //
    // Set up material slots (we find unique materials in this mesh and assign them a slot
    //
    let mut material_slots: Vec<ObjectId> = Vec::default();
    let mut material_slots_lookup: HashMap<Option<usize>, u32> = HashMap::default();
    for primitive in mesh.primitives() {
        let material_index = primitive.material().index();
        //primitive.material()

        if !material_slots_lookup.contains_key(&material_index) {
            let slot_index = material_slots.len() as u32;
            //TODO: This implies we always import materials, we'd need a different way to get the ObjectId
            // of an already imported material from this file.
            material_slots.push(*material_index_to_object_id.get(&material_index).unwrap());
            material_slots_lookup.insert(material_index, slot_index);
        }
    }

    //
    // Create the asset (mainly we create a list of material slots referencing the appropriate material asset)
    //
    let default_asset = {
        let mut default_asset_object =
            MeshAdvMeshAssetRecord::new_single_object(schema_set).unwrap();
        let mut default_asset_data_container =
            DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
        let x = MeshAdvMeshAssetRecord::default();

        for material_slot in material_slots {
            let entry = x
                .material_slots()
                .add_entry(&mut default_asset_data_container);
            x.material_slots()
                .entry(entry)
                .set(&mut default_asset_data_container, material_slot)
                .unwrap();
        }

        default_asset_object
    };

    //
    // Create import data
    //
    let mut import_data = MeshAdvMeshImportedDataRecord::new_single_object(schema_set).unwrap();
    let mut import_data_container =
        DataContainerMut::new_single_object(&mut import_data, schema_set);
    let x = MeshAdvMeshImportedDataRecord::default();

    //
    // Iterate all mesh parts, building a single vertex and index buffer. Each MeshPart will
    // hold offsets/lengths to their sections in the vertex/index buffers
    //
    for primitive in mesh.primitives() {
        let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|x| &**x));

        let positions = reader.read_positions();
        let normals = reader.read_normals();
        let tex_coords = reader.read_tex_coords(0);
        let indices = reader.read_indices();

        if let (Some(indices), Some(positions), Some(normals), Some(tex_coords)) =
            (indices, positions, normals, tex_coords)
        {
            let part_indices: Vec<u32> = indices.into_u32().collect();
            let part_indices_bytes = PushBuffer::from_vec(&part_indices).into_data();

            let positions: Vec<_> = positions.collect();
            let positions_bytes = PushBuffer::from_vec(&positions).into_data();
            let normals: Vec<_> = normals.collect();
            let normals_bytes = PushBuffer::from_vec(&normals).into_data();
            let tex_coords: Vec<_> = tex_coords.into_f32().collect();
            let tex_coords_bytes = PushBuffer::from_vec(&tex_coords).into_data();

            if primitive.material().index().is_none() {
                return Err(distill::importer::Error::Boxed(Box::new(
                    GltfImportError::new("A mesh primitive did not have a material"),
                )));
            }

            let material_index = *material_slots_lookup
                .get(&primitive.material().index())
                .unwrap();

            // let Some(material_index) = primitive.material().index() else {
            //     return Err(distill::importer::Error::Boxed(Box::new(
            //         GltfImportError::new("A mesh primitive did not have a material"),
            //     )));
            // };

            let entry_uuid = x.mesh_parts().add_entry(&mut import_data_container);
            let entry = x.mesh_parts().entry(entry_uuid);
            entry
                .positions()
                .set(&mut import_data_container, positions_bytes.to_vec())
                .unwrap();
            entry
                .normals()
                .set(&mut import_data_container, normals_bytes.to_vec())
                .unwrap();
            entry
                .texture_coordinates()
                .set(&mut import_data_container, tex_coords_bytes.to_vec())
                .unwrap();
            entry
                .indices()
                .set(&mut import_data_container, part_indices_bytes)
                .unwrap();
            entry
                .material_index()
                .set(&mut import_data_container, material_index)
                .unwrap();
        } else {
            log::error!(
                "Mesh primitives must specify indices, positions, normals, tangents, and tex_coords"
            );

            return Err(distill::importer::Error::Boxed(Box::new(
                GltfImportError::new("Mesh primitives must specify indices, positions, normals, tangents, and tex_coords"),
            )));
        }
    }

    // let mesh_id = mesh
    //     .name()
    //     .map(|s| GltfObjectId::Name(s.to_string()))
    //     .unwrap_or_else(|| GltfObjectId::Index(mesh.index()));
    //
    // let mesh_to_import = MeshToImport { id: mesh_id, asset };
    //
    // // Verify that we iterate meshes in order so that our resulting assets are in order
    // assert!(mesh.index() == meshes_to_import.len());

    log::debug!(
        "Importing Mesh name: {:?} index: {}",
        mesh.name(),
        mesh.index(),
    );

    //meshes_to_import.push(mesh_to_import);

    imported_objects.insert(
        Some(asset_name.to_string()),
        ImportedImportable {
            file_references: Default::default(),
            import_data: Some(import_data),
            default_asset: Some(default_asset),
        },
    );

    Ok(())
}

fn name_or_index(
    prefix: &str,
    name: Option<&str>,
    index: usize,
) -> String {
    if let Some(name) = name {
        format!("{}_{}", prefix, name)
    } else {
        format!("{}_{}", prefix, index)
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "01d71c49-867c-4d96-ad16-7c08b6cbfaf9"]
pub struct GltfImporter;

impl hydrate_model::Importer for GltfImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["gltf", "glb"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
        importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        let mesh_asset_type = schema_set
            .find_named_type(MeshAdvMeshAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let material_asset_type = schema_set
            .find_named_type(MeshAdvMaterialAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let image_asset_type = schema_set
            .find_named_type(GpuImageAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let (doc, buffers, images) = ::gltf::import(path).unwrap();

        let mut importables = Vec::default();

        let mut uses_default_material = false;
        for (i, mesh) in doc.meshes().enumerate() {
            let name = name_or_index("mesh", mesh.name(), i);

            for primitive in mesh.primitives() {
                if primitive.material().index().is_none() {
                    uses_default_material = true;
                    break;
                }
            }

            importables.push(ScannedImportable {
                name: Some(name),
                asset_type: mesh_asset_type.clone(),
                file_references: Default::default(),
            });
        }

        for (i, material) in doc.materials().enumerate() {
            let name = name_or_index("material", material.name(), i);

            importables.push(ScannedImportable {
                name: Some(name),
                asset_type: material_asset_type.clone(),
                file_references: Default::default(),
            });
        }

        for (i, image) in doc.images().enumerate() {
            let name = name_or_index("image", image.name(), i);

            importables.push(ScannedImportable {
                name: Some(name),
                asset_type: image_asset_type.clone(),
                file_references: Default::default(),
            });
        }

        if uses_default_material {
            //TODO: Warn?
            let name = "material__default_material".to_string();

            importables.push(ScannedImportable {
                name: Some(name),
                asset_type: material_asset_type.clone(),
                file_references: Default::default(),
            });
        }

        importables
    }

    fn import_file(
        &self,
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
        //import_info: &ImportInfo,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let (doc, buffers, images) = ::gltf::import(path).unwrap();

        let mut imported_objects = HashMap::default();

        let mut image_index_to_object_id = HashMap::default();
        let mut material_index_to_object_id = HashMap::default();

        let image_color_space_assignments =
            build_image_color_space_assignments_from_materials(&doc);

        for (i, image) in doc.images().enumerate() {
            let asset_name = name_or_index("image", image.name(), i);
            if let Some(importable_object) = importable_objects.get(&Some(asset_name.clone())) {
                image_index_to_object_id.insert(image.index(), importable_object.id);
                hydrate_import_image(
                    &asset_name,
                    schema_set,
                    &image,
                    &images,
                    &mut imported_objects,
                    &image_color_space_assignments,
                )
                .unwrap();
            }
        }

        for (i, material) in doc.materials().enumerate() {
            let asset_name = name_or_index("material", material.name(), i);
            if let Some(importable_object) = importable_objects.get(&Some(asset_name.clone())) {
                material_index_to_object_id.insert(material.index(), importable_object.id);
                hydrate_import_material(
                    &asset_name,
                    schema_set,
                    &material,
                    &mut imported_objects,
                    &image_index_to_object_id,
                )
                .unwrap();
            }
        }

        for (i, mesh) in doc.meshes().enumerate() {
            let asset_name = name_or_index("mesh", mesh.name(), i);
            if importable_objects.contains_key(&Some(asset_name.clone())) {
                hydrate_import_mesh(
                    &asset_name,
                    &buffers,
                    schema_set,
                    &mesh,
                    &mut imported_objects,
                    &material_index_to_object_id,
                )
                .unwrap();
            }
        }

        imported_objects
    }
}

pub struct GltfAssetPlugin;

impl AssetPlugin for GltfAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<GltfImporter>(schema_linker);
    }
}
