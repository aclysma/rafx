//! Part of `rafx-framework`. Handles extracting, preparing, and writing draw calls

mod render_objects;
pub use render_objects::RenderObjectCount;
pub use render_objects::RenderObjectHandle;
pub use render_objects::RenderObjectId;
pub use render_objects::RenderObjectSet;
pub use render_objects::RenderObjectsMap;

mod render_views;
pub use render_views::RenderView;
pub use render_views::RenderViewCount;
pub use render_views::RenderViewDepthRange;
pub use render_views::RenderViewIndex;
pub use render_views::RenderViewSet;

mod jobs;
pub use jobs::*;

mod registry;
pub use registry::RenderFeature;
pub use registry::RenderFeatureDebugConstants;
pub use registry::RenderFeatureFlag;
pub use registry::RenderFeatureFlagIndex;
pub use registry::RenderFeatureIndex;
pub use registry::RenderPhase;
pub use registry::RenderPhaseIndex;
pub use registry::RenderRegistry;
pub use registry::RenderRegistryBuilder;
pub use registry::MAX_RENDER_PHASE_COUNT;

mod macro_render_feature;
mod macro_render_feature_flag;
mod macro_render_phase;

mod render_feature_mask;
pub use render_feature_mask::RenderFeatureMask;
pub use render_feature_mask::RenderFeatureMaskBuilder;
mod render_feature_flag_mask;
pub use render_feature_flag_mask::RenderFeatureFlagMask;
pub use render_feature_flag_mask::RenderFeatureFlagMaskBuilder;
mod render_phase_mask;
pub use render_phase_mask::RenderPhaseMask;
pub use render_phase_mask::RenderPhaseMaskBuilder;

pub mod render_features_prelude {
    pub use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
    pub use rafx_base::atomic_once_cell::AtomicOnceCell;
    pub use rafx_base::atomic_once_cell_array::AtomicOnceCellArray;
    pub use rafx_base::atomic_once_cell_stack::AtomicOnceCellStack;
    pub use rafx_base::trust_cell::TrustCell;

    pub use crate::render_features::{
        ExtractResources, FramePacketData, FramePacketSize, PreparedRenderData, RenderFeature,
        RenderFeatureDebugConstants, RenderFeatureExtractJob, RenderFeatureFramePacket,
        RenderFeatureIndex, RenderFeaturePrepareJob, RenderFeatureSubmitNode,
        RenderFeatureSubmitNodeBlock, RenderFeatureSubmitPacket, RenderFeatureViewPacket,
        RenderFeatureViewSubmitPacket, RenderFeatureWriteJob, RenderObjectHandle, RenderObjectId,
        RenderObjectInstanceId, RenderObjectInstanceObjectIds, RenderObjectInstancePerViewId,
        RenderObjectsMap, RenderPhase, RenderPhaseIndex, RenderRegistry, RenderRegistryBuilder,
        RenderView, RenderViewIndex, RenderViewSet, RenderViewSubmitNodeCount,
        RenderViewVisibilityQuery, SubmitNode, SubmitNodeBlocks, SubmitNodeId, SubmitNodeSortKey,
        SubmitPacketData, ViewFrameIndex, ViewPacketSize, ViewPhase, ViewPhaseSubmitNodeBlock,
        ViewVisibilityJob, VisibleRenderObjects,
    };

    pub use crate::visibility::{ObjectId, VisibilityRegion};

    pub use crate::render_features::{
        RenderFeatureFramePacketAsConcrete, RenderFeatureFramePacketIntoConcrete,
        RenderFeatureSubmitPacketAsConcrete, RenderFeatureSubmitPacketIntoConcrete,
        RenderFeatureViewPacketAsConcrete, RenderFeatureViewPacketIntoConcrete,
        RenderFeatureViewSubmitPacketAsConcrete, RenderFeatureViewSubmitPacketIntoConcrete,
    };

    pub use crate::render_features::{
        ExtractJob, ExtractJobEntryPoints, FramePacket, PrepareJob, PrepareJobEntryPoints,
        RenderObjectInstance, RenderObjectInstancePerView, SubmitNodeBlock, SubmitPacket,
        ViewPacket, ViewSubmitPacket,
    };

    pub use crate::render_features::{
        DefaultJobContext, ExtractPerFrameContext, ExtractPerViewContext,
        ExtractRenderObjectInstanceContext, ExtractRenderObjectInstancePerViewContext,
        PreparePerFrameContext, PreparePerViewContext, PrepareRenderObjectInstanceContext,
        PrepareRenderObjectInstancePerViewContext, RenderJobBeginExecuteGraphContext,
        RenderJobCommandBufferContext, RenderJobExtractAllocationContext, RenderJobExtractContext,
        RenderJobPrepareContext, RenderJobWriteContext, RenderObjectsJobContext,
    };
}
