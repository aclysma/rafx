use crate::render_features::render_features_prelude::*;
use crate::visibility::{ViewFrustumId, VisibilityConfig};
use crate::RenderResources;
use fnv::{FnvHashMap, FnvHashSet};
use rafx_base::owned_pool::{OwnedPool, Pooled};

pub type ExtractResources<'extract> = rafx_base::resource_ref_map::ResourceRefMap<'extract>;
pub type RenderObjectInstanceObjectIds = FnvHashSet<RenderObjectInstance>;

#[derive(Clone)]
pub struct ViewPacketSize {
    pub view: RenderView,
    pub num_render_object_instances: usize,
    pub num_volumes: usize,
}

impl ViewPacketSize {
    pub fn size_of(view_packet: &dyn RenderFeatureViewPacket) -> Self {
        Self {
            view: view_packet.view().clone(),
            num_render_object_instances: view_packet.num_render_object_instances(),
            num_volumes: 0,
        }
    }
}

#[derive(Default, Clone)]
pub struct FramePacketSize {
    pub num_render_object_instances: usize,
    pub view_packet_sizes: Vec<ViewPacketSize>,
}

#[derive(Default, Clone)]
pub struct FramePacketMetadata {
    pub feature_index: RenderFeatureIndex,
    pub is_relevant: bool,
    pub frame_packet_size: FramePacketSize,
}

pub type VisibilityVecs = Vec<Vec<RenderObjectInstance>>;

pub struct RenderJobExtractAllocationContext {
    pub frame_packet_metadata: Vec<TrustCell<FramePacketMetadata>>,
    pub frame_packets: Vec<TrustCell<Option<Box<dyn RenderFeatureFramePacket>>>>,
    pub render_object_instances: Vec<TrustCell<RenderObjectInstanceObjectIds>>,
    visibility_vecs: Mutex<FnvHashMap<ViewFrustumId, OwnedPool<VisibilityVecs>>>,
    num_features: usize,
}

impl RenderJobExtractAllocationContext {
    pub fn new(num_features: usize) -> Self {
        let mut allocation_context = Self {
            frame_packet_metadata: Vec::with_capacity(num_features),
            frame_packets: Vec::with_capacity(num_features),
            render_object_instances: Vec::with_capacity(num_features),
            visibility_vecs: Default::default(),
            num_features,
        };

        allocation_context.clear();
        allocation_context
    }

    pub fn clear(&mut self) {
        self.frame_packet_metadata.clear();
        self.frame_packets.clear();
        self.render_object_instances.clear();

        for _ in 0..self.num_features {
            self.frame_packet_metadata
                .push(TrustCell::new(FramePacketMetadata::default()));
            self.frame_packets.push(TrustCell::new(None));
            self.render_object_instances
                .push(TrustCell::new(RenderObjectInstanceObjectIds::default()));
        }

        self.visibility_vecs.lock().clear();
    }

    pub fn query_visibility_vecs(
        &self,
        view: &RenderView,
    ) -> Pooled<VisibilityVecs> {
        let id = view.view_frustum().view_frustum_id();

        let mut visibility_vecs = self.visibility_vecs.lock();
        let pool = visibility_vecs.entry(id).or_insert_with(|| {
            OwnedPool::with_capacity(
                1,
                || vec![Vec::default(); RenderRegistry::registered_feature_count() as usize],
                |val| {
                    for feature in val.iter_mut() {
                        feature.clear();
                    }
                },
            )
        });

        pool.try_recv();
        pool.borrow()
    }
}

/// Holds references to resources valid for the entirety of the `extract` step as
/// represented by the `'extract` lifetime. `RenderFeatureExtractJob`s should cache
/// any resources needed from the `RenderJobExtractContext` during their `new` function.
pub struct RenderJobExtractContext<'extract> {
    pub allocation_context: &'extract RenderJobExtractAllocationContext,
    pub extract_resources: &'extract ExtractResources<'extract>,
    pub render_resources: &'extract RenderResources,
    pub visibility_config: &'extract VisibilityConfig,
}

impl<'extract> RenderJobExtractContext<'extract> {
    pub fn new(
        allocation_context: &'extract RenderJobExtractAllocationContext,
        extract_resources: &'extract ExtractResources<'extract>,
        render_resources: &'extract RenderResources,
        visibility_config: &'extract VisibilityConfig,
    ) -> Self {
        RenderJobExtractContext {
            allocation_context,
            extract_resources,
            render_resources,
            visibility_config,
        }
    }
}
