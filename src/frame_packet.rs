use std::sync::Mutex;
use std::sync::Arc;
use crate::{RenderRegistry, RenderNodeSet, RenderView, GenericRenderNodeHandle};
use crate::visibility::{VisibilityResult};
use crate::render_view::RenderViewSet;

struct PerFrameNode {}

struct PerViewNode {}

////////////////// FramePacket //////////////////
#[derive(Default)]
struct FramePacketInner {
    view_packets: Vec<Arc<ViewPacket>>,
    frame_nodes: Vec<PerFrameNode>,
}

#[derive(Default)]
pub struct FramePacket {
    //TODO: Use atomics instead of mutex
    inner: Mutex<FramePacketInner>,
}

impl FramePacket {
    pub fn get_or_allocate_view_packet(
        &self,
        index: usize,
    ) -> Arc<ViewPacket> {
        let mut guard = self.inner.lock().unwrap();

        guard
            .view_packets
            .resize_with(index + 1, || Arc::new(ViewPacket::default()));
        guard.view_packets[index].clone()
    }

    pub fn append_frame_node(
        &self,
        handle: GenericRenderNodeHandle,
    ) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let index = guard.frame_nodes.len();
        guard.frame_nodes.push(PerFrameNode {});
        log::trace!("push frame node");
        index
    }
}

#[derive(Default)]
struct ViewPacketInner {
    view_nodes: Vec<PerViewNode>,
}

#[derive(Default)]
pub struct ViewPacket {
    //TODO: Use atomics instead of mutex
    inner: Mutex<ViewPacketInner>,
}

impl ViewPacket {
    pub fn append_view_node(
        &self,
        handle: GenericRenderNodeHandle,
        frame_node: usize,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.view_nodes.push(PerViewNode {});
        log::trace!("push view node");
    }
}

pub struct FramePacketBuilder {
    frame_packet: FramePacket,

    // index by feature index, then render object index
    frame_node_assignments: Vec<Vec<i32>>,
}

impl FramePacketBuilder {
    pub fn new(render_node_set: &RenderNodeSet) -> Self {
        let frame_packet = FramePacket::default();

        let feature_count = RenderRegistry::registered_feature_count();

        let node_count_by_type = render_node_set.node_count_by_type();
        let frame_node_assignments = node_count_by_type
            .iter()
            .map(|node_count| vec![-1; *node_count as usize])
            .collect();

        FramePacketBuilder {
            frame_packet,
            frame_node_assignments,
        }
    }

    pub fn allocate_frame_packet_nodes(
        &self,
        render_node_set: &RenderNodeSet,
        view: &RenderView,
        visibility_results: &[VisibilityResult],
    ) {
        log::info!("Allocate frame packet nodes for {}", view.debug_name());
        let view_packet = self
            .frame_packet
            .get_or_allocate_view_packet(view.view_index());

        for visibility_result in visibility_results {
            for handle in &visibility_result.handles {
                let frame_node_index = self.frame_packet.append_frame_node(*handle);
                let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
            }
        }
    }

    pub fn build(&self) -> FramePacket {
        FramePacket::default()
    }
}
