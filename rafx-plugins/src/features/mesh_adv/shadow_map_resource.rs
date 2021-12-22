use super::MeshAdvRenderFeature;
use super::MeshAdvRenderOptions;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use crate::features::mesh_adv::internal::{ShadowMapAtlas, ShadowMapAtlasElement};
use crate::features::mesh_adv::ShadowMapAtlasElementInfo;
use crate::phases::ShadowMapRenderPhase;
use fnv::{FnvHashMap, FnvHashSet};
use legion::*;
use rafx::framework::render_features::RenderFeatureFlagMask;
use rafx::rafx_visibility::{
    DepthRange, OrthographicParameters, PerspectiveParameters, Projection,
};
use rafx::render_features::{
    ExtractResources, RenderFeatureMask, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder,
    RenderView, RenderViewDepthRange, RenderViewIndex, RenderViewSet,
};
use rafx::visibility::{ObjectId, ViewFrustumArc};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeshAdvLightId {
    PointLight(ObjectId),
    SpotLight(ObjectId),
    DirectionalLight(ObjectId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeshAdvShadowViewId {
    SpotLight(ObjectId),
    DirectionalLight(ObjectId),
    PointLight(ObjectId, u8),
}

impl MeshAdvShadowViewId {
    // this function just exists to provide a Ord impl for sorting lights by score (we use the light
    // id as a tiebreaker)
    fn light_type_to_int_and_object_id(&self) -> (u8, ObjectId) {
        match self {
            MeshAdvShadowViewId::SpotLight(object_id) => (1, *object_id),
            MeshAdvShadowViewId::DirectionalLight(object_id) => (0, *object_id),
            MeshAdvShadowViewId::PointLight(object_id, cube_map_index) => {
                (2 + cube_map_index, *object_id)
            }
        }
    }
}

impl PartialOrd for MeshAdvShadowViewId {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MeshAdvShadowViewId {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        let (lhs_type, lhs_object_id) = self.light_type_to_int_and_object_id();
        let (rhs_type, rhs_object_id) = other.light_type_to_int_and_object_id();

        lhs_object_id
            .cmp(&rhs_object_id)
            .then_with(|| lhs_type.cmp(&rhs_type))
    }
}

#[derive(Clone)]
pub enum MeshAdvShadowMapRenderViewIndices {
    Single(ShadowViewIndex),
    Cube([Option<ShadowViewIndex>; 6]),
}

// These functions are primarily used to easily grab the render view index when you already know
// what variant of the enum is in use
impl MeshAdvShadowMapRenderViewIndices {
    pub fn unwrap_single(&self) -> ShadowViewIndex {
        match self {
            MeshAdvShadowMapRenderViewIndices::Single(value) => *value,
            MeshAdvShadowMapRenderViewIndices::Cube(_) => {
                panic!("Called unwrap_single() on MeshAdvShadowMapRenderViewIndices::Cube")
            }
        }
    }

    pub fn unwrap_cube_any(&self) -> ShadowViewIndex {
        match self {
            MeshAdvShadowMapRenderViewIndices::Single(_) => {
                panic!("Called unwrap_cube_any() on MeshAdvShadowMapRenderViewIndices::Single")
            }
            MeshAdvShadowMapRenderViewIndices::Cube(views) => {
                for view in views {
                    if view.is_some() {
                        return view.unwrap();
                    }
                }

                panic!("Called unwrap_cube_any() on MeshAdvShadowMapRenderViewIndices::Cube but all views are unassigned")
            }
        }
    }
}

#[derive(PartialEq)]
struct PotentialShadowView {
    score: f32,
    shadow_view_id: MeshAdvShadowViewId,
    // other info?
}

// Based on the way we're using floats here, I think this is ok, even if the rust compiler by default
// doesn't like Eq for f32
impl Eq for PotentialShadowView {}

impl PartialOrd for PotentialShadowView {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PotentialShadowView {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap()
            .then_with(|| self.shadow_view_id.cmp(&other.shadow_view_id))
    }
}

#[derive(Clone)]
pub struct MeshAdvShadowMapRenderViewMeta {
    pub view_dir: glam::Vec3,
    pub view_proj: glam::Mat4,
    pub depth_range: RenderViewDepthRange,
}

impl MeshAdvShadowMapRenderViewMeta {
    fn new(
        view: &glam::Mat4,
        proj: &glam::Mat4,
        depth_range: RenderViewDepthRange,
    ) -> MeshAdvShadowMapRenderViewMeta {
        MeshAdvShadowMapRenderViewMeta {
            view_dir: RenderView::view_mat4_to_view_dir(&view),
            view_proj: (*proj) * (*view),
            depth_range,
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct ShadowViewIndex(usize);

// The data structures in this struct are primarily indexed by "shadow view index" which is looked up
// via the maps.
#[derive(Default)]
pub struct MeshAdvShadowMapResource {
    //
    // These are reassigned in reassign_shadow_atlas_elements(). In this phase, we decide which
    // lights will have shadow maps.
    //
    // The shadow view ID is based primarily on type of light and ObjectID it's attached to. This
    // map allows looking up the shadow view index, which can be used to get more info about the
    // shadow view.
    pub(super) shadow_map_lookup_by_shadow_view_id:
        FnvHashMap<MeshAdvShadowViewId, ShadowViewIndex>,
    // The assignments for views to specific regions of the shadow atlas. These refs are RAII-style,
    // so dropping them will free the atlas space for future allocation.
    pub(super) shadow_map_atlas_element_assignments: Vec<Option<ShadowMapAtlasElement>>,

    pub(super) shadow_maps_needing_redraw: FnvHashSet<ShadowViewIndex>,

    //
    // These are set in calculate_shadow_map_views(). In this phase, we generate render views which
    // will drive rendering the shadow maps.
    //
    // All the views associated with shadow maps. This list is parallel with
    // shadow_map_atlas_element_assignments. shadow_map_render_views is a view object (that only
    // exists if we are actually rendering it this frame) and the shadow_map_render_views_meta
    // always exists and includes things like the projection matrix/depth range (must be known to
    // correctly sample from the image)
    pub(super) shadow_map_render_views_meta: Vec<MeshAdvShadowMapRenderViewMeta>,
    pub(super) shadow_map_render_views: Vec<Option<RenderView>>,
    // Looks up the shadow view index based on the RenderViewIndex (which is easily obtained if you
    // have a RenderView.)
    pub(super) shadow_map_lookup_by_render_view_index: FnvHashMap<RenderViewIndex, ShadowViewIndex>,
    // Looks up the shadow view index for a light (for point lights, there will be 6 indices, and some of them may be None.)
    pub(super) shadow_map_lookup_by_light_id:
        FnvHashMap<MeshAdvLightId, MeshAdvShadowMapRenderViewIndices>,
}

impl MeshAdvShadowMapResource {
    pub fn shadow_map_render_views(&self) -> &[Option<RenderView>] {
        &self.shadow_map_render_views
    }

    pub fn shadow_maps_needing_redraw(&self) -> &FnvHashSet<ShadowViewIndex> {
        &self.shadow_maps_needing_redraw
    }

    pub fn clear(&mut self) {
        self.shadow_maps_needing_redraw.clear();
        self.shadow_map_atlas_element_assignments.clear();
        self.shadow_map_lookup_by_shadow_view_id.clear();
        self.shadow_map_lookup_by_render_view_index.clear();
        self.shadow_map_render_views.clear();
        self.shadow_map_render_views_meta.clear();
        self.shadow_map_lookup_by_light_id.clear();
    }

    pub(super) fn shadow_map_atlas_element_assignment(
        &self,
        shadow_view_index: ShadowViewIndex,
    ) -> &ShadowMapAtlasElement {
        self.shadow_map_atlas_element_assignments[shadow_view_index.0]
            .as_ref()
            .unwrap()
    }

    pub(super) fn shadow_map_render_views_meta(
        &self,
        shadow_view_index: ShadowViewIndex,
    ) -> &MeshAdvShadowMapRenderViewMeta {
        &self.shadow_map_render_views_meta[shadow_view_index.0]
    }

    pub fn shadow_map_atlas_element_info_for_shadow_view_index(
        &self,
        shadow_view_index: ShadowViewIndex,
    ) -> ShadowMapAtlasElementInfo {
        self.shadow_map_atlas_element_assignments[shadow_view_index.0]
            .as_ref()
            .unwrap()
            .info()
    }

    // Looks up shadow atlas info associated with a particular render view.
    pub fn shadow_map_atlas_element_info_for_view(
        &self,
        view_index: RenderViewIndex,
    ) -> Option<ShadowMapAtlasElementInfo> {
        self.shadow_map_lookup_by_render_view_index
            .get(&view_index)
            .map(|&index| {
                self.shadow_map_atlas_element_assignments[index.0]
                    .as_ref()
                    .unwrap()
                    .info()
            })
    }

    // Adds the shadow map render views to the given list
    pub fn append_render_views(
        &self,
        render_views: &mut Vec<RenderView>,
    ) {
        for shadow_map_view in &self.shadow_map_render_views {
            if let Some(shadow_map_view) = shadow_map_view {
                render_views.push(shadow_map_view.clone());
            }
        }
    }

    // Called once a frame to reassign shadow atlas space
    pub fn recalculate_shadow_map_views(
        &mut self,
        render_view_set: &RenderViewSet,
        extract_resources: &ExtractResources,
        shadow_map_atlas: &mut ShadowMapAtlas,
        main_view_eye_position: glam::Vec3,
    ) {
        // After this function returns, shadow_map_lookup_by_shadow_view_id and shadow_map_atlas_element_assignments
        // will have any stale state removed. We need info from previous frame, so we do not clear these lists.
        Self::reassign_shadow_atlas_elements(
            &mut self.shadow_map_lookup_by_shadow_view_id,
            &mut self.shadow_map_atlas_element_assignments,
            &mut self.shadow_maps_needing_redraw,
            extract_resources,
            main_view_eye_position,
            shadow_map_atlas,
        );

        Self::calculate_shadow_map_views(
            render_view_set,
            extract_resources,
            &self.shadow_map_lookup_by_shadow_view_id,
            &self.shadow_map_atlas_element_assignments,
            &self.shadow_maps_needing_redraw,
            &mut self.shadow_map_render_views_meta,
            &mut self.shadow_map_render_views,
            &mut self.shadow_map_lookup_by_render_view_index,
            &mut self.shadow_map_lookup_by_light_id,
        );
    }

    // Find all views we would like to generate shadow maps for and sort by descending priority
    fn find_potential_shadow_views(
        extract_resources: &ExtractResources,
        main_view_eye_position: glam::Vec3,
    ) -> Vec<PotentialShadowView> {
        let world_fetch = extract_resources.fetch::<World>();
        let world = &*world_fetch;

        fn calculate_score(
            eye_position: glam::Vec3,
            light_position: glam::Vec3,
        ) -> f32 {
            eye_position.distance_squared(light_position)
        }

        let mut heap = std::collections::binary_heap::BinaryHeap::<PotentialShadowView>::new();

        let mut query = <(Entity, Read<SpotLightComponent>, Read<TransformComponent>)>::query();
        for (entity, _light, transform) in query.iter(world) {
            let shadow_view_id = MeshAdvShadowViewId::SpotLight(ObjectId::from(*entity));
            let score = calculate_score(main_view_eye_position, transform.translation);

            heap.push(PotentialShadowView {
                shadow_view_id,
                score,
            });
        }

        let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
        for (entity, _light) in query.iter(world) {
            // Hardcode a score of 0 for these because directional lights have no position, there
            // tend to be few of them per scene, and they tend to be important.
            let shadow_view_id = MeshAdvShadowViewId::DirectionalLight(ObjectId::from(*entity));
            let score = 0.0;

            heap.push(PotentialShadowView {
                shadow_view_id,
                score,
            });
        }

        let mut query = <(Entity, Read<PointLightComponent>, Read<TransformComponent>)>::query();
        for (entity, _light, transform) in query.iter(world) {
            for i in 0..6 {
                let shadow_view_id = MeshAdvShadowViewId::PointLight(ObjectId::from(*entity), i);
                let score = calculate_score(main_view_eye_position, transform.translation);

                heap.push(PotentialShadowView {
                    shadow_view_id,
                    score,
                });
            }
        }

        heap.into_sorted_vec()
    }

    // Determine shadow views we would like to draw in this frame. Assign space in the shadow map atlas
    // to them, first using free space if possible, then stealing space from lower-priority shadows.
    // This function will also release atlas space for lights that no longer exist.
    fn reassign_shadow_atlas_elements(
        shadow_map_lookup_by_shadow_view_id: &mut FnvHashMap<MeshAdvShadowViewId, ShadowViewIndex>,
        shadow_map_atlas_element_assignments: &mut Vec<Option<ShadowMapAtlasElement>>,
        shadow_maps_needing_redraw: &mut FnvHashSet<ShadowViewIndex>,
        extract_resources: &ExtractResources,
        main_view_eye_position: glam::Vec3,
        shadow_map_atlas: &mut ShadowMapAtlas,
    ) {
        shadow_maps_needing_redraw.clear();

        //
        // Find all potential shadow views, sorted by priority
        //
        let mut potential_views =
            Self::find_potential_shadow_views(extract_resources, main_view_eye_position);

        let mut new_assignments = Vec::with_capacity(potential_views.len());

        //
        // Place them into a lookup by shadow view ID and carry shadow assignments from previous frame
        // forward to this frame
        //
        for potential_view in &potential_views {
            // Carry over old assignments from previous frame to the new frame
            if let Some(&old_shadow_view_index) =
                shadow_map_lookup_by_shadow_view_id.get(&potential_view.shadow_view_id)
            {
                // unwrap should always succeed, we do not take the value from any one index more than once
                //TODO: Don't preserve shadow maps for shadow map views that have moved/changed (by a hash?)
                let assignment = shadow_map_atlas_element_assignments[old_shadow_view_index.0]
                    .take()
                    .unwrap();
                new_assignments.push(Some(assignment));
            } else {
                new_assignments.push(None);
            }
        }

        //
        // Release any shadow atlas elements that are no longer useful. This will allow us to reallocate them
        //
        shadow_map_atlas_element_assignments.clear();

        //
        // Create a lookup to find shadow maps already in use. The Vec is indexed by quality level (0=lowest)
        // and because they are in descending priority and push/pop is LIFO, we can look up the lowest
        // scoring assignment by quality
        //
        let quality_level_count = shadow_map_atlas.quality_level_count();
        let mut in_use_assignments_by_quality = vec![Vec::default(); quality_level_count];
        for (shadow_view_index, assignment) in new_assignments.iter_mut().enumerate() {
            if let Some(assignment) = assignment {
                in_use_assignments_by_quality[assignment.quality() as usize]
                    .push(shadow_view_index);
            }
        }

        for shadow_view_index in 0..new_assignments.len() {
            'find_shadow_map: for desired_quality in 0..quality_level_count {
                // If the assignment we already have is as good as anything else that could be
                // available, use it
                if let Some(assignment) = &new_assignments[shadow_view_index] {
                    if assignment.quality() <= desired_quality as u8 {
                        break 'find_shadow_map;
                    }
                }

                // See if there is an unused element in the atlas
                if let Some(element) = shadow_map_atlas.allocate(desired_quality as u8) {
                    // An unused element is available, use it
                    shadow_maps_needing_redraw.insert(ShadowViewIndex(shadow_view_index));
                    new_assignments[shadow_view_index] = Some(element);
                    break 'find_shadow_map;
                }

                // Alternatively, try to find some other view that is lower priority
                while let Some(other_shadow_view_index) =
                    in_use_assignments_by_quality[desired_quality].pop()
                {
                    if other_shadow_view_index > shadow_view_index {
                        shadow_maps_needing_redraw.insert(ShadowViewIndex(shadow_view_index));
                        new_assignments[shadow_view_index] =
                            new_assignments[other_shadow_view_index].take();
                        break 'find_shadow_map;
                    }
                }
            }
        }

        let shadow_view_count = new_assignments
            .iter()
            .position(|x| x.is_none())
            .unwrap_or(new_assignments.len());
        // Reduce length of these to number of views that have shadow atlas elements assigned to them
        new_assignments.resize_with(shadow_view_count, || unreachable!());
        potential_views.resize_with(shadow_view_count, || unreachable!());

        let mut new_lookup = FnvHashMap::default();
        new_lookup.reserve(shadow_view_count);
        for (i, potential_view) in potential_views.iter().enumerate() {
            new_lookup.insert(potential_view.shadow_view_id, ShadowViewIndex(i));
        }

        // Force full redraws
        // for i in 0..shadow_view_count {
        //     shadow_maps_needing_redraw.insert(ShadowViewIndex(i));
        // }

        *shadow_map_lookup_by_shadow_view_id = new_lookup;
        *shadow_map_atlas_element_assignments = new_assignments;
    }

    // Iterate through all lights. If we have allocated a shadow map to the light, set up a render view for it.
    #[profiling::function]
    fn calculate_shadow_map_views(
        render_view_set: &RenderViewSet,
        extract_resources: &ExtractResources,
        shadow_map_lookup_by_shadow_view_id: &FnvHashMap<MeshAdvShadowViewId, ShadowViewIndex>,
        shadow_map_atlas_element_assignments: &Vec<Option<ShadowMapAtlasElement>>,
        shadow_maps_needing_redraw: &FnvHashSet<ShadowViewIndex>,
        out_shadow_map_render_views_meta: &mut Vec<MeshAdvShadowMapRenderViewMeta>,
        out_shadow_map_render_views: &mut Vec<Option<RenderView>>,
        out_shadow_map_lookup_by_view_index: &mut FnvHashMap<RenderViewIndex, ShadowViewIndex>,
        out_shadow_map_lookup_by_light_id: &mut FnvHashMap<
            MeshAdvLightId,
            MeshAdvShadowMapRenderViewIndices,
        >,
    ) {
        let world_fetch = extract_resources.fetch::<World>();
        let world = &*world_fetch;

        // These are out parameters for the below function
        out_shadow_map_render_views_meta.clear();
        out_shadow_map_render_views.clear();
        out_shadow_map_lookup_by_view_index.clear();
        out_shadow_map_lookup_by_light_id.clear();

        let mut shadow_map_render_views_meta =
            vec![None; shadow_map_atlas_element_assignments.len()];
        let mut shadow_map_render_views = vec![None; shadow_map_atlas_element_assignments.len()];

        let shadow_map_phase_mask = RenderPhaseMaskBuilder::default()
            .add_render_phase::<ShadowMapRenderPhase>()
            .build();

        let render_options = extract_resources.fetch::<MeshAdvRenderOptions>();

        let shadow_map_feature_mask = if render_options.show_surfaces
            && render_options.show_shadows
            && render_options.enable_lighting
        {
            RenderFeatureMaskBuilder::default()
                .add_render_feature::<MeshAdvRenderFeature>()
                .build()
        } else {
            RenderFeatureMask::empty()
        };

        //TODO: The look-at calls in this fn will fail if the light is pointed straight down

        //
        // Handle spot lights
        //
        let mut query = <(Entity, Read<SpotLightComponent>, Read<TransformComponent>)>::query();
        for (entity, light, transform) in query.iter(world) {
            let shadow_view_id = MeshAdvShadowViewId::SpotLight(ObjectId::from(*entity));
            let shadow_view_index = shadow_map_lookup_by_shadow_view_id.get(&shadow_view_id);

            if let Some(&shadow_view_index) = shadow_view_index {
                //TODO: Transform direction by rotation
                let eye_position = transform.translation;
                let light_to = transform.translation + light.direction;
                let view =
                    glam::Mat4::look_at_rh(eye_position, light_to, glam::Vec3::new(0.0, 0.0, 1.0));

                let near_plane = 0.25;
                let far_plane = 100.0;
                let projection = Projection::Perspective(PerspectiveParameters::new(
                    light.spotlight_half_angle * 2.0,
                    1.0,
                    near_plane,
                    far_plane,
                    DepthRange::Reverse,
                ));
                let proj = projection.as_rh_mat4();

                let depth_range = RenderViewDepthRange::from_projection(&projection);
                shadow_map_render_views_meta[shadow_view_index.0] = Some(
                    MeshAdvShadowMapRenderViewMeta::new(&view, &proj, depth_range.clone()),
                );

                if shadow_maps_needing_redraw.contains(&shadow_view_index) {
                    let view_frustum: ViewFrustumArc = light.view_frustum.clone();
                    view_frustum.set_projection(&projection).set_transform(
                        eye_position,
                        light_to,
                        glam::Vec3::new(0.0, 0.0, 1.0),
                    );

                    let shadow_map_assignment = shadow_map_atlas_element_assignments
                        [shadow_view_index.0]
                        .as_ref()
                        .unwrap();
                    let resolution = shadow_map_assignment.texture_size_pixels() as u32;

                    let view = render_view_set.create_view(
                        view_frustum.clone(),
                        eye_position,
                        view,
                        proj,
                        (resolution, resolution),
                        depth_range,
                        shadow_map_phase_mask,
                        shadow_map_feature_mask,
                        RenderFeatureFlagMask::empty(),
                        "shadow_map_spotlight".to_string(),
                    );

                    out_shadow_map_lookup_by_view_index
                        .insert(view.view_index(), shadow_view_index);
                    shadow_map_render_views[shadow_view_index.0] = Some(view);
                }

                let light_id = MeshAdvLightId::SpotLight(ObjectId::from(*entity));
                out_shadow_map_lookup_by_light_id.insert(
                    light_id,
                    MeshAdvShadowMapRenderViewIndices::Single(shadow_view_index),
                );
            }
        }

        //
        // Handle directional lights
        //
        let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
        for (entity, light) in query.iter(world) {
            let shadow_view_id = MeshAdvShadowViewId::DirectionalLight(ObjectId::from(*entity));
            let shadow_view_index = shadow_map_lookup_by_shadow_view_id.get(&shadow_view_id);

            if let Some(&shadow_view_index) = shadow_view_index {
                //let shadow_map_assignment = shadow_map_atlas_element_assignments[shadow_view_index].as_ref().unwrap();

                let eye_position = light.direction * -40.0;
                let view = glam::Mat4::look_at_rh(
                    eye_position,
                    glam::Vec3::ZERO,
                    glam::Vec3::new(0.0, 0.0, 1.0),
                );

                let near_plane = 0.25;
                let far_plane = 100.0;
                let ortho_projection_size = 10.0;
                let view_frustum: ViewFrustumArc = light.view_frustum.clone();
                let projection = Projection::Orthographic(OrthographicParameters::new(
                    -ortho_projection_size,
                    ortho_projection_size,
                    -ortho_projection_size,
                    ortho_projection_size,
                    near_plane,
                    far_plane,
                    DepthRange::Reverse,
                ));
                let proj = projection.as_rh_mat4();

                let depth_range = RenderViewDepthRange::from_projection(&projection);
                shadow_map_render_views_meta[shadow_view_index.0] = Some(
                    MeshAdvShadowMapRenderViewMeta::new(&view, &proj, depth_range.clone()),
                );

                if shadow_maps_needing_redraw.contains(&shadow_view_index) {
                    view_frustum.set_projection(&projection).set_transform(
                        eye_position,
                        glam::Vec3::ZERO,
                        glam::Vec3::new(0.0, 0.0, 1.0),
                    );

                    let shadow_map_assignment = shadow_map_atlas_element_assignments
                        [shadow_view_index.0]
                        .as_ref()
                        .unwrap();
                    let resolution = shadow_map_assignment.texture_size_pixels() as u32;

                    let view = render_view_set.create_view(
                        view_frustum,
                        eye_position,
                        view,
                        proj,
                        (resolution, resolution),
                        depth_range,
                        shadow_map_phase_mask,
                        shadow_map_feature_mask,
                        RenderFeatureFlagMask::empty(),
                        "shadow_map_directional".to_string(),
                    );

                    out_shadow_map_lookup_by_view_index
                        .insert(view.view_index(), shadow_view_index);
                    shadow_map_render_views[shadow_view_index.0] = Some(view);
                }

                let light_id = MeshAdvLightId::DirectionalLight(ObjectId::from(*entity));
                out_shadow_map_lookup_by_light_id.insert(
                    light_id,
                    MeshAdvShadowMapRenderViewIndices::Single(shadow_view_index),
                );
            }
        }

        #[rustfmt::skip]
            // The eye offset and up vector. The directions are per the specification of cubemaps
            let cube_map_view_directions = [
            (glam::Vec3::X, glam::Vec3::Y),
            (glam::Vec3::X * -1.0, glam::Vec3::Y),
            (glam::Vec3::Y, glam::Vec3::Z * -1.0),
            (glam::Vec3::Y * -1.0, glam::Vec3::Z),
            (glam::Vec3::Z, glam::Vec3::Y),
            (glam::Vec3::Z * -1.0, glam::Vec3::Y),
        ];

        //
        // Handle point lights
        //
        let mut query = <(Entity, Read<PointLightComponent>, Read<TransformComponent>)>::query();
        for (entity, light, transform) in query.iter(world) {
            let mut any_face_has_shadow_map = false;
            let mut shadow_view_indices = [None; 6];

            for face_index in 0..6 {
                let shadow_view_id =
                    MeshAdvShadowViewId::PointLight(ObjectId::from(*entity), face_index as u8);
                let shadow_view_index = shadow_map_lookup_by_shadow_view_id.get(&shadow_view_id);

                if let Some(&shadow_view_index) = shadow_view_index {
                    let cube_map_view_directions = cube_map_view_directions[face_index];
                    let eye_position = transform.translation;

                    let near = 0.25;
                    let far = light.range;

                    let view_frustum: ViewFrustumArc = light.view_frustums[face_index].clone();
                    let projection = Projection::Perspective(PerspectiveParameters::new(
                        std::f32::consts::FRAC_PI_2,
                        1.0,
                        near,
                        far,
                        DepthRange::Reverse,
                    ));
                    let proj = projection.as_lh_mat4();

                    // NOTE: Cubemaps always use LH
                    let view = glam::Mat4::look_at_lh(
                        eye_position,
                        eye_position + cube_map_view_directions.0,
                        cube_map_view_directions.1,
                    );

                    let depth_range = RenderViewDepthRange::from_projection(&projection);
                    shadow_map_render_views_meta[shadow_view_index.0] = Some(
                        MeshAdvShadowMapRenderViewMeta::new(&view, &proj, depth_range.clone()),
                    );
                    shadow_view_indices[face_index] = Some(shadow_view_index);
                    any_face_has_shadow_map = true;

                    if shadow_maps_needing_redraw.contains(&shadow_view_index) {
                        view_frustum.set_projection(&projection).set_transform(
                            eye_position,
                            eye_position + cube_map_view_directions.0,
                            cube_map_view_directions.1,
                        );

                        let shadow_map_assignment = shadow_map_atlas_element_assignments
                            [shadow_view_index.0]
                            .as_ref()
                            .unwrap();
                        let resolution = shadow_map_assignment.texture_size_pixels() as u32;

                        let view = render_view_set.create_view(
                            view_frustum,
                            eye_position,
                            view,
                            proj,
                            (resolution, resolution),
                            RenderViewDepthRange::from_projection(&projection),
                            shadow_map_phase_mask,
                            shadow_map_feature_mask,
                            RenderFeatureFlagMask::empty(),
                            format!("shadow_map_point_light_face_{}", face_index),
                        );

                        out_shadow_map_lookup_by_view_index
                            .insert(view.view_index(), shadow_view_index);
                        shadow_map_render_views[shadow_view_index.0] = Some(view);
                    }
                }
            }

            if any_face_has_shadow_map {
                MeshAdvShadowMapRenderViewIndices::Cube(shadow_view_indices).unwrap_cube_any();

                let light_id = MeshAdvLightId::PointLight(ObjectId::from(*entity));
                out_shadow_map_lookup_by_light_id.insert(
                    light_id,
                    MeshAdvShadowMapRenderViewIndices::Cube(shadow_view_indices),
                );
            }
        }

        *out_shadow_map_render_views = shadow_map_render_views;
        *out_shadow_map_render_views_meta = shadow_map_render_views_meta
            .into_iter()
            .map(|x| x.unwrap())
            .collect();
    }
}
