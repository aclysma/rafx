use crate::assets::mesh_adv::MeshAdvMaterialData;
use crate::schema::{
    MeshAdvMaterialAssetAccessor, MeshAdvMeshAssetAccessor, MeshAdvMeshImportedDataAccessor,
};
use fnv::FnvHashMap;
use gltf::buffer::Data as GltfBufferData;
use hydrate_base::handle::Handle;
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{DataContainerRefMut, RecordAccessor, SchemaSet};
use hydrate_pipeline::{
    AssetPlugin, BuilderRegistryBuilder, ImportContext, ImportableAsset, ImportedImportable,
    ImporterRegistry, ImporterRegistryBuilder, JobProcessorRegistryBuilder, ScanContext,
    ScannedImportable, SchemaLinker,
};
use rafx::assets::schema::{GpuImageAssetAccessor, GpuImageImportedDataAccessor};
use rafx::assets::PushBuffer;
use rafx::assets::{GpuImageImporterSimple, ImageAsset, ImageImporterOptions};
use rafx::assets::{ImageAssetColorSpaceConfig, ImageAssetData};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
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

// #[derive(Debug)]
// struct GltfImportError {
//     error_message: String,
// }
//
// impl GltfImportError {
//     pub fn new(error_message: &str) -> Self {
//         GltfImportError {
//             error_message: error_message.to_string(),
//         }
//     }
// }

