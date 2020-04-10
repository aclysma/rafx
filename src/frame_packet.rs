use std::sync::Mutex;
use std::sync::Arc;
use crate::{RenderRegistry, RenderNodeSet, RenderView, GenericRenderNodeHandle};
use crate::visibility::{VisibilityResult};
use crate::render_view::RenderViewSet;

struct PerFrameNode {

}

struct PerViewNode {

}

////////////////// FramePacket //////////////////
#[derive(Default)]
struct FramePacketInner {
    view_packets: Vec<Arc<ViewPacket>>,
    frame_nodes: Vec<PerFrameNode>
}

#[derive(Default)]
pub struct FramePacket {
    //TODO: Use atomics instead of mutex
    inner: Mutex<FramePacketInner>
}

impl FramePacket {
    // pub fn new(view_count: usize) -> Self {
    //     //let view_packets = (0..view_count).map(|_| Arc::new(ViewPacket::default())).collect();
    //
    //     let inner = FramePacketInner {
    //         view_packets,
    //         frame_nodes: Default::default()
    //     };
    //
    //     FramePacket {
    //         inner: Mutex::new(inner)
    //     }
    // }
    //
    // pub fn allocate_view_packet(&self) -> usize {
    //     let mut guard = self.inner.lock().unwrap();
    //     let index = guard.view_packets.len();
    //     let view_packet = Arc::new(ViewPacket::default());
    //     guard.view_packets.push(view_packet);
    //     index
    // }
    //
    // pub fn view_packet(&self, index: usize) -> Arc<ViewPacket> {
    //     let guard = self.inner.lock().unwrap();
    //     guard.view_packets[index].clone()
    // }

    pub fn get_or_allocate_view_packet(&self, index: usize) -> Arc<ViewPacket> {
        let mut guard = self.inner.lock().unwrap();

        guard.view_packets.resize_with(index + 1, || Arc::new(ViewPacket::default()));
        guard.view_packets[index].clone()
    }

    pub fn append_frame_node(&self, handle: GenericRenderNodeHandle) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let index = guard.frame_nodes.len();
        guard.frame_nodes.push(PerFrameNode {

        });
        //println!("push frame node");
        index
    }

    // pub fn allocate_nodes(render_node_set: &RenderNodeSet) {
    //     let node_count_by_type = render_node_set.node_count_by_type();
    //     let frame_node_assignments = node_count_by_type.iter().map(|node_count| vec![-1; *node_count as usize]).collect();
    //
    //     for handle in &static_visibility.handles {
    //         let frame_node_index = frame_packet.append_frame_node(*handle);
    //         let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
    //     }
    //
    //     for handle in &dynamic_visibility.handles {
    //         let frame_node_index = frame_packet.append_frame_node(*handle);
    //         let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
    //     }
    // }
}

#[derive(Default)]
struct ViewPacketInner {
    view_nodes: Vec<PerViewNode>

}

#[derive(Default)]
pub struct ViewPacket {
    //TODO: Use atomics instead of mutex
    inner: Mutex<ViewPacketInner>
}

impl ViewPacket {
    pub fn append_view_node(&self, handle: GenericRenderNodeHandle, frame_node: usize) {
        let mut guard = self.inner.lock().unwrap();
        guard.view_nodes.push(PerViewNode {

        });
        //println!("push view node");
    }
}


pub struct FramePacketBuilder {
    frame_packet: FramePacket,

    // index by feature index, then render object index
    frame_node_assignments: Vec<Vec<i32>>
}

impl FramePacketBuilder {
    pub fn new(render_node_set: &RenderNodeSet/*, view_set: &RenderViewSet*/) -> Self {
        // let view_count = view_set.view_count();
        // let frame_packet = FramePacket::new(view_count);
        let frame_packet = FramePacket::default();

        //let view_count = view_set.view_count();
        //let view_packet_index : Vec<_> = (0..view_count).map(|_| frame_packet.allocate_view_packet()).collect();
        //let view_packets = frame_packet.view_packet(view_packet_index as usize);
        //let view_packets = (0..view_count).map(|_| frame_packet.allocate_view_packet()).map(|(_index, view_packet)| view_packet).collect();

        let feature_count = RenderRegistry::registered_feature_count();
        //let frame_node_assignments : Vec<_>= (0..feature_count).map(|_| ).collect();

        let node_count_by_type = render_node_set.node_count_by_type();
        let frame_node_assignments = node_count_by_type.iter().map(|node_count| vec![-1; *node_count as usize]).collect();

        FramePacketBuilder {
            frame_packet,
            frame_node_assignments
        }
    }

    // pub fn allocate_view_packet(&self) -> usize {
    //     let (index, view_packet) = self.frame_packet.allocate_view_packet();
    //     index
    // }

    pub fn allocate_frame_packet_nodes(
        &self,
        render_node_set: &RenderNodeSet,
        //frame_packet: &FramePacket,
        view: &RenderView,
        visibility_results: &[VisibilityResult],
    ) {
        log::info!("Allocate frame packet nodes for {}", view.debug_name());
        let view_packet = self.frame_packet.get_or_allocate_view_packet(view.view_index());

        //let handle_bins =

        // Compute views
        // Kick off extract job per view
        //  - Produce list of visible objects for the view
        //  - Create per-view nodes
        //  - Create per-frame nodes
        //  (sync point here to wait for all views to be done?)
        //  - frame extract entry point
        //  - extract per-frame nodes
        //  - extract per-view nodes

        // Are there per object nodes?


        for visibility_result in visibility_results {
            for handle in &visibility_result.handles {
                let frame_node_index = self.frame_packet.append_frame_node(*handle);
                let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
            }
        }
        // for handle in &static_visibility.handles {
        //     let frame_node_index = frame_packet.append_frame_node(*handle);
        //     let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
        // }
        //
        // for handle in &dynamic_visibility.handles {
        //     let frame_node_index = frame_packet.append_frame_node(*handle);
        //     let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
        // }
    }

    pub fn build(&self) -> FramePacket {
        FramePacket::default()
    }
}