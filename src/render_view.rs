
use glam::Mat4;
use crate::frame_packet::FramePacket;
use crate::static_visibility_node_set::StaticVisibilityResult;
use crate::dynamic_visibility_node_set::DynamicVisibilityResult;
use crate::RenderNodeSet;

////////////////// Views //////////////////
pub struct RenderView {
    view_proj: Mat4,
    view_index: usize
}

impl RenderView {
    pub fn new(frame_packet: &mut FramePacket, view_proj: Mat4) -> RenderView {
        let view_index = frame_packet.allocate_view_packet();
        Self {
            view_proj,
            view_index
        }
    }

    pub fn allocate_frame_packet_nodes(
        &self,
        render_node_set: &RenderNodeSet,
        frame_packet: &FramePacket,
        static_visibility: &StaticVisibilityResult,
        dynamic_visibility: &DynamicVisibilityResult)
    {
        let view_packet = frame_packet.view_packet(self.view_index);

        for handle in &static_visibility.handles {
            let frame_node_index = frame_packet.append_frame_node(*handle);
            let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
        }

        for handle in &dynamic_visibility.handles {
            let frame_node_index = frame_packet.append_frame_node(*handle);
            let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
        }
    }

    pub fn extract(
        &self,
        frame_packet: &mut FramePacket,
        //world: &World
    ) {
        // Extract all the data into the frame packet
    }
}
