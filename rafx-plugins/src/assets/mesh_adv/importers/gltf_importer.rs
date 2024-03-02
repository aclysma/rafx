use crate::schema::{
    MeshAdvMaterialAssetRecord, MeshAdvMeshAssetRecord, MeshAdvMeshImportedDataRecord,
};
use fnv::FnvHashMap;
use gltf::buffer::Data as GltfBufferData;
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{ImportableName, Record, RecordBuilder};
use hydrate_pipeline::{
    AssetPlugin, AssetPluginSetupContext, ImportContext, Importer, PipelineResult, ScanContext,
};
use rafx::assets::schema::{GpuImageAssetRecord, GpuImageImportedDataRecord};
use rafx::assets::PushBuffer;
use rafx::assets::{GpuImageImporterSimple, ImageImporterOptions};
use rafx::assets::{ImageAssetColorSpaceConfig, ImageAssetData};
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
    context: &ImportContext,
    asset_name: &ImportableName,
    image: &gltf::Image,
    images: &Vec<gltf::image::Data>,
    image_color_space_assignments: &FnvHashMap<usize, ImageAssetColorSpaceConfig>,
) -> PipelineResult<()> {
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
        .ok_or("Could not convert image format")?
        .convert(),
        Format::R8G8 => image::ImageBuffer::<image::LumaA<u8>, Vec<u8>>::from_vec(
            image_data.width,
            image_data.height,
            image_data.pixels.clone(),
        )
        .ok_or("Could not convert image format")?
        .convert(),
        Format::R8G8B8 => image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_vec(
            image_data.width,
            image_data.height,
            image_data.pixels.clone(),
        )
        .ok_or("Could not convert image format")?
        .convert(),
        Format::R8G8B8A8 => image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
            image_data.width,
            image_data.height,
            image_data.pixels.clone(),
        )
        .ok_or("Could not convert image format")?
        .convert(),
        Format::B8G8R8 => unimplemented!(),
        Format::B8G8R8A8 => unimplemented!(),
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
    // Create import data
    //
    let import_data = GpuImageImportedDataRecord::new_builder(context.schema_set);
    import_data
        .image_bytes()
        .set(Arc::new(converted_image.to_vec()))?;
    import_data.width().set(converted_image.width())?;
    import_data.height().set(converted_image.height())?;

    //
    // Create the default asset
    //
    let mut default_asset = RecordBuilder::<GpuImageAssetRecord>::new(context.schema_set);
    GpuImageImporterSimple::set_default_asset_properties(&default_settings, &mut default_asset);

    //
    // Return the created objects
    //
    context.add_importable(
        asset_name.clone(),
        default_asset.into_inner()?,
        Some(import_data.into_inner()?),
    );
    Ok(())
}

fn hydrate_import_material(
    context: &ImportContext,
    asset_name: &ImportableName,
    material: &gltf::Material,
    image_object_ids: &HashMap<usize, AssetId>,
) -> PipelineResult<()> {
    //
    // Create the default asset
    //
    let default_asset = MeshAdvMaterialAssetRecord::new_builder(context.schema_set);

    default_asset
        .base_color_factor()
        .set_vec4(material.pbr_metallic_roughness().base_color_factor())?;
    default_asset
        .emissive_factor()
        .set_vec3(material.emissive_factor())?;
    default_asset
        .metallic_factor()
        .set(material.pbr_metallic_roughness().metallic_factor())?;
    default_asset
        .roughness_factor()
        .set(material.pbr_metallic_roughness().roughness_factor())?;
    default_asset
        .normal_texture_scale()
        .set(material.normal_texture().map_or(1.0, |x| x.scale()))?;

    if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
        let texture_index = texture.texture().index();
        let texture_object_id = image_object_ids[&texture_index];
        default_asset.color_texture().set(texture_object_id)?;
    }

    if let Some(texture) = material
        .pbr_metallic_roughness()
        .metallic_roughness_texture()
    {
        let texture_index = texture.texture().index();
        let texture_object_id = image_object_ids[&texture_index];
        default_asset
            .metallic_roughness_texture()
            .set(texture_object_id)?;
    }

    if let Some(texture) = material.normal_texture() {
        let texture_index = texture.texture().index();
        let texture_object_id = image_object_ids[&texture_index];
        default_asset.normal_texture().set(texture_object_id)?;
    }

    if let Some(texture) = material.emissive_texture() {
        let texture_index = texture.texture().index();
        let texture_object_id = image_object_ids[&texture_index];
        default_asset.emissive_texture().set(texture_object_id)?;
    }

    if let Some(texture) = material.occlusion_texture() {
        let texture_index = texture.texture().index();
        let texture_object_id = image_object_ids[&texture_index];
        default_asset.occlusion_texture().set(texture_object_id)?;
    }

    //x.shadow_method()

    //x.shadow_method().set(&mut default_asset_data_container, shadow_method)?;
    //x.blend_method().set(&mut default_asset_data_container, blend_method)?;
    default_asset
        .alpha_threshold()
        .set(material.alpha_cutoff().unwrap_or(0.5))?;
    default_asset.backface_culling().set(false)?;
    //TODO: Does this incorrectly write older enum string names when code is older than schema file?
    default_asset.color_texture_has_alpha_channel().set(false)?;

    //
    // Return the created objects
    //
    context.add_importable(asset_name.clone(), default_asset.into_inner()?, None);
    Ok(())
}

