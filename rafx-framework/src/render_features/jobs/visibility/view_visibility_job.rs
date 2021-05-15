use crate::render_features::render_features_prelude::*;
use crate::render_features::VisibilityVecs;
use crate::visibility::VisibilityObjectId;
use rafx_base::owned_pool::Pooled;
use rafx_visibility::{ObjectHandle, VisibilityResult};
use slotmap::KeyData;

pub type VisibleRenderObjects = Pooled<VisibilityVecs>;

/// The `RenderObject`s visible to a specific `RenderView` for the current frame. Each `RenderObject`
/// is represented by a `RenderViewObject` with the `ObjectId` returned by the `VisibilityObject`
/// and a `RenderObjectId`. If a `VisibilityObject` has multiple `RenderObject`s associated with it,
/// the results will be returned as 0 or more `RenderViewObject`s. The visible `RenderObject`s will
/// only contain `RenderObject`s associated with a `RenderFeature` included by the `RenderView`'s
/// `RenderFeatureMask`.
pub struct RenderViewVisibilityQuery {
    pub view: RenderView,
    pub per_view_render_objects: VisibleRenderObjects,
}

impl RenderViewVisibilityQuery {
    pub fn render_object_instances_per_view(
        &self,
        feature_index: RenderFeatureIndex,
    ) -> Option<&Vec<RenderObjectInstance>> {
        self.per_view_render_objects
            .get(feature_index as usize)
            .and_then(|feature| {
                if feature.is_empty() {
                    None
                } else {
                    Some(feature)
                }
            })
    }
}

/// Determines the visible `RenderObject`s in a given `RenderView` and returns the results in the
/// `RenderViewVisibilityQuery`. It is thread-safe to call `query_visibility` on multiple `RenderView`s
/// simultaneously.
pub struct ViewVisibilityJob<'a> {
    pub view: RenderView,
    pub visibility_region: &'a VisibilityRegion,
}

impl<'a> ViewVisibilityJob<'a> {
    pub fn new(
        view: RenderView,
        visibility_region: &'a VisibilityRegion,
    ) -> Self {
        Self {
            view,
            visibility_region,
        }
    }

    pub fn view(&self) -> &RenderView {
        &self.view
    }

    #[profiling::function]
    pub fn query_visibility<'extract>(
        &self,
        extract_context: &RenderJobExtractContext<'extract>,
    ) -> RenderViewVisibilityQuery {
        let mut view_frustum = self.view.view_frustum();
        let visibility_query = view_frustum
            .query_visibility(extract_context.visibility_config)
            .unwrap();

        let visibility_object_lookup = self.visibility_region.object_lookup();
        let render_feature_mask = self.view.render_feature_mask();

        let mut render_objects = extract_context
            .allocation_context
            .query_visibility_vecs(&self.view);

        let visible_objects = &visibility_query.objects;
        for visibility_object in visible_objects.iter().map(|visibility_result| {
            visibility_object_lookup.object_ref(self.visibility_object_id(visibility_result))
        }) {
            let object_id = visibility_object.object_id();
            for render_object in visibility_object.render_objects() {
                let render_feature_index = render_object.render_feature_index();
                if !render_feature_mask.is_included_index(render_feature_index) {
                    continue;
                }

                render_objects[render_feature_index as usize]
                    .push(RenderObjectInstance::new(object_id, *render_object));
            }
        }

        // Sort the results.
        for feature in render_objects.iter_mut() {
            if feature.is_empty() {
                continue;
            }

            profiling::scope!("sort visible render objects");
            feature.sort_unstable_by_key(|render_object| render_object.render_object_id);
        }

        let per_view_render_objects = render_objects;
        RenderViewVisibilityQuery {
            view: self.view().clone(),
            per_view_render_objects,
        }
    }

    fn visibility_object_id(
        &self,
        visibility_result: &VisibilityResult<ObjectHandle>,
    ) -> VisibilityObjectId {
        VisibilityObjectId::from(KeyData::from_ffi(visibility_result.id))
    }
}
