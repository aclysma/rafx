use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use fnv::FnvHashMap;
use std::sync::RwLock;

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

    fn sort_callback();
}

pub struct RegisteredPhase {
    sort_callback: fn()
}

impl RegisteredPhase {
    fn new<T: RenderPhase>(mut self) -> Self {
        RegisteredPhase {
            sort_callback: T::sort_callback
        }
    }
}

static RENDER_REGISTRY_FEATURE_COUNT: AtomicU32 = AtomicU32::new(0);
static RENDER_REGISTRY_PHASE_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
pub struct RenderRegistryBuilder {
    sort_callback: FnvHashMap<RenderPhaseIndex, RegisteredPhase>
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
        self
    }

    pub fn build(self) -> RenderRegistry {
        RenderRegistry {
            sort_callback: self.sort_callback
        }
    }
}

pub struct RenderRegistry {
    sort_callback: FnvHashMap<RenderPhaseIndex, RegisteredPhase>
}

impl RenderRegistry {
    pub fn register_feature<T>(&mut self)
    where
        T: RenderFeature,
    {
        let feature_index = RENDER_REGISTRY_FEATURE_COUNT.fetch_add(1, Ordering::AcqRel);
        T::set_feature_index(feature_index);
    }

    pub fn registered_feature_count() -> RenderFeatureIndex {
        RENDER_REGISTRY_FEATURE_COUNT.load(Ordering::Acquire)
    }

    pub fn register_render_phase<T>(&mut self)
    where
        T: RenderPhase,
    {
        let render_phase_index = RENDER_REGISTRY_PHASE_COUNT.fetch_add(1, Ordering::AcqRel);
        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        T::set_render_phase_index(render_phase_index);
    }

    pub fn registered_render_phase_count() -> RenderPhaseIndex {
        RENDER_REGISTRY_PHASE_COUNT.load(Ordering::Acquire)
    }
}
