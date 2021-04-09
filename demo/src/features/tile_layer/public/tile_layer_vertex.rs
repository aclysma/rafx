/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct TileLayerVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}
