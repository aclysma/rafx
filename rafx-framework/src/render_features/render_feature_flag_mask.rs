use crate::render_features::registry::{
    RenderFeatureFlagMaskInnerType, MAX_RENDER_FEATURE_FLAG_COUNT,
};
use crate::render_features::{RenderFeatureFlag, RenderFeatureFlagIndex};

#[derive(Default)]
pub struct RenderFeatureFlagMaskBuilder(RenderFeatureFlagMaskInnerType);

impl RenderFeatureFlagMaskBuilder {
    pub fn add_render_feature_flag<RenderFeatureFlagT: RenderFeatureFlag>(
        mut self
    ) -> RenderFeatureFlagMaskBuilder {
        let index = RenderFeatureFlagT::feature_flag_index();
        assert!(
            index < MAX_RENDER_FEATURE_FLAG_COUNT,
            "feature flag {} is not registered",
            RenderFeatureFlagT::feature_flag_debug_name()
        );
        self.0 |= 1 << RenderFeatureFlagT::feature_flag_index();
        self
    }

    pub fn build(self) -> RenderFeatureFlagMask {
        RenderFeatureFlagMask(self.0)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RenderFeatureFlagMask(RenderFeatureFlagMaskInnerType);

impl RenderFeatureFlagMask {
    pub fn is_included<RenderFeatureFlagT: RenderFeatureFlag>(&self) -> bool {
        let index = RenderFeatureFlagT::feature_flag_index();
        assert!(
            index < MAX_RENDER_FEATURE_FLAG_COUNT,
            "feature flag {} is not registered",
            RenderFeatureFlagT::feature_flag_debug_name()
        );

        self.is_included_index_unchecked(index)
    }

    #[inline(always)]
    pub fn is_included_index(
        &self,
        index: RenderFeatureFlagIndex,
    ) -> bool {
        assert!(
            index < MAX_RENDER_FEATURE_FLAG_COUNT,
            "feature flag index {} is invalid (did you forget to register a feature flag?)",
            index
        );

        self.is_included_index_unchecked(index)
    }

    pub fn empty() -> Self {
        RenderFeatureFlagMask(0)
    }

    #[inline(always)]
    fn is_included_index_unchecked(
        &self,
        index: RenderFeatureFlagIndex,
    ) -> bool {
        (self.0 & 1 << index) != 0
    }
}
