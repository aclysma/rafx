use super::registry::RenderFeatureCount;
use super::render_nodes::{RenderNodeIndex, RenderNodeReservations};
use super::VisibilityResult;
use super::{GenericRenderNodeHandle, RenderFeatureIndex, RenderRegistry, RenderView};
use std::sync::Mutex;

pub type FrameNodeIndex = u32;
pub type FrameNodeCount = u32;

pub type ViewNodeIndex = u32;
pub type ViewNodeCount = u32;

#[derive(Debug, Copy, Clone)]
pub struct PerFrameNode {
    render_node_index: u32,
}

impl PerFrameNode {
    pub fn render_node_index(self) -> RenderNodeIndex {
        self.render_node_index
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PerViewNode {
    render_node_index: u32,
    frame_node_index: u32,
}

impl PerViewNode {
    pub fn render_node_index(self) -> RenderNodeIndex {
        self.render_node_index
    }

    pub fn frame_node_index(self) -> FrameNodeIndex {
        self.frame_node_index
    }
}

#[derive(Debug)]
pub struct FramePacket {
    view_packets: Vec<Option<ViewPacket>>,

    // index by feature index
    frame_nodes: Vec<Vec<PerFrameNode>>,
}

impl FramePacket {
    pub fn view_nodes(
        &self,
        view: &RenderView,
        feature_index: RenderFeatureIndex,
    ) -> Option<&[PerViewNode]> {
        if let Some(view_packet) = &self.view_packets[view.view_index() as usize] {
            Some(view_packet.view_nodes(feature_index))
        } else {
            None
        }
    }

    pub fn view_node_count(
        &self,
        view: &RenderView,
        feature_index: RenderFeatureIndex,
    ) -> ViewNodeCount {
        if let Some(view_packet) = &self.view_packets[view.view_index() as usize] {
            view_packet.view_nodes(feature_index).len() as ViewNodeCount
        } else {
            0
        }
    }

    pub fn frame_nodes(
        &self,
        feature_index: RenderFeatureIndex,
    ) -> &[PerFrameNode] {
        &self.frame_nodes[feature_index as usize]
    }

    pub fn frame_node_count(
        &self,
        feature_index: RenderFeatureIndex,
    ) -> FrameNodeCount {
        self.frame_nodes[feature_index as usize].len() as FrameNodeCount
    }
}

#[derive(Debug)]
pub struct ViewPacket {
    // index by feature index
    view_nodes: Vec<Vec<PerViewNode>>,
}

impl ViewPacket {
    pub fn view_nodes(
        &self,
        feature_index: RenderFeatureIndex,
    ) -> &[PerViewNode] {
        &self.view_nodes[feature_index as usize]
    }
}

struct ViewPacketBuilderInner {
    view_nodes: Vec<Vec<PerViewNode>>,
}

struct ViewPacketBuilder {
    inner: Mutex<ViewPacketBuilderInner>,
}

impl ViewPacketBuilder {
    pub fn new(feature_count: RenderFeatureCount) -> Self {
        let view_nodes = (0..feature_count).map(|_| Vec::new()).collect();

        let inner = Mutex::new(ViewPacketBuilderInner { view_nodes });

        ViewPacketBuilder { inner }
    }

    pub fn append_view_node(
        &self,
        handle: GenericRenderNodeHandle,
        frame_node_index: u32,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.view_nodes[handle.render_feature_index() as usize].push(PerViewNode {
            frame_node_index,
            render_node_index: handle.render_node_index(),
        });
        log::trace!("push view node");
    }

    pub fn build(self) -> ViewPacket {
        let mut guard = self.inner.lock().unwrap();
        let mut view_nodes = vec![];
        std::mem::swap(&mut view_nodes, &mut guard.view_nodes);

        ViewPacket { view_nodes }
    }
}

//TODO: Maybe the frame_node_assignments needs to be a heap of bitfields, sorted by render node,
// a bit per view to indicate it's present in the view
struct FramePacketBuilderInner {
    // O(1) lookup for if the render node is already inserted into the per frame node list
    // index by feature index, then render object index
    frame_node_assignments: Vec<Vec<i32>>,

    // A builder per view
    view_packet_builders: Vec<Option<ViewPacketBuilder>>,

    // All frame nodes, grouped by feature index
    frame_nodes: Vec<Vec<PerFrameNode>>,
}

pub struct FramePacketBuilder {
    inner: Mutex<FramePacketBuilderInner>,
}

impl FramePacketBuilder {
    pub fn new(render_node_set: &RenderNodeReservations) -> Self {
        let feature_count = RenderRegistry::registered_feature_count();
        let max_render_node_count_by_type = render_node_set.max_render_nodes_by_feature();

        debug_assert_eq!(feature_count as usize, max_render_node_count_by_type.len());

        for (feature_index, max_render_node_count) in
            max_render_node_count_by_type.iter().enumerate()
        {
            log::trace!(
                "node count for feature {}: {}",
                feature_index,
                max_render_node_count
            );
        }

        let frame_node_assignments = max_render_node_count_by_type
            .iter()
            .map(|max_render_node_count| vec![-1; *max_render_node_count as usize])
            .collect();

        let frame_nodes = (0..feature_count).map(|_| Default::default()).collect();

        let inner = FramePacketBuilderInner {
            frame_node_assignments,
            view_packet_builders: Default::default(),
            frame_nodes,
        };

        FramePacketBuilder {
            inner: Mutex::new(inner),
        }
    }

    pub fn add_view(
        &self,
        view: &RenderView,
        visibility_results: &[VisibilityResult],
    ) {
        let feature_count = RenderRegistry::registered_feature_count();

        log::trace!("Allocate frame packet nodes for {}", view.debug_name());
        let view_packet_builder = ViewPacketBuilder::new(feature_count);

        for visibility_result in visibility_results {
            for handle in &visibility_result.handles {
                let frame_node_index = self.append_frame_node(*handle);
                view_packet_builder.append_view_node(*handle, frame_node_index);
            }
        }

        let mut guard = self.inner.lock().unwrap();
        guard
            .view_packet_builders
            .resize_with(view.view_index() as usize + 1, || None);
        guard.view_packet_builders[view.view_index() as usize] = Some(view_packet_builder);
    }

    fn append_frame_node(
        &self,
        handle: GenericRenderNodeHandle,
    ) -> FrameNodeIndex {
        let mut guard = self.inner.lock().unwrap();

        // A crash here likely means render nodes for this feature weren't registered
        let index = guard.frame_node_assignments[handle.render_feature_index() as usize]
            [handle.render_node_index() as usize];

        if index == -1 {
            let index = guard.frame_nodes[handle.render_feature_index() as usize].len();
            guard.frame_nodes[handle.render_feature_index() as usize].push(PerFrameNode {
                render_node_index: handle.render_node_index(),
            });
            log::trace!("push frame node");
            guard.frame_node_assignments[handle.render_feature_index() as usize]
                [handle.render_node_index() as usize] = index as i32;
            index as u32
        } else {
            index as u32
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
            view_packets,
        }
    }
}