fn hydrate_import_mesh(
    context: &ImportContext,
    asset_name: &ImportableName,
    buffers: &[GltfBufferData],
    mesh: &gltf::Mesh,
    material_index_to_asset_id: &HashMap<Option<usize>, AssetId>,
) -> PipelineResult<()> {
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
    let default_asset = MeshAdvMeshAssetRecord::new_builder(context.schema_set);
    for material_slot in material_slots {
        let entry = default_asset.material_slots().add_entry()?;
        default_asset
            .material_slots()
            .entry(entry)
            .set(material_slot)?;
    }

    //
    // Create import data
    //
    let import_data = MeshAdvMeshImportedDataRecord::new_builder(context.schema_set);

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

            let entry_uuid = import_data.mesh_parts().add_entry()?;
            let entry = import_data.mesh_parts().entry(entry_uuid);
            entry.positions().set(Arc::new(positions_bytes.to_vec()))?;
            entry.normals().set(Arc::new(normals_bytes.to_vec()))?;
            entry
                .texture_coordinates()
                .set(Arc::new(tex_coords_bytes.to_vec()))?;
            entry.indices().set(Arc::new(part_indices_bytes))?;
            entry.material_index().set(material_index)?;
        } else {
            log::error!(
                "Mesh primitives must specify indices, positions, normals, tangents, and tex_coords"
            );

            panic!("Mesh primitives must specify indices, positions, normals, tangents, and tex_coords");
        }
    }

    log::trace!(
        "Importing Mesh name: {:?} index: {}",
        mesh.name(),
        mesh.index(),
    );

    context.add_importable(
        asset_name.clone(),
        default_asset.into_inner()?,
        Some(import_data.into_inner()?),
    );
    Ok(())
}

fn name_or_index(
    prefix: &str,
    name: Option<&str>,
    index: usize,
) -> ImportableName {
    if let Some(name) = name {
        ImportableName::new(format!("{}_{}", prefix, name))
    } else {
        ImportableName::new(format!("{}_{}", prefix, index))
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "01d71c49-867c-4d96-ad16-7c08b6cbfaf9"]
pub struct GltfImporter;

impl Importer for GltfImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["gltf", "glb"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        let (doc, _buffers, _images) =
            ::gltf::import(context.path).map_err(|e| format!("gltf::import error {:?}", e))?;

        let mut uses_default_material = false;
        for (i, mesh) in doc.meshes().enumerate() {
            let name = name_or_index("mesh", mesh.name(), i);

            for primitive in mesh.primitives() {
                if primitive.material().index().is_none() {
                    uses_default_material = true;
                    break;
                }
            }

            context.add_importable::<MeshAdvMeshAssetRecord>(name)?;
        }

        for (i, material) in doc.materials().enumerate() {
            let name = name_or_index("material", material.name(), i);
            context.add_importable::<MeshAdvMaterialAssetRecord>(name)?;
        }

        for (i, image) in doc.images().enumerate() {
            let name = name_or_index("image", image.name(), i);
            context.add_importable::<GpuImageAssetRecord>(name)?;
        }

        if uses_default_material {
            //TODO: Warn?
            let name = ImportableName::new("material__default_material".to_string());
            context.add_importable::<MeshAdvMaterialAssetRecord>(name)?;
        }

        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let (doc, buffers, images) =
            ::gltf::import(context.path).map_err(|e| format!("gltf::import error {:?}", e))?;

        let mut image_index_to_object_id = HashMap::default();
        let mut material_index_to_object_id = HashMap::default();

        let image_color_space_assignments =
            build_image_color_space_assignments_from_materials(&doc);

        for (i, image) in doc.images().enumerate() {
            let asset_name = name_or_index("image", image.name(), i);
            if let Some(importable_asset_id) = context.asset_id_for_importable(&asset_name) {
                image_index_to_object_id.insert(image.index(), importable_asset_id);
                hydrate_import_image(
                    &context,
                    &asset_name,
                    &image,
                    &images,
                    &image_color_space_assignments,
                )?;
            }
        }

        for (i, material) in doc.materials().enumerate() {
            let asset_name = name_or_index("material", material.name(), i);
            if let Some(importable_asset_id) = context.asset_id_for_importable(&asset_name) {
                material_index_to_object_id.insert(material.index(), importable_asset_id);
                hydrate_import_material(
                    &context,
                    &asset_name,
                    &material,
                    &image_index_to_object_id,
                )?;
            }
        }

        for (i, mesh) in doc.meshes().enumerate() {
            let asset_name = name_or_index("mesh", mesh.name(), i);
            if context.should_import(&asset_name) {
                hydrate_import_mesh(
                    &context,
                    &asset_name,
                    &buffers,
                    &mesh,
                    &material_index_to_object_id,
                )?;
            }
        }

        Ok(())
    }
}

pub struct GltfAssetPlugin;

impl AssetPlugin for GltfAssetPlugin {
    fn setup(context: AssetPluginSetupContext) {
        context.importer_registry.register_handler::<GltfImporter>();
    }
}