// impl std::error::Error for GltfImportError {}
//
// impl std::fmt::Display for GltfImportError {
//     fn fmt(
//         &self,
//         f: &mut std::fmt::Formatter<'_>,
//     ) -> std::fmt::Result {
//         write!(f, "{}", self.error_message)
//     }
// }

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum GltfObjectId {
    Name(String),
    Index(usize),
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

fn hydrate_import_image(
    asset_name: &str,
    schema_set: &SchemaSet,
    image: &gltf::Image,
    images: &Vec<gltf::image::Data>,
    imported_objects: &mut HashMap<Option<String>, ImportedImportable>,
    image_color_space_assignments: &FnvHashMap<usize, ImageAssetColorSpaceConfig>,
) {
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
        let mut default_asset_object =
            GpuImageAssetAccessor::new_single_object(schema_set).unwrap();
        let mut default_asset_data_container =
            DataContainerRefMut::from_single_object(&mut default_asset_object, schema_set);
        let x = GpuImageAssetAccessor::default();

        GpuImageImporterSimple::set_default_asset_properties(
            &default_settings,
            &mut default_asset_data_container,
            &x,
        );

        default_asset_object
    };

    let import_data = {
        let mut import_data = GpuImageImportedDataAccessor::new_single_object(schema_set).unwrap();
        let mut import_data_container =
            DataContainerRefMut::from_single_object(&mut import_data, schema_set);
        let x = GpuImageImportedDataAccessor::default();

        x.image_bytes()
            .set(
                &mut import_data_container,
                Arc::new(converted_image.to_vec()),
            )
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
}

fn hydrate_import_material(
    asset_name: &str,
    schema_set: &SchemaSet,
    material: &gltf::Material,
    imported_objects: &mut HashMap<Option<String>, ImportedImportable>,
    image_object_ids: &HashMap<usize, AssetId>,
) {
    //
    // Create the default asset
    //
    let default_asset = {
        let mut default_asset_object =
            MeshAdvMaterialAssetAccessor::new_single_object(schema_set).unwrap();
        let mut default_asset_data_container =
            DataContainerRefMut::from_single_object(&mut default_asset_object, schema_set);
        let x = MeshAdvMaterialAssetAccessor::default();
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
}

fn hydrate_import_mesh(
    asset_name: &str,
    buffers: &[GltfBufferData],
    schema_set: &SchemaSet,
    mesh: &gltf::Mesh,
    imported_objects: &mut HashMap<Option<String>, ImportedImportable>,
    material_index_to_asset_id: &HashMap<Option<usize>, AssetId>,
) {
    //
    // Set up material slots (we find unique materials in this mesh and assign them a slot
    //
    let mut material_slots: Vec<AssetId> = Vec::default();
    let mut material_slots_lookup: HashMap<Option<usize>, u32> = HashMap::default();
    for primitive in mesh.primitives() {
        let material_index = primitive.material().index();
        //primitive.material()

        if !material_slots_lookup.contains_key(&material_index) {
            let slot_index = material_slots.len() as u32;
            //TODO: This implies we always import materials, we'd need a different way to get the AssetId
            // of an already imported material from this file.
            material_slots.push(*material_index_to_asset_id.get(&material_index).unwrap());
            material_slots_lookup.insert(material_index, slot_index);
        }
    }

    //
    // Create the asset (mainly we create a list of material slots referencing the appropriate material asset)
    //
    let default_asset = {
        let mut default_asset_object =
            MeshAdvMeshAssetAccessor::new_single_object(schema_set).unwrap();
        let mut default_asset_data_container =
            DataContainerRefMut::from_single_object(&mut default_asset_object, schema_set);
        let x = MeshAdvMeshAssetAccessor::default();

        for material_slot in material_slots {
            let entry = x
                .material_slots()
                .add_entry(&mut default_asset_data_container)
                .unwrap();
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
    let mut import_data = MeshAdvMeshImportedDataAccessor::new_single_object(schema_set).unwrap();
    let mut import_data_container =
        DataContainerRefMut::from_single_object(&mut import_data, schema_set);
    let x = MeshAdvMeshImportedDataAccessor::default();

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
                panic!("A mesh primitive did not have a material");
            }

            let material_index = *material_slots_lookup
                .get(&primitive.material().index())
                .unwrap();

            // let Some(material_index) = primitive.material().index() else {
            //     return Err(distill::importer::Error::Boxed(Box::new(
            //         GltfImportError::new("A mesh primitive did not have a material"),
            //     )));
            // };

            let entry_uuid = x
                .mesh_parts()
                .add_entry(&mut import_data_container)
                .unwrap();
            let entry = x.mesh_parts().entry(entry_uuid);
            entry
                .positions()
                .set(
                    &mut import_data_container,
                    Arc::new(positions_bytes.to_vec()),
                )
                .unwrap();
            entry
                .normals()
                .set(&mut import_data_container, Arc::new(normals_bytes.to_vec()))
                .unwrap();
            entry
                .texture_coordinates()
                .set(
                    &mut import_data_container,
                    Arc::new(tex_coords_bytes.to_vec()),
                )
                .unwrap();
            entry
                .indices()
                .set(&mut import_data_container, Arc::new(part_indices_bytes))
                .unwrap();
            entry
                .material_index()
                .set(&mut import_data_container, material_index)
                .unwrap();
        } else {
            log::error!(
                "Mesh primitives must specify indices, positions, normals, tangents, and tex_coords"
            );

            panic!("Mesh primitives must specify indices, positions, normals, tangents, and tex_coords");
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

impl hydrate_pipeline::Importer for GltfImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["gltf", "glb"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        let mesh_asset_type = context
            .schema_set
            .find_named_type(MeshAdvMeshAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let material_asset_type = context
            .schema_set
            .find_named_type(MeshAdvMaterialAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let image_asset_type = context
            .schema_set
            .find_named_type(GpuImageAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let (doc, _buffers, _images) = ::gltf::import(context.path).unwrap();

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
        context: ImportContext,
        //import_info: &ImportInfo,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let (doc, buffers, images) = ::gltf::import(context.path).unwrap();

        let mut imported_objects = HashMap::default();

        let mut image_index_to_object_id = HashMap::default();
        let mut material_index_to_object_id = HashMap::default();

        let image_color_space_assignments =
            build_image_color_space_assignments_from_materials(&doc);

        for (i, image) in doc.images().enumerate() {
            let asset_name = name_or_index("image", image.name(), i);
            if let Some(importable_object) =
                context.importable_assets.get(&Some(asset_name.clone()))
            {
                image_index_to_object_id.insert(image.index(), importable_object.id);
                hydrate_import_image(
                    &asset_name,
                    context.schema_set,
                    &image,
                    &images,
                    &mut imported_objects,
                    &image_color_space_assignments,
                );
            }
        }

        for (i, material) in doc.materials().enumerate() {
            let asset_name = name_or_index("material", material.name(), i);
            if let Some(importable_object) =
                context.importable_assets.get(&Some(asset_name.clone()))
            {
                material_index_to_object_id.insert(material.index(), importable_object.id);
                hydrate_import_material(
                    &asset_name,
                    context.schema_set,
                    &material,
                    &mut imported_objects,
                    &image_index_to_object_id,
                );
            }
        }

        for (i, mesh) in doc.meshes().enumerate() {
            let asset_name = name_or_index("mesh", mesh.name(), i);
            if context
                .importable_assets
                .contains_key(&Some(asset_name.clone()))
            {
                hydrate_import_mesh(
                    &asset_name,
                    &buffers,
                    context.schema_set,
                    &mesh,
                    &mut imported_objects,
                    &material_index_to_object_id,
                );
            }
        }

        imported_objects
    }
}

pub struct GltfAssetPlugin;

impl AssetPlugin for GltfAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        _builder_registry: &mut BuilderRegistryBuilder,
        _job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<GltfImporter>();
    }
}
