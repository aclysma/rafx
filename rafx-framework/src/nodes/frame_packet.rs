use super::registry::RenderFeatureCount;
use super::render_nodes::RenderNodeIndex;
use super::{GenericRenderNodeHandle, RenderFeatureIndex, RenderRegistry, RenderView};
use crate::visibility::{EntityId, VisibilityObjectId, VisibilityRegion, VisibilityConfig};
use fnv::FnvHashMap;
use slotmap::KeyData;
use std::sync::Mutex;

pub type FrameNodeIndex = u32;
pub type FrameNodeCount = u32;

pub type ViewNodeIndex = u32;
pub type ViewNodeCount = u32;

#[derive(Debug, Copy, Clone)]
pub struct PerFrameNode {
    entity_id: EntityId,
    render_node_index: u32,
}

impl PerFrameNode {
    pub fn render_node_index(self) -> RenderNodeIndex {
        self.render_node_index
    }

    pub fn entity_id(self) -> EntityId {
        self.entity_id
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
    // index by feature index, then entity ID
    frame_node_assignments: Vec<FnvHashMap<EntityId, FrameNodeIndex>>,

    // A builder per view
    view_packet_builders: Vec<Option<ViewPacketBuilder>>,

    // All frame nodes, grouped by feature index
    frame_nodes: Vec<Vec<PerFrameNode>>,
}

pub struct FramePacketBuilder {
    inner: Mutex<FramePacketBuilderInner>,
}

impl FramePacketBuilder {
    pub fn new() -> Self {
        let feature_count = RenderRegistry::registered_feature_count();

        let frame_node_assignments = (0..feature_count).map(|_| FnvHashMap::default()).collect();

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

    pub fn query_visibility_and_add_results(
        &self,
        view: &RenderView,
        visibility_region: &VisibilityRegion,
        visibility_config: &VisibilityConfig
    ) {
        let feature_count = RenderRegistry::registered_feature_count();

        log::trace!("Allocate frame packet nodes for {}", view.debug_name());
        let view_packet_builder = ViewPacketBuilder::new(feature_count);

        let mut view_frustum = view.view_frustum();
        let visibility_results = view_frustum.query_visibility(visibility_config).unwrap();

        let mut guard = self.inner.lock().unwrap();

        for visibility_result in visibility_results.objects.iter() {
            let visibility_object_id =
                VisibilityObjectId::from(KeyData::from_ffi(visibility_result.id));

            let visibility_object = visibility_region.object_ref(visibility_object_id);

            for render_node_handle in visibility_object.features() {
                if view.feature_index_is_relevant(render_node_handle.render_feature_index()) {
                    let frame_node_index = Self::append_frame_node(
                        &mut *guard,
                        visibility_object.entity_id(),
                        *render_node_handle,
                    );
                    view_packet_builder.append_view_node(*render_node_handle, frame_node_index);
                }
            }
        }

        guard
            .view_packet_builders
            .resize_with(view.view_index() as usize + 1, || None);
        guard.view_packet_builders[view.view_index() as usize] = Some(view_packet_builder);
    }

    fn append_frame_node(
        guard: &mut FramePacketBuilderInner,
        entity_id: EntityId,
        handle: GenericRenderNodeHandle,
    ) -> FrameNodeIndex {
        let frame_node_assignments =
            &mut guard.frame_node_assignments[handle.render_feature_index() as usize];

        // A crash here likely means render nodes for this feature weren't registered
        let index = frame_node_assignments.get(&entity_id);

        if let Some(index) = index {
            *index as FrameNodeIndex
        } else {
            log::trace!("push frame node");

            let index = guard.frame_nodes[handle.render_feature_index() as usize].len();
            guard.frame_nodes[handle.render_feature_index() as usize].push(PerFrameNode {
                render_node_index: handle.render_node_index(),
                entity_id,
            });
            frame_node_assignments.insert(entity_id, index as FrameNodeIndex);
            index as FrameNodeIndex
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
