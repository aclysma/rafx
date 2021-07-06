use crate::assets::mesh::{MeshAssetData, MeshPartAssetData};
use crate::features::mesh::MeshVertex;
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use glam::Vec3;
use rafx::api::{RafxIndexType, RafxResourceType};
use rafx::assets::push_buffer::PushBuffer;
use rafx::assets::{BufferAssetData, MaterialInstanceAsset};
use rafx::distill::loader::handle::Handle;
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
    pub material: Handle<MaterialInstanceAsset>,
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

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "1411cdbc-d63f-45aa-b9cf-adf610e43989"]
pub struct BlenderMeshImporterState {
    mesh_id: Option<AssetUuid>,
    vertex_buffer_id: Option<AssetUuid>,
    index_buffer_id: Option<AssetUuid>,
}

#[derive(TypeUuid)]
#[uuid = "f3486ab5-a780-4251-ab55-1000c638bc08"]
pub struct BlenderMeshImporter;
impl Importer for BlenderMeshImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        5
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = BlenderMeshImporterState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let mesh_id = state
            .mesh_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        let vertex_buffer_id = state
            .vertex_buffer_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        let index_buffer_id = state
            .index_buffer_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = BlenderMeshImporterState {
            mesh_id: Some(mesh_id),
            vertex_buffer_id: Some(vertex_buffer_id),
            index_buffer_id: Some(index_buffer_id),
        };
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let b3f_reader = rafx::base::b3f::B3FReader::new(&bytes)
            .ok_or("Blender Mesh Import error, mesh file format not recognized")?;
        let mesh_as_json: MeshJson =
            serde_json::from_slice(b3f_reader.get_block(0)).map_err(|e| e.to_string())?;

        let mut all_positions = Vec::<glam::Vec3>::with_capacity(1024);
        let mut all_position_indices = Vec::<u16>::with_capacity(8192);

        let mut all_vertices = PushBuffer::new(16384);
        let mut all_indices = PushBuffer::new(16384);

        let mut mesh_parts: Vec<MeshPartAssetData> =
            Vec::with_capacity(mesh_as_json.mesh_parts.len());

        for mesh_part in &mesh_as_json.mesh_parts {
            let positions =
                b3f_reader.get_block(mesh_part.position.ok_or("No position data")? as usize);
            let normals = b3f_reader.get_block(mesh_part.normal.ok_or("No normal data")? as usize);
            let tangents =
                b3f_reader.get_block(mesh_part.tangent.ok_or("No tangent data")? as usize);
            let tex_coords =
                b3f_reader.get_block(*mesh_part.uv.get(0).ok_or("No position data")? as usize);
            let part_indices = b3f_reader.get_block(mesh_part.indices as usize);

            let positions = try_cast_u8_slice::<[f32; 3]>(positions)
                .ok_or("Could not cast due to alignment")?;
            let normals =
                try_cast_u8_slice::<[f32; 3]>(normals).ok_or("Could not cast due to alignment")?;
            let tangents =
                try_cast_u8_slice::<[f32; 4]>(tangents).ok_or("Could not cast due to alignment")?;
            let tex_coords = try_cast_u8_slice::<[f32; 2]>(tex_coords)
                .ok_or("Could not cast due to alignment")?;
            let part_indices =
                try_cast_u8_slice::<u16>(part_indices).ok_or("Could not cast due to alignment")?;

            let vertex_offset = all_vertices.len();
            let indices_offset = all_indices.len();

            for i in 0..positions.len() {
                all_positions.push(Vec3::new(positions[i][0], positions[i][1], positions[i][2]));
                all_vertices.push(
                    &[MeshVertex {
                        position: positions[i],
                        normal: normals[i],
                        tangent: tangents[i],
                        tex_coord: tex_coords[i],
                    }],
                    1,
                );
            }

            all_indices.push(&part_indices, 1);
            all_position_indices.extend_from_slice(&part_indices);

            let vertex_size = all_vertices.len() - vertex_offset;
            let indices_size = all_indices.len() - indices_offset;

            let material_instance = mesh_part.material.clone();

            let index_type = match mesh_part.index_type {
                MeshPartJsonIndexType::U16 => RafxIndexType::Uint16,
                MeshPartJsonIndexType::U32 => RafxIndexType::Uint32,
            };

            mesh_parts.push(MeshPartAssetData {
                material_instance,
                vertex_buffer_offset_in_bytes: vertex_offset as u32,
                vertex_buffer_size_in_bytes: vertex_size as u32,
                index_buffer_offset_in_bytes: indices_offset as u32,
                index_buffer_size_in_bytes: indices_size as u32,
                index_type,
            })
        }

        let mut imported_assets = Vec::with_capacity(3);

        //
        // Vertex Buffer
        //
        let vertex_buffer_asset = BufferAssetData {
            resource_type: RafxResourceType::VERTEX_BUFFER,
            data: all_vertices.into_data(),
        };

        let vertex_buffer_handle = make_handle(vertex_buffer_id);

        //
        // Index Buffer
        //
        let index_buffer_asset = BufferAssetData {
            resource_type: RafxResourceType::INDEX_BUFFER,
            data: all_indices.into_data(),
        };

        let index_buffer_handle = make_handle(index_buffer_id);

        let mesh_data = PolygonSoup {
            vertex_positions: all_positions,
            index: PolygonSoupIndex::Indexed16(all_position_indices),
        };

        let asset_data = MeshAssetData {
            mesh_parts,
            vertex_buffer: vertex_buffer_handle,
            index_buffer: index_buffer_handle,
            visible_bounds: VisibleBounds::from(mesh_data),
        };

        imported_assets.push(ImportedAsset {
            id: mesh_id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(asset_data),
        });

        imported_assets.push(ImportedAsset {
            id: vertex_buffer_id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(vertex_buffer_asset),
        });

        imported_assets.push(ImportedAsset {
            id: index_buffer_id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(index_buffer_asset),
        });

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}
