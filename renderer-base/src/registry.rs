use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use fnv::FnvHashMap;
use crate::SubmitNode;
use std::sync::Arc;

pub type RenderFeatureIndex = u32;
pub type RenderFeatureCount = u32;
pub type RenderPhaseIndex = u32;

pub type RenderPhaseMaskInnerType = u32;
pub const MAX_RENDER_PHASE_COUNT: u32 = 32;

pub trait RenderFeature {
    fn set_feature_index(index: RenderFeatureIndex);
    fn feature_index() -> RenderFeatureIndex;

    fn feature_debug_name() -> &'static str;
}

pub trait RenderPhase {
    fn set_render_phase_index(index: RenderPhaseIndex);
    fn render_phase_index() -> RenderPhaseIndex;

    fn sort_submit_nodes(submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode>;

    fn render_phase_debug_name() -> &'static str;
}

type SortCallback = fn(Vec<SubmitNode>) -> Vec<SubmitNode>;

pub struct RegisteredPhase {
    sort_submit_nodes_callback: SortCallback,
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

    pub fn register_render_phase<T>(mut self) -> Self
    where
        T: RenderPhase,
    {
        let render_phase_index = RENDER_REGISTRY_PHASE_COUNT.fetch_add(1, Ordering::AcqRel);
        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        T::set_render_phase_index(render_phase_index);
        self.registered_phases
            .insert(T::render_phase_index(), RegisteredPhase::new::<T>());
        self
    }

    pub fn build(self) -> RenderRegistry {
        RenderRegistry {
            registered_phases: Arc::new(self.registered_phases),
        }
    }
}

#[derive(Clone)]
pub struct RenderRegistry {
    registered_phases: Arc<FnvHashMap<RenderPhaseIndex, RegisteredPhase>>,
}

impl RenderRegistry {
    pub fn registered_feature_count() -> RenderFeatureIndex {
        RENDER_REGISTRY_FEATURE_COUNT.load(Ordering::Acquire)
    }

    pub fn registered_render_phase_count() -> RenderPhaseIndex {
        RENDER_REGISTRY_PHASE_COUNT.load(Ordering::Acquire)
    }

    pub fn sort_submit_nodes(
        &self,
        render_phase_index: RenderPhaseIndex,
        submit_nodes: Vec<SubmitNode>,
    ) -> Vec<SubmitNode> {
        (self.registered_phases[&render_phase_index].sort_submit_nodes_callback)(submit_nodes)
    }
}
