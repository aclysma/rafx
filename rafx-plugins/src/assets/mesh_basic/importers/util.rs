use crate::features::mesh_basic::{MeshVertexFull, MeshVertexPosition};
use glam::{Vec2, Vec3};
use rafx::api::RafxIndexType;
use rafx::assets::push_buffer::PushBuffer;

// Calculates a tangent and binormal that are orthogonal to the polygon normal and align with the
// uv coordinate x and y axes respectively. May return zero vector if uv coordinates are the same
pub(super) fn calculate_tangent_binormal(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    uv0: Vec2,
    uv1: Vec2,
    uv2: Vec2,
) -> (Vec3, Vec3) {
    let dp1 = p1 - p0;
    let dp2 = p2 - p0;

    let duv1 = uv1 - uv0;
    let duv2 = uv2 - uv0;

    let denominator = duv1.x * duv2.y - duv1.y * duv2.x;
    let (t, b) = if denominator.abs() > 0.0001 {
        let r = 1.0 / denominator;
        let t = (dp1 * duv2.y - dp2 * duv1.y) * r;
        let b = (dp2 * duv1.x - dp1 * duv2.x) * r;

        // We normalize, weighted by the size of the triangle. So we can just add all the
        // tangents/binormals and normalize it at the end
        (t, b)
    } else {
        // If not divisible, assume uv coordinates don't change and the tangent/binormal
        // won't matter
        (glam::Vec3::ZERO, glam::Vec3::ZERO)
    };
    (t, b)
}

// Given unnormalized and possibly invalid tangent/binormals, produce a tangent/binormal that is
// normalized and appropriate for right-hand coordinate system.
fn fix_tangent_binormal(
    n: Vec3,
    t: Vec3,
    b: Vec3,
) -> (Vec3, Vec3) {
    let mut t = if t.length_squared() > 0.0001 {
        t.normalize()
    } else {
        // If we don't have a valid vector to normalize, just pick anything
        n.any_orthogonal_vector()
    };

    let b = if b.length_squared() > 0.0001 {
        let b = b.normalize();

        // Ensure right-handed coordinate system.. flip the tangent if it's not
        // See handedness section in
        // https://www.opengl-tutorial.org/intermediate-tutorials/tutorial-13-normal-mapping/
        if n.cross(t).dot(b) < 0.0 {
            t = t * -1.0;
        }

        b
    } else {
        // If we don't have a valid vector to normalize, pick anything orthogonal
        // to normal and tangent
        n.cross(t).normalize()
    };

    (t, b)
}

pub(super) struct MeshPartData {
    pub vertex_full_buffer_offset_in_bytes: u32,
    pub vertex_full_buffer_size_in_bytes: u32,
    pub vertex_position_buffer_offset_in_bytes: u32,
    pub vertex_position_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub index_type: RafxIndexType,
}

