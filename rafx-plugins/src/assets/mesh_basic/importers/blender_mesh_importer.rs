use crate::assets::mesh_basic::{
    MeshBasicAssetData, MeshBasicPartAssetData, MeshMaterialBasicAsset,
};
use glam::Vec3;
use hydrate_base::handle::Handle;
use rafx::api::RafxResourceType;
use rafx::assets::BufferAssetData;
use rafx::assets::PushBuffer;
use rafx::distill::make_handle;
use rafx::rafx_visibility::{PolygonSoup, PolygonSoupIndex, VisibleBounds};
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug)]
enum MeshPartJsonIndexType {
    U16,
    U32,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshPartJson {
    #[serde(default)]
    pub position: Option<u32>,
    #[serde(default)]
    pub normal: Option<u32>,
    #[serde(default)]
    pub tangent: Option<u32>,
    #[serde(default)]
    pub uv: Vec<u32>,
    pub indices: u32,
    pub index_type: MeshPartJsonIndexType,
    pub material: Handle<MeshMaterialBasicAsset>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshJson {
    pub mesh_parts: Vec<MeshPartJson>,
}

fn try_cast_u8_slice<T: Copy + 'static>(data: &[u8]) -> Option<&[T]> {
    if data.len() % std::mem::size_of::<T>() != 0 {
        return None;
    }

    let ptr = data.as_ptr() as *const T;
    if ptr as usize % std::mem::align_of::<T>() != 0 {
        return None;
    }

    let casted: &[T] =
        unsafe { std::slice::from_raw_parts(ptr, data.len() / std::mem::size_of::<T>()) };

    Some(casted)
}
//
// #[derive(TypeUuid, Serialize, Deserialize, Default)]
// #[uuid = "1411cdbc-d63f-45aa-b9cf-adf610e43989"]
// pub struct MeshBasicBlenderImporterState {
//     mesh_id: Option<AssetUuid>,
//     vertex_full_buffer_id: Option<AssetUuid>,
//     vertex_position_buffer_id: Option<AssetUuid>,
//     index_buffer_id: Option<AssetUuid>,
// }
//
// #[derive(TypeUuid)]
// #[uuid = "f3486ab5-a780-4251-ab55-1000c638bc08"]
// pub struct MeshBasicBlenderImporter;
// impl Importer for MeshBasicBlenderImporter {
//     fn version_static() -> u32
//     where
//         Self: Sized,
//     {
//         6
//     }
//
//     fn version(&self) -> u32 {
//         Self::version_static()
//     }
//
//     type Options = ();
//
//     type State = MeshBasicBlenderImporterState;
//
//     /// Reads the given bytes and produces assets.
//     #[profiling::function]
//     fn import(
//         &self,
//         _op: &mut ImportOp,
//         source: &mut dyn Read,
//         _options: &Self::Options,
//         state: &mut Self::State,
//     ) -> distill::importer::Result<ImporterValue> {
//         let mesh_id = state
//             .mesh_id
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         let vertex_full_buffer_id = state
//             .vertex_full_buffer_id
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         let vertex_position_buffer_id = state
//             .vertex_position_buffer_id
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         let index_buffer_id = state
//             .index_buffer_id
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         *state = MeshBasicBlenderImporterState {
//             mesh_id: Some(mesh_id),
//             vertex_full_buffer_id: Some(vertex_full_buffer_id),
//             vertex_position_buffer_id: Some(vertex_position_buffer_id),
//             index_buffer_id: Some(index_buffer_id),
//         };
//         let mut bytes = Vec::new();
//         source.read_to_end(&mut bytes)?;
//
//         let b3f_reader = rafx::base::b3f::B3FReader::new(&bytes)
//             .ok_or("Blender Mesh Import error, mesh file format not recognized")?;
//         let mesh_as_json: MeshJson =
//             serde_json::from_slice(b3f_reader.get_block(0)).map_err(|e| e.to_string())?;
//
//         let mut all_positions = Vec::<glam::Vec3>::with_capacity(1024);
//         let mut all_position_indices = Vec::<u32>::with_capacity(8192);
//
//         let mut all_vertices_full = PushBuffer::new(16384);
//         let mut all_vertices_position = PushBuffer::new(16384);
//         let mut all_indices = PushBuffer::new(16384);
//
//         let mut mesh_parts: Vec<MeshBasicPartAssetData> =
//             Vec::with_capacity(mesh_as_json.mesh_parts.len());
//
//         for mesh_part in &mesh_as_json.mesh_parts {
//             //
//             // Get byte slices of all input data for this mesh part
//             //
//             let positions_bytes =
//                 b3f_reader.get_block(mesh_part.position.ok_or("No position data")? as usize);
//             let normals_bytes =
//                 b3f_reader.get_block(mesh_part.normal.ok_or("No normal data")? as usize);
//             let tex_coords_bytes = b3f_reader
//                 .get_block(*mesh_part.uv.get(0).ok_or("No texture coordinate data")? as usize);
//             let part_indices_bytes = b3f_reader.get_block(mesh_part.indices as usize);
//
//             //
//             // Get strongly typed slices of all input data for this mesh part
//             //
//             let positions = try_cast_u8_slice::<[f32; 3]>(positions_bytes)
//                 .ok_or("Could not cast due to alignment")?;
//             let normals = try_cast_u8_slice::<[f32; 3]>(normals_bytes)
//                 .ok_or("Could not cast due to alignment")?;
//             let tex_coords = try_cast_u8_slice::<[f32; 2]>(tex_coords_bytes)
//                 .ok_or("Could not cast due to alignment")?;
//
//             // Indices may be encoded as u16 or u32, either way copy them out to a Vec<u32>
//             let mut part_indices = Vec::<u32>::default();
//             match mesh_part.index_type {
//                 MeshPartJsonIndexType::U16 => {
//                     let part_indices_u16 = try_cast_u8_slice::<u16>(part_indices_bytes)
//                         .ok_or("Could not cast due to alignment")?;
//                     part_indices.reserve(part_indices_u16.len());
//                     for &part_index in part_indices_u16 {
//                         part_indices.push(part_index as u32);
//                     }
//                 }
//                 MeshPartJsonIndexType::U32 => {
//                     let part_indices_u32 = try_cast_u8_slice::<u32>(part_indices_bytes)
//                         .ok_or("Could not cast due to alignment")?;
//                     part_indices.reserve(part_indices_u32.len());
//                     for &part_index in part_indices_u32 {
//                         part_indices.push(part_index);
//                     }
//                 }
//             };
//
//             let part_data = super::util::process_mesh_part(
//                 &part_indices,
//                 &positions,
//                 &normals,
//                 &tex_coords,
//                 &mut all_vertices_full,
//                 &mut all_vertices_position,
//                 &mut all_indices,
//             );
//
//             //
//             // Positions and indices for the visibility system
//             //
//             for index in part_indices {
//                 all_position_indices.push(index as u32);
//             }
//
//             for i in 0..positions.len() {
//                 all_positions.push(Vec3::new(positions[i][0], positions[i][1], positions[i][2]));
//             }
//
//             let mesh_material = mesh_part.material.clone();
//
//             mesh_parts.push(MeshBasicPartAssetData {
//                 mesh_material,
//                 vertex_full_buffer_offset_in_bytes: part_data.vertex_full_buffer_offset_in_bytes,
//                 vertex_full_buffer_size_in_bytes: part_data.vertex_full_buffer_size_in_bytes,
//                 vertex_position_buffer_offset_in_bytes: part_data
//                     .vertex_position_buffer_offset_in_bytes,
//                 vertex_position_buffer_size_in_bytes: part_data
//                     .vertex_position_buffer_size_in_bytes,
//                 index_buffer_offset_in_bytes: part_data.index_buffer_offset_in_bytes,
//                 index_buffer_size_in_bytes: part_data.index_buffer_size_in_bytes,
//                 index_type: part_data.index_type,
//             })
//         }
//
//         let mut imported_assets = Vec::with_capacity(3);
//
//         //
//         // Vertex Full Buffer
//         //
//         assert!(!all_vertices_full.is_empty());
//         let vertex_full_buffer_asset = BufferAssetData {
//             resource_type: RafxResourceType::VERTEX_BUFFER,
//             data: all_vertices_full.into_data(),
//         };
//
//         let vertex_full_buffer_handle = make_handle(vertex_full_buffer_id);
//
//         //
//         // Vertex Position Buffer
//         //
//         assert!(!all_vertices_position.is_empty());
//         let vertex_position_buffer_asset = BufferAssetData {
//             resource_type: RafxResourceType::VERTEX_BUFFER,
//             data: all_vertices_position.into_data(),
//         };
//
//         let vertex_position_buffer_handle = make_handle(vertex_position_buffer_id);
//
//         //
//         // Index Buffer
//         //
//         assert!(!all_indices.is_empty());
//         let index_buffer_asset = BufferAssetData {
//             resource_type: RafxResourceType::INDEX_BUFFER,
//             data: all_indices.into_data(),
//         };
//
//         let index_buffer_handle = make_handle(index_buffer_id);
//
//         let mesh_data = PolygonSoup {
//             vertex_positions: all_positions,
//             index: PolygonSoupIndex::Indexed32(all_position_indices),
//         };
//
//         let asset_data = MeshBasicAssetData {
//             mesh_parts,
//             vertex_full_buffer: vertex_full_buffer_handle,
//             vertex_position_buffer: vertex_position_buffer_handle,
//             index_buffer: index_buffer_handle,
//             visible_bounds: VisibleBounds::from(mesh_data),
//         };
//
//         imported_assets.push(ImportedAsset {
//             id: mesh_id,
//             search_tags: vec![],
//             build_deps: vec![],
//             load_deps: vec![],
//             build_pipeline: None,
//             asset_data: Box::new(asset_data),
//         });
//
//         imported_assets.push(ImportedAsset {
//             id: vertex_full_buffer_id,
//             search_tags: vec![],
//             build_deps: vec![],
//             load_deps: vec![],
//             build_pipeline: None,
//             asset_data: Box::new(vertex_full_buffer_asset),
//         });
//
//         imported_assets.push(ImportedAsset {
//             id: vertex_position_buffer_id,
//             search_tags: vec![],
//             build_deps: vec![],
//             load_deps: vec![],
//             build_pipeline: None,
//             asset_data: Box::new(vertex_position_buffer_asset),
//         });
//
//         imported_assets.push(ImportedAsset {
//             id: index_buffer_id,
//             search_tags: vec![],
//             build_deps: vec![],
//             load_deps: vec![],
//             build_pipeline: None,
//             asset_data: Box::new(index_buffer_asset),
//         });
//
//         Ok(ImporterValue {
//             assets: imported_assets,
//         })
//     }
// }
