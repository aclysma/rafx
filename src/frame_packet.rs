use std::sync::Mutex;
use std::sync::Arc;
use crate::{RenderRegistry, RenderNodeSet, RenderView, GenericRenderNodeHandle};
use crate::visibility::{VisibilityResult};
use crate::render_view::RenderViewSet;
use crate::render_node_set::AllRenderNodes;

struct PerFrameNode {}

struct PerViewNode {}

////////////////// FramePacket //////////////////
pub struct FramePacket {
    view_packets: Vec<Option<ViewPacket>>,
    frame_nodes: Vec<PerFrameNode>,
}

impl FramePacket {

}

pub struct ViewPacket {
    view_nodes: Vec<PerViewNode>,
}

impl ViewPacket {

}



#[derive(Default)]
struct ViewPacketBuilderInner {
    view_nodes: Vec<PerViewNode>,
}

#[derive(Default)]
struct ViewPacketBuilder {
    inner: Mutex<ViewPacketBuilderInner>
}

impl ViewPacketBuilder {
    pub fn append_view_node(
        &self,
        handle: GenericRenderNodeHandle,
        frame_node: usize,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.view_nodes.push(PerViewNode {});
        log::trace!("push view node");
    }

    pub fn build(mut self) -> ViewPacket {
        let mut guard = self.inner.lock().unwrap();
        let mut view_nodes = vec![];
        std::mem::swap(&mut view_nodes, &mut guard.view_nodes);

        ViewPacket {
            view_nodes
        }
    }
}

struct FramePacketBuilderInner {
    // index by feature index, then render object index
    frame_node_assignments: Vec<Vec<i32>>,
    view_packet_builders: Vec<Option<ViewPacketBuilder>>,
    frame_nodes: Vec<PerFrameNode>,
}

pub struct FramePacketBuilder {
    inner: Mutex<FramePacketBuilderInner>
}

impl FramePacketBuilder {
    pub fn new(render_node_set: &AllRenderNodes) -> Self {
        let feature_count = RenderRegistry::registered_feature_count();

        let max_node_count_by_type = render_node_set.max_node_count_by_type();
        for (feature_index, max_node_count) in max_node_count_by_type.iter().enumerate() {
            log::debug!("node count for feature {}: {}", feature_index, max_node_count);
        }
        let frame_node_assignments = max_node_count_by_type
            .iter()
            .map(|max_node_count| vec![-1; *max_node_count as usize])
            .collect();

        let inner = FramePacketBuilderInner {
            frame_node_assignments,
            view_packet_builders: Default::default(),
            frame_nodes: Default::default()
        };

        FramePacketBuilder {
            inner: Mutex::new(inner)
        }
    }

    pub fn add_view(
        &self,
        view: &RenderView,
        visibility_results: &[VisibilityResult],
    ) {
        log::info!("Allocate frame packet nodes for {}", view.debug_name());
        let view_packet = ViewPacketBuilder::default();

        for visibility_result in visibility_results {
            for handle in &visibility_result.handles {
                let frame_node_index = self.append_frame_node(*handle);
                let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
            }
        }

        let mut guard = self.inner.lock().unwrap();
        guard
            .view_packet_builders
            .resize_with(view.view_index() + 1, || None);
    }

    fn append_frame_node(
        &self,
        handle: GenericRenderNodeHandle,
    ) -> usize {
        let mut guard = self.inner.lock().unwrap();

        let index = guard.frame_node_assignments[handle.render_feature_index() as usize][handle.slab_index() as usize];

        if index == -1 {
            let index = guard.frame_nodes.len();
            guard.frame_nodes.push(PerFrameNode {});
            log::trace!("push frame node");
            guard.frame_node_assignments[handle.render_feature_index() as usize][handle.slab_index() as usize] = index as i32;
            index as usize
        } else {
            index as usize
        }
    }

    pub fn build(self) -> FramePacket {
        let mut guard = self.inner.lock().unwrap();

        let mut frame_nodes = vec![];
        std::mem::swap(&mut frame_nodes, &mut guard.frame_nodes);

        let mut view_packet_builders = vec![];
        std::mem::swap(&mut view_packet_builders, &mut guard.view_packet_builders);

        let mut view_packets = Vec::with_capacity(view_packet_builders.len());
        for view_packet_builder in view_packet_builders {
            view_packets.push(view_packet_builder.map(|v| v.build()));
        }

        FramePacket {
            frame_nodes,
            view_packets
        }
    }
}