// Appends index/vertex data to buffers and returns metadata about the processed mesh part
pub(super) fn process_mesh_part(
    part_indices: &[u32],
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    tex_coords: &[[f32; 2]],
    all_vertices_full: &mut PushBuffer,
    all_vertices_position: &mut PushBuffer,
    all_indices: &mut PushBuffer,
) -> MeshPartData {
    //
    // Use texcoords to build tangents/binormals
    //
    let mut tangents = Vec::<glam::Vec3>::new();
    tangents.resize(positions.len(), glam::Vec3::default());

    let mut binormals = Vec::<glam::Vec3>::new();
    binormals.resize(positions.len(), glam::Vec3::default());

    assert_eq!(part_indices.len() % 3, 0);
    for i in 0..(part_indices.len() / 3) {
        let i0 = part_indices[i * 3] as usize;
        let i1 = part_indices[i * 3 + 1] as usize;
        let i2 = part_indices[i * 3 + 2] as usize;

        let p0 = glam::Vec3::from(positions[i0]);
        let p1 = glam::Vec3::from(positions[i1]);
        let p2 = glam::Vec3::from(positions[i2]);

        let uv0 = glam::Vec2::from(tex_coords[i0]);
        let uv1 = glam::Vec2::from(tex_coords[i1]);
        let uv2 = glam::Vec2::from(tex_coords[i2]);

        let (t, b) = super::util::calculate_tangent_binormal(p0, p1, p2, uv0, uv1, uv2);

        tangents[i0] += t;
        tangents[i1] += t;
        tangents[i2] += t;
        binormals[i0] += b;
        binormals[i1] += b;
        binormals[i2] += b;
    }

    //
    // Generate vertex buffers
    //
    let mut part_vertices_full = Vec::with_capacity(positions.len());
    let mut part_vertices_position = Vec::with_capacity(positions.len());
    for i in 0..positions.len() {
        let (t, b) = fix_tangent_binormal(glam::Vec3::from(normals[i]), tangents[i], binormals[i]);

        part_vertices_full.push(MeshVertexFull {
            position: positions[i],
            normal: normals[i],
            tangent: t.into(),
            binormal: b.into(),
            tex_coord: tex_coords[i],
        });
        part_vertices_position.push(MeshVertexPosition {
            position: positions[i],
        });
    }

    //
    // Optimize vertex/index buffers
    //
    #[cfg(feature = "meshopt")]
    let (part_indices_data, part_vertices_full, part_vertices_position) = {
        //WARNING: meshopt functions mutate values, even if they only take non-mut borrows. This is
        // technically unsound, so we need to be careful here. (And in theory it could become UB if
        // the compiler assumes data won't change)
        meshopt::optimize_vertex_cache_in_place(&part_indices, part_vertices_full.len());

        // This step is intentionally disabled, depth prepass should avoid overdraws
        //let threshold = 1.05f32;
        //let vertex_data_slice = part_vertices_full.as_slice();
        //let vertex_adapter = meshopt::VertexDataAdapter::new(
        //    rafx::base::memory::any_slice_as_bytes(vertex_data_slice),
        //    std::mem::size_of::<MeshVertexFull>(),
        //    rafx::base::offset_of!(MeshVertexFull, position) as usize
        //).unwrap();
        //meshopt::optimize_overdraw_in_place(&part_indices, &vertex_adapter, threshold);

        let remap = meshopt::optimize_vertex_fetch_remap(&part_indices, part_vertices_full.len());
        let part_indices =
            meshopt::remap_index_buffer(Some(&part_indices), part_indices.len(), &remap);
        let part_vertices_full =
            meshopt::remap_vertex_buffer(&part_vertices_full, part_vertices_full.len(), &remap);
        let part_vertices_position = meshopt::remap_vertex_buffer(
            &part_vertices_position,
            part_vertices_position.len(),
            &remap,
        );

        (part_indices, part_vertices_full, part_vertices_position)
    };
    #[cfg(feature = "meshopt")]
    let part_indices = &part_indices_data;

    //
    // Push the optimized vertex info into the combined buffer for the mesh
    //
    let vertex_full_offset = all_vertices_full.len();
    all_vertices_full.push(&part_vertices_full, 1);
    let vertex_full_size = all_vertices_full.len() - vertex_full_offset;

    let vertex_position_offset = all_vertices_position.len();
    all_vertices_position.push(&part_vertices_position, 1);
    let vertex_position_size = all_vertices_position.len() - vertex_position_offset;

    //
    // Do we need to use u32 index buffers?
    //
    assert!(part_vertices_position.len() == part_vertices_full.len());
    let index_type = if part_vertices_position.len() >= (u16::MAX as usize) {
        RafxIndexType::Uint32
    } else {
        RafxIndexType::Uint16
    };

    //
    // Push the optimized index info into the combined buffer for the mesh
    //
    let indices_offset = all_indices.len();
    match index_type {
        RafxIndexType::Uint32 => {
            all_indices.push(&part_indices, 1);
        }
        RafxIndexType::Uint16 => {
            for &index in part_indices {
                all_indices.push(&[index as u16], 1);
            }
        }
    }
    let indices_size = all_indices.len() - indices_offset;

    MeshPartData {
        vertex_full_buffer_offset_in_bytes: vertex_full_offset as u32,
        vertex_full_buffer_size_in_bytes: vertex_full_size as u32,
        vertex_position_buffer_offset_in_bytes: vertex_position_offset as u32,
        vertex_position_buffer_size_in_bytes: vertex_position_size as u32,
        index_buffer_offset_in_bytes: indices_offset as u32,
        index_buffer_size_in_bytes: indices_size as u32,
        index_type,
    }
}
