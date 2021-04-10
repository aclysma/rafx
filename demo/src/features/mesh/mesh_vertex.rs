use serde::{Deserialize, Serialize};

/// Vertex format for vertices sent to the GPU
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    // w component is a sign value (-1 or +1) indicating handedness of the tangent basis
    // see GLTF spec for more info
    pub tangent: [f32; 4],
    pub tex_coord: [f32; 2],
}
