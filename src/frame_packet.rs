use std::sync::Mutex;
use std::sync::Arc;
use crate::GenericRenderNodeHandle;

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
    pub fn allocate_view_packet(&self) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let index = guard.view_packets.len();
        guard.view_packets.push(Arc::new(ViewPacket::default()));
        index
    }

    pub fn view_packet(&self, index: usize) -> Arc<ViewPacket> {
        let guard = self.inner.lock().unwrap();
        guard.view_packets[index].clone()
    }

    pub fn append_frame_node(&self, handle: GenericRenderNodeHandle) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let index = guard.frame_nodes.len();
        guard.frame_nodes.push(PerFrameNode {

        });
        println!("push frame node");
        index
    }
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
        println!("push view node");
    }
}