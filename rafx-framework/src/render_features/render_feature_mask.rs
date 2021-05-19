use crate::render_features::registry::{RenderFeatureMaskInnerType, MAX_RENDER_FEATURE_COUNT};
use crate::render_features::{RenderFeature, RenderFeatureIndex};

#[derive(Default)]
pub struct RenderFeatureMaskBuilder(RenderFeatureMaskInnerType);

impl RenderFeatureMaskBuilder {
    pub fn add_render_feature<RenderFeatureT: RenderFeature>(mut self) -> RenderFeatureMaskBuilder {
        let index = RenderFeatureT::feature_index();
        assert!(
            index < MAX_RENDER_FEATURE_COUNT,
            "feature {} is not registered",
            RenderFeatureT::feature_debug_name()
        );
        self.0 |= 1 << RenderFeatureT::feature_index();
        self
    }

    pub fn build(self) -> RenderFeatureMask {
        RenderFeatureMask(self.0)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RenderFeatureMask(RenderFeatureMaskInnerType);

impl RenderFeatureMask {
    pub fn is_included<RenderFeatureT: RenderFeature>(&self) -> bool {
        let index = RenderFeatureT::feature_index();
        assert!(
            index < MAX_RENDER_FEATURE_COUNT,
            "feature {} is not registered",
            RenderFeatureT::feature_debug_name()
        );

        self.is_included_index_unchecked(index)
    }

    #[inline(always)]
    pub fn is_included_index(
        &self,
        index: RenderFeatureIndex,
    ) -> bool {
        assert!(
            index < MAX_RENDER_FEATURE_COUNT,
            "feature index {} is invalid (did you forget to register a feature?)",
            index
        );

        self.is_included_index_unchecked(index)
    }

    pub fn empty() -> Self {
        RenderFeatureMask(0)
    }

    #[inline(always)]
    fn is_included_index_unchecked(
        &self,
        index: RenderFeatureIndex,
    ) -> bool {
        (self.0 & 1 << index) != 0
    }
}
