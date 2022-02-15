use rafx::framework::{BufferResource, ResourceArc};
use rafx::render_features::RenderView;

pub struct OcclusionJob {
    pub draw_data: ResourceArc<BufferResource>,
    pub transforms: ResourceArc<BufferResource>,
    pub bounding_spheres: ResourceArc<BufferResource>,
    pub indirect_commands: ResourceArc<BufferResource>,

    pub render_view: RenderView,
    pub draw_data_count: u32,
    pub indirect_first_command_index: u32,
}

#[derive(Default)]
pub struct MeshAdvGpuOcclusionCullRenderResource {
    pub data: Vec<OcclusionJob>,
}
