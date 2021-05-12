use crate::render_features::render_features_prelude::*;
use fnv::{FnvHashMap, FnvHasher};
use std::hash::{BuildHasherDefault, Hash};

/// The `FramePacket` is the data that must be extracted from either the `AssetManager` or the game's
/// `World` during the `Extract` step. After the `Extract` step has finished, there is no more access
/// allowed to either the `AssetManager` or the game's `World`. The `FramePacket` **must** be considered
/// immutable after the `Extract` step has finished.
pub trait FramePacketData {
    /// All data that is either unassociated with or shared by
    /// any of the render objects across each `RenderView`.
    type PerFrameData: Sync + Send;

    /// All data associated with a particular `Entity` and `RenderObject`.
    type RenderObjectInstanceData: Sync + Send;

    /// All data that is associated with a particular `RenderView`.
    /// This and a `Vec`<`RenderObjectInstancePerViewData`> define
    /// each `ViewPacket` in the `FramePacket`.
    type PerViewData: Sync + Send;

    /// All data associated with a particular `Entity`, `RenderObject`, and `RenderView`.
    type RenderObjectInstancePerViewData: Sync + Send;

    // TODO(dvd): see issue #29661 <https://github.com/rust-lang/rust/issues/29661> for more information
    // type FramePacket = GenericFramePacket<Self>;
    // type ViewPacket = GenericViewPacket<Self>;
}

/// Read documentation on `FramePacketData`.
pub struct FramePacket<FramePacketDataT: FramePacketData> {
    feature_index: RenderFeatureIndex,
    render_object_instance_ids: FnvHashMap<RenderObjectInstance, RenderObjectInstanceId>,

    pub(crate) per_frame_data: AtomicOnceCell<FramePacketDataT::PerFrameData>,
    pub(crate) render_object_instances: Vec<RenderObjectInstance>,
    pub(crate) render_object_instances_data:
        AtomicOnceCellArray<FramePacketDataT::RenderObjectInstanceData>,

    views: Vec<ViewPacket<FramePacketDataT>>,
}

impl<FramePacketDataT: FramePacketData> FramePacket<FramePacketDataT> {
    pub fn new(
        feature_index: RenderFeatureIndex,
        frame_packet_size: &FramePacketSize,
    ) -> Self {
        // TODO(dvd): 3 + 2 * N allocations here -- the vec of views, the per frame entities vec, the entity indices hash map, and two vecs * N views for objects / volumes.
        let mut views = Vec::with_capacity(frame_packet_size.view_packet_sizes.len());

        for view_packet_size in frame_packet_size.view_packet_sizes.iter() {
            views.push(ViewPacket::new(view_packet_size))
        }

        Self {
            feature_index,
            per_frame_data: AtomicOnceCell::new(),
            render_object_instances: Vec::with_capacity(
                frame_packet_size.num_render_object_instances,
            ),
            render_object_instances_data: AtomicOnceCellArray::with_capacity(
                frame_packet_size.num_render_object_instances,
            ),
            render_object_instance_ids: FnvHashMap::with_capacity_and_hasher(
                frame_packet_size.num_render_object_instances,
                BuildHasherDefault::<FnvHasher>::default(),
            ),
            views,
        }
    }

    pub fn view_packets(&self) -> &Vec<ViewPacket<FramePacketDataT>> {
        &self.views
    }

    pub fn view_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &ViewPacket<FramePacketDataT> {
        self.views.get(view_index as usize).unwrap_or_else(|| {
            panic!(
                "ViewPacket with ViewFrameIndex {} was not found in {}.",
                view_index,
                std::any::type_name::<FramePacketDataT>()
            )
        })
    }

    pub fn view_packet_mut(
        &mut self,
        view_index: ViewFrameIndex,
    ) -> &mut ViewPacket<FramePacketDataT> {
        self.views.get_mut(view_index as usize).unwrap_or_else(|| {
            panic!(
                "ViewPacket with ViewFrameIndex {} was not found in {}.",
                view_index,
                std::any::type_name::<FramePacketDataT>()
            )
        })
    }

    pub fn render_object_instances(&self) -> &Vec<RenderObjectInstance> {
        &self.render_object_instances
    }

    pub fn render_object_instances_data(
        &self
    ) -> &AtomicOnceCellArray<FramePacketDataT::RenderObjectInstanceData> {
        &self.render_object_instances_data
    }

    pub fn per_frame_data(&self) -> &AtomicOnceCell<FramePacketDataT::PerFrameData> {
        &self.per_frame_data
    }
}

impl<FramePacketDataT: 'static + Sync + Send + FramePacketData> RenderFeatureFramePacket
    for FramePacket<FramePacketDataT>
{
    fn get_or_push_render_object_instance(
        &mut self,
        render_object_instance: RenderObjectInstance,
    ) -> RenderObjectInstanceId {
        return if let Some(render_object_instance_id) =
            self.render_object_instance_ids.get(&render_object_instance)
        {
            *render_object_instance_id
        } else {
            let render_object_instance_id =
                self.render_object_instances.len() as RenderObjectInstanceId;

            self.render_object_instances.push(render_object_instance);
            self.render_object_instance_ids
                .insert(render_object_instance, render_object_instance_id);

            render_object_instance_id
        };
    }

    fn render_feature_view_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewPacket {
        self.view_packet(view_index)
    }

    fn render_feature_view_packet_mut(
        &mut self,
        view_index: u32,
    ) -> &mut dyn RenderFeatureViewPacket {
        self.view_packet_mut(view_index)
    }

    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex {
        self.views
            .iter()
            .position(|view_packet| view_packet.view().view_index() == view.view_index())
            .unwrap_or_else(|| {
                panic!(
                    "View {} with ViewIndex {} was not found in {}.",
                    view.debug_name(),
                    view.view_index(),
                    std::any::type_name::<FramePacketDataT>()
                )
            }) as ViewFrameIndex
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        self.feature_index
    }
}

/// A specific `Entity` and `RenderObject` represented by the `ObjectId` and `RenderObjectId`. A
/// `RenderObject` is associated with a particular `RenderFeature` by the `RenderFeatureIndex`.
/// The `FramePacket` only contains unique `RenderObjectInstance`s.
#[derive(Copy, Eq, PartialEq, Hash, Clone, Debug)]
pub struct RenderObjectInstance {
    pub object_id: ObjectId,
    pub render_object_id: RenderObjectId,
}

impl RenderObjectInstance {
    pub fn new(
        object_id: ObjectId,
        render_object_id: RenderObjectId,
    ) -> Self {
        Self {
            object_id,
            render_object_id,
        }
    }
}
