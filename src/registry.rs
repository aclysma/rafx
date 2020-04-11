use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use crate::FramePacket;

pub type RenderFeatureIndex = u32;
pub type RenderPhaseIndex = u32;

pub type RenderPhaseMaskInnerType = u32;
pub const MAX_RENDER_PHASE_COUNT: u32 = 32;

pub trait RenderFeature {
    fn set_feature_index(index: RenderFeatureIndex);
    fn feature_index() -> RenderFeatureIndex;

    fn feature_debug_name() -> &'static str;

    //fn create_render_feature_extract_impl() -> Box<RenderFeatureExtractImpl>;
}

pub trait RenderFeatureExtractImpl {
    fn feature_index(&self) -> RenderFeatureIndex;
    fn feature_debug_name(&self) -> &str;

    fn extract_begin(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_frame_node(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_view_nodes(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_view_finalize(
        &self,
        frame_packet: &FramePacket,
    );
    fn extract_frame_finalize(
        &self,
        frame_packet: &FramePacket,
    );
}

pub trait RenderPhase {
    fn set_render_phase_index(index: RenderPhaseIndex);
    fn render_phase_index() -> RenderPhaseIndex;
}

static RENDER_REGISTRY_FEATURE_COUNT: AtomicU32 = AtomicU32::new(0);
static RENDER_REGISTRY_PHASE_COUNT: AtomicU32 = AtomicU32::new(0);

pub struct RenderRegistry;

impl RenderRegistry {
    pub fn register_feature<T>()
    where
        T: RenderFeature,
    {
        let feature_index = RENDER_REGISTRY_FEATURE_COUNT.fetch_add(1, Ordering::AcqRel);
        T::set_feature_index(feature_index);
    }

    pub fn registered_feature_count() -> RenderFeatureIndex {
        RENDER_REGISTRY_FEATURE_COUNT.load(Ordering::Acquire)
    }

    pub fn register_render_phase<T>()
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
