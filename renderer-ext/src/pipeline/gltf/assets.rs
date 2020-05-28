use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;
use crate::pipeline::buffer::BufferAsset;
use atelier_assets::loader::handle::Handle;
use crate::pipeline::image::ImageAsset;
use crate::pipeline::pipeline::MaterialInstanceAsset;

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

// This is non-texture data associated with the material. It's appropriate to be loaded as a uniform
// for a shader
#[derive(Serialize, Deserialize, Default, Clone)]
#[repr(C)]
pub struct GltfMaterialData {
    pub base_color_factor: [f32; 4], // default: 1,1,1,1
    pub emissive_factor: [f32; 3],
    pub metallic_factor: f32, //default: 1,
    pub roughness_factor: f32, // default: 1,
    pub normal_texture_scale: f32, // default: 1
    pub occlusion_texture_strength: f32, // default 1
    pub alpha_cutoff: f32, // default 0.5
}

// We would need to change the pipeline for these
// struct GltfShaderSetting {
//     double_sided: bool, // defult false
//     alpha_mode: String, // OPAQUE, MASK, BLEND
// }

#[derive(TypeUuid, Serialize, Deserialize, Default, Clone)]
#[uuid = "130a91a8-ba80-4cad-9bce-848326b234c7"]
pub struct GltfMaterialAsset {
    //pub name: Option<String>,
    pub material_data: GltfMaterialData,

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

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[repr(packed(1))]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MeshPart {
    pub vertex_buffer_offset_in_bytes: u32,
    pub vertex_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub material: Option<Handle<GltfMaterialAsset>>,
    pub material_instance: Option<Handle<MaterialInstanceAsset>>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "cf232526-3757-4d94-98d1-c2f7e27c979f"]
pub struct MeshAsset {
    pub mesh_parts: Vec<MeshPart>,
    pub vertex_buffer: Handle<BufferAsset>, //Vec<MeshVertex>,
    pub index_buffer: Handle<BufferAsset>, //Vec<u16>,
}
