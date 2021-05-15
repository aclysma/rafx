use super::RenderFeatureSubmitNode;
use crate::render_features::SubmitNodeSortFunction;
use fnv::FnvHashMap;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// The `ID` of a registered `RenderFeature`.
pub type RenderFeatureIndex = u32;

/// The `ID` of a registered `RenderPhase`.
pub type RenderPhaseIndex = u32;

pub type RenderFeatureMaskInnerType = u64;
pub const MAX_RENDER_FEATURE_COUNT: u32 = 64;

pub type RenderPhaseMaskInnerType = u32;
pub const MAX_RENDER_PHASE_COUNT: u32 = 32;

pub struct RenderFeatureDebugConstants {
    pub feature_name: &'static str,

    pub begin_per_frame_extract: &'static str,
    pub extract_render_object_instance: &'static str,
    pub extract_render_object_instance_per_view: &'static str,
    pub end_per_view_extract: &'static str,
    pub end_per_frame_extract: &'static str,

    pub begin_per_frame_prepare: &'static str,
    pub prepare_render_object_instance: &'static str,
    pub prepare_render_object_instance_per_view: &'static str,
    pub end_per_view_prepare: &'static str,
    pub end_per_frame_prepare: &'static str,

    pub on_begin_execute_graph: &'static str,
    pub render_submit_node: &'static str,
    pub apply_setup: &'static str,
    pub revert_setup: &'static str,
}

pub trait RenderFeature {
    fn set_feature_index(index: RenderFeatureIndex);
    fn feature_index() -> RenderFeatureIndex;

    fn feature_debug_name() -> &'static str;
    fn feature_debug_constants() -> &'static RenderFeatureDebugConstants;
}

pub trait RenderPhase {
    fn set_render_phase_index(index: RenderPhaseIndex);
    fn render_phase_index() -> RenderPhaseIndex;

    fn sort_submit_nodes(submit_nodes: &mut Vec<RenderFeatureSubmitNode>);

    fn render_phase_debug_name() -> &'static str;
}

pub struct RegisteredPhase {
    sort_submit_nodes_callback: SubmitNodeSortFunction,
}

impl RegisteredPhase {
    fn new<T: RenderPhase>() -> Self {
        RegisteredPhase {
            sort_submit_nodes_callback: T::sort_submit_nodes,
        }
    }
}

static RENDER_REGISTRY_FEATURE_COUNT: AtomicU32 = AtomicU32::new(0);
static RENDER_REGISTRY_PHASE_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
pub struct RenderRegistryBuilder {
    registered_phases: FnvHashMap<RenderPhaseIndex, RegisteredPhase>,
    phase_name_to_index: FnvHashMap<String, RenderPhaseIndex>,
}

impl RenderRegistryBuilder {
    pub fn register_feature<T>(self) -> Self
    where
        T: RenderFeature,
    {
        let feature_index = RENDER_REGISTRY_FEATURE_COUNT.fetch_add(1, Ordering::AcqRel);
        T::set_feature_index(feature_index);
        self
    }

    pub fn register_render_phase<T>(
        mut self,
        name: &str,
    ) -> Self
    where
        T: RenderPhase,
    {
        let render_phase_index = RENDER_REGISTRY_PHASE_COUNT.fetch_add(1, Ordering::AcqRel);
        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        T::set_render_phase_index(render_phase_index);
        let old = self
            .registered_phases
            .insert(T::render_phase_index(), RegisteredPhase::new::<T>());
        assert!(old.is_none());
        let old = self
            .phase_name_to_index
            .insert(name.to_string(), render_phase_index);
        assert!(old.is_none());
        self
    }

    pub fn build(self) -> RenderRegistry {
        let inner = RenderRegistryInner {
            registered_phases: self.registered_phases,
            phase_name_to_index: self.phase_name_to_index,
        };

        RenderRegistry {
            inner: Arc::new(inner),
        }
    }
}

struct RenderRegistryInner {
    registered_phases: FnvHashMap<RenderPhaseIndex, RegisteredPhase>,
    phase_name_to_index: FnvHashMap<String, RenderPhaseIndex>,
}

#[derive(Clone)]
pub struct RenderRegistry {
    inner: Arc<RenderRegistryInner>,
}

impl RenderRegistry {
    pub fn registered_feature_count() -> RenderFeatureIndex {
        RENDER_REGISTRY_FEATURE_COUNT.load(Ordering::Acquire)
    }

    pub fn registered_render_phase_count() -> RenderPhaseIndex {
        RENDER_REGISTRY_PHASE_COUNT.load(Ordering::Acquire)
    }

    pub fn render_phase_index_from_name(
        &self,
        name: &str,
    ) -> Option<RenderPhaseIndex> {
        self.inner.phase_name_to_index.get(name).copied()
    }

    pub fn submit_node_sort_function(
        &self,
        render_phase_index: RenderPhaseIndex,
    ) -> SubmitNodeSortFunction {
        self.inner.registered_phases[&render_phase_index].sort_submit_nodes_callback
    }
}
