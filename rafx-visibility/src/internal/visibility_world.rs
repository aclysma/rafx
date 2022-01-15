use crate::frustum_culling::PackedBoundingSphereChunk;
use crate::geometry::{BoundingSphere, Transform};
use crate::internal::{VisibilityObject, Volume, Zone};
use crate::{
    DepthRange, ModelHandle, PolygonSoup, PolygonSoupIndex, ViewFrustum, ViewFrustumHandle,
    VisibilityObjectHandle, VisibleBounds, VolumeHandle, ZoneHandle,
};
use glam::Vec3;
use rustc_hash::FxHashMap;
use slotmap::{DenseSlotMap, SecondaryMap, SlotMap};

pub struct VisibilityWorldInternal {
    pub(crate) zones: DenseSlotMap<ZoneHandle, Zone>,

    pub(crate) models: SlotMap<ModelHandle, VisibleBounds>,
    pub(crate) model_ref_counts: SecondaryMap<ModelHandle, u64>,
    pub(crate) model_hashes: FxHashMap<u64, ModelHandle>,

    pub(crate) objects: DenseSlotMap<VisibilityObjectHandle, VisibilityObject>,

    pub(crate) view_frustums: DenseSlotMap<ViewFrustumHandle, ViewFrustum>,
    pub(crate) view_frustum_ids: SecondaryMap<ViewFrustumHandle, u64>,
    pub(crate) view_frustum_zones: SecondaryMap<ViewFrustumHandle, ZoneHandle>,

    #[allow(dead_code)]
    pub(crate) volumes: DenseSlotMap<VolumeHandle, Volume>,
}

impl VisibilityWorldInternal {
    pub fn new() -> Self {
        VisibilityWorldInternal {
            zones: Default::default(),

            models: Default::default(),
            model_ref_counts: Default::default(),
            model_hashes: Default::default(),

            objects: Default::default(),

            view_frustums: Default::default(),
            view_frustum_ids: Default::default(),
            view_frustum_zones: Default::default(),

            volumes: Default::default(),
        }
    }

    // --------
    // Zones
    // --------

    /// A `Zone` contains `Objects` & `ViewFrustums`, similar to the concept of a `World` or `Layer` in a collision API.
    /// Visibility queries can only traverse zones through a portal. Portals are unimplemented.
    pub fn new_zone(&mut self) -> ZoneHandle {
        self.zones.insert(Zone::new())
    }

    /// All `Objects`, `ViewFrustums`, and `Volumes` must be removed from `Zone`.
    pub fn destroy_zone(
        &mut self,
        zone: ZoneHandle,
    ) {
        let removed = self.zones.remove(zone).unwrap();
        assert_eq!(removed.objects.len(), 0);
        assert_eq!(removed.volumes.len(), 0);
    }

    // --------
    // View Frustums
    // --------

    /// Creates a new `ViewFrustum`. A `ViewFrustum` must be in a `Zone` to query visibility.
    pub fn new_view_frustum(&mut self) -> ViewFrustumHandle {
        self.view_frustums.insert(ViewFrustum::empty())
    }

    pub fn view_frustum(
        &self,
        handle: ViewFrustumHandle,
    ) -> Option<&ViewFrustum> {
        self.view_frustums.get(handle)
    }

    /// Sets the `ViewFrustum`'s ID. This is an arbitrary 64-bit number for use by the application.
    /// It should correspond to a game object ID, or a pointer, or an ECS entity ID.
    pub fn set_view_frustum_id(
        &mut self,
        view_frustum: ViewFrustumHandle,
        id: u64,
    ) {
        self.view_frustum_ids.insert(view_frustum, id);
    }

    /// Sets `angle`, `ratio`, `near_distance`, and `far_distance` for perspective `ViewFrustum`.
    pub fn set_view_frustum_perspective(
        &mut self,
        view_frustum: ViewFrustumHandle,
        fov_y_radians: f32,
        ratio: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) {
        let view_frustum = self.view_frustums.get_mut(view_frustum).unwrap();
        view_frustum.set_perspective(
            fov_y_radians,
            ratio,
            near_distance,
            far_distance,
            depth_range,
        );
    }

    /// Sets `left`, `right`, `bottom`, `top`, `near_distance`, and `far_distance` for orthographic `ViewFrustum`.
    pub fn set_view_frustum_orthographic(
        &mut self,
        view_frustum: ViewFrustumHandle,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) {
        let view_frustum = self.view_frustums.get_mut(view_frustum).unwrap();
        view_frustum.set_orthographic(
            left,
            right,
            bottom,
            top,
            near_distance,
            far_distance,
            depth_range,
        );
    }

    /// Sets the `ViewFrustum`'s `Zone`. A `ViewFrustum` must be in a `Zone` to query visibility.
    pub fn set_view_frustum_zone(
        &mut self,
        view_frustum: ViewFrustumHandle,
        zone: Option<ZoneHandle>,
    ) {
        if let Some(zone) = zone {
            self.view_frustum_zones.insert(view_frustum, zone);
        } else {
            self.view_frustum_zones.remove(view_frustum);
        }
    }

    /// Returns the `Object`'s `Zone`. A `ViewFrustum` must be in a `Zone` to query visibility.
    pub fn get_view_frustum_zone(
        &self,
        view_frustum: ViewFrustumHandle,
    ) -> Option<&ZoneHandle> {
        self.view_frustum_zones.get(view_frustum)
    }

    /// Sets the `ViewFrustum`'s transform relative to the `Zone`'s position.
    pub fn set_view_frustum_transforms(
        &mut self,
        view_frustum: ViewFrustumHandle,
        eye_position: Vec3,
        look_at: Vec3,
        up: Vec3,
    ) {
        let view_frustum = self.view_frustums.get_mut(view_frustum).unwrap();
        view_frustum.set_transforms(eye_position, look_at, up);
    }

    /// Destroying a `ViewFrustum` will also remove it from the `Zone`.
    pub fn destroy_view_frustum(
        &mut self,
        view_frustum: ViewFrustumHandle,
    ) {
        self.set_view_frustum_zone(view_frustum, None);
        self.view_frustums.remove(view_frustum);
    }

    // --------
    // Models
    // --------

    /// Returns a handle to a `Model` created from `PolygonSoup`
    pub fn new_model(
        &mut self,
        polygons: PolygonSoup,
    ) -> ModelHandle {
        let hash = polygons.calculate_hash();
        return if let Some(handle) = self.model_hashes.get(&hash) {
            // NOTE(dvd): Return the existing model.
            *handle
        } else {
            // NOTE(dvd): Create a new model.
            let handle = self.models.insert(VisibleBounds::new(hash, polygons));
            self.model_hashes.insert(hash, handle);
            handle
        };
    }

    /// Returns a handle to a `Model` created from `VisibleBounds`
    pub fn new_visible_bounds(
        &mut self,
        bounds: VisibleBounds,
    ) -> ModelHandle {
        let hash = bounds.hash;
        return if let Some(handle) = self.model_hashes.get(&hash) {
            // NOTE(dvd): Return the existing model.
            *handle
        } else {
            // NOTE(dvd): Store the model.
            let handle = self.models.insert(bounds);
            self.model_hashes.insert(hash, handle);
            handle
        };
    }

    /// Returns a `ModelHandle` sized to contain a `BoundingSphere` with `radius`.
    pub fn new_bounding_sphere(
        &mut self,
        radius: f32,
    ) -> ModelHandle {
        // NOTE(dvd): In a 45-45-90 triangle, the hypotenuse h = a * sqrt(2).
        // So if h = diameter, then radius * 2 = a*sqrt(2), then a = (radius * 2) / sqrt(2).
        let a = (2. * radius) / f32::sqrt(2.);
        self.new_quad(a, a)
    }

    /// Returns a `ModelHandle` sized to contain a `Quad` with `width` and `height`.
    pub fn new_quad(
        &mut self,
        width: f32,
        height: f32,
    ) -> ModelHandle {
        let one_half_width = 0.5 * width;
        let one_half_height = 0.5 * height;
        let lt = Vec3::new(-one_half_width, one_half_height, 0.);
        let rt = Vec3::new(one_half_width, one_half_height, 0.);
        let lb = Vec3::new(-one_half_width, -one_half_height, 0.);
        let rb = Vec3::new(one_half_width, -one_half_height, 0.);
        let polygon_soup = PolygonSoup {
            vertex_positions: [lt, rt, lb, rb].to_vec(),
            index: PolygonSoupIndex::Indexed16([2, 0, 1, 1, 3, 2].to_vec()),
        };
        self.new_model(polygon_soup)
    }

    /// Returns `true` if the `Model` could be destroyed.
    /// If the `Model` is referenced by an `Object`, this function will return `false`.
    pub fn destroy_model(
        &mut self,
        model: ModelHandle,
    ) -> bool {
        if let Some(count) = self.model_ref_counts.get(model) {
            if *count > 0 {
                return false;
            }
        }

        let removed_model = self.models.remove(model).unwrap();
        self.model_hashes.remove(&removed_model.hash);
        true
    }

    // --------
    // Objects
    // --------

    /// Creates a new `Object`. An `Object` must be in a `Zone` to be visible.
    pub fn new_object(&mut self) -> VisibilityObjectHandle {
        self.objects
            .insert_with_key(|handle| VisibilityObject::new(0, handle))
    }

    pub fn visibility_object(
        &self,
        handle: VisibilityObjectHandle,
    ) -> Option<&VisibilityObject> {
        self.objects.get(handle)
    }

    /// Sets the `Object`'s ID. This is an arbitrary 64-bit number for use by the application.
    /// It should correspond to a game object ID, or a pointer, or an ECS entity ID.
    pub fn set_object_id(
        &mut self,
        object: VisibilityObjectHandle,
        id: u64,
    ) {
        let object = self.objects.get_mut(object).unwrap();
        object.id = id;
        if let Some(zone) = object.zone {
            let zone = self.zones.get_mut(zone).unwrap();

            let (chunk_idx, in_chunk_idx) = *zone.objects.get(object.handle).unwrap();
            let chunk: &mut PackedBoundingSphereChunk = zone.chunks.get_mut(chunk_idx).unwrap();

            chunk.update_id(in_chunk_idx, id);
        }
    }

    /// Sets the `Object`'s `Zone`. An `Object` must be in a `Zone` to be visible.
    pub fn set_object_zone(
        &mut self,
        object: VisibilityObjectHandle,
        zone: Option<ZoneHandle>,
    ) {
        let handle = object;
        let object = self.objects.get(handle).unwrap();

        if let Some(zone) = object.zone {
            self.internal_remove_object_in_zone(handle, zone);
        }

        if let Some(zone) = zone {
            self.internal_add_object_to_zone(handle, zone);
            self.internal_update_object_in_zone(handle, zone);
        }

        let object = self.objects.get_mut(handle).unwrap();
        object.zone = zone;
    }

    /// Sets the `Object`'s position relative to the `Zone`'s position.
    pub fn set_object_transform(
        &mut self,
        object: VisibilityObjectHandle,
        transform: Transform,
    ) {
        let handle = object;
        let object = self.objects.get_mut(object).unwrap();
        object.transform = Some(transform);

        if let Some(zone) = object.zone {
            self.internal_update_object_in_zone(handle, zone);
        }
    }

    /// Sets the `Object`'s cull `Model`. The cull `Model` is tested against the occlusion buffer.
    /// This is like a `Collider` in a collision API.
    pub fn set_object_cull_model(
        &mut self,
        object: VisibilityObjectHandle,
        model: Option<ModelHandle>,
    ) {
        let handle = object;
        let object = self.objects.get_mut(object).unwrap();

        if let Some(model) = object.cull_model {
            let count = *self.model_ref_counts.get(model).unwrap();
            self.model_ref_counts.insert(model, count - 1);
        }

        object.cull_model = model;

        if let Some(model) = object.cull_model {
            let count = self.model_ref_counts.get(model).map_or(0, |count| *count);
            self.model_ref_counts.insert(model, count + 1);
        }

        if let Some(zone) = object.zone {
            self.internal_update_object_in_zone(handle, zone);
        }
    }

    /// Destroying an `Object` will also remove it from the `Zone`.
    /// This will **NOT** destroy the cull `Model`.
    pub fn destroy_object(
        &mut self,
        object: VisibilityObjectHandle,
    ) {
        self.set_object_zone(object, None);
        self.set_object_cull_model(object, None);
        self.objects.remove(object).unwrap();
    }

    // --------
    // Volumes
    // --------

    /*
    /// Creates a new `Volume`. An `Volume` must be in a `Zone` to be visible.
    pub fn new_volume(&self) -> VolumeHandle {
        unimplemented!();
    }

    /// Sets the `Volume`'s ID. This is an arbitrary 64-bit number for use by the application.
    /// It should correspond to a game object ID, or a pointer, or an ECS entity ID.
    pub fn set_volume_id(
        &self,
        _volume: VolumeHandle,
        _id: u64,
    ) {
        unimplemented!();
    }

    /// Sets the `Volume`'s `Zone`. An `Volume` must be in a `Zone` to be visible.
    pub fn set_volume_zone(
        &self,
        _volume: VolumeHandle,
        _zone: ZoneHandle,
    ) {
        unimplemented!();
    }

    /// Returns the `Volume`'s `Zone`. An `Volume` must be in a `Zone` to be visible.
    pub fn get_volume_zone(
        &self,
        _volume: VolumeHandle,
    ) -> Option<ZoneHandle> {
        None
    }

    /// Sets the `Volume`'s position relative to the `Zone`'s position.
    pub fn set_volume_position(
        &self,
        _volume: VolumeHandle,
        _transform: Transform,
    ) {
        unimplemented!();
    }

    /// Returns the `Volume`'s position relative to the `Zone`'s position.
    pub fn get_volume_position(
        &self,
        _volume: VolumeHandle,
    ) -> Transform {
        Transform::default()
    }

    /// Sets the `Volume`'s `Model`. The `Model` is tested against the `ViewFrustum` frustum for intersections.
    /// This is like a `Collider` in a collision API.
    pub fn set_volume_model(
        &self,
        _volume: VolumeHandle,
        _model: ModelHandle,
    ) {
        unimplemented!();
    }

    /// Returns the `Model` associated with the `Volume`.
    pub fn get_volume_model(
        &self,
        _volume: VolumeHandle,
    ) -> Option<ModelHandle> {
        None
    }

    /// Destroying an `Volume` will also remove it from the `Zone`.
    /// This will **NOT** destroy the cull `Model`.
    pub fn destroy_volume(
        &self,
        _volume: VolumeHandle,
    ) {
        unimplemented!();
    }
    */

    fn internal_add_object_to_zone(
        &mut self,
        object: VisibilityObjectHandle,
        zone: ZoneHandle,
    ) {
        let object = self.objects.get(object).unwrap();
        let zone = self.zones.get_mut(zone).unwrap();
        let chunks = &mut zone.chunks;
        let mut chunk_idx = chunks.len();

        let next_chunk = {
            if chunks.is_empty() {
                chunks.push(PackedBoundingSphereChunk::new());
                chunks.last_mut().unwrap()
            } else {
                let last_chunk = chunks.last_mut().unwrap();
                if last_chunk.len() == PackedBoundingSphereChunk::MAX_LEN {
                    chunks.push(PackedBoundingSphereChunk::new());
                    chunks.last_mut().unwrap()
                } else {
                    chunk_idx -= 1;
                    last_chunk
                }
            }
        };

        let transform = object.transform.unwrap_or_default();
        let in_chunk_idx = next_chunk
            .add(
                object.handle,
                object.id,
                VisibilityObject::default_bounding_sphere(transform),
            )
            .unwrap();
        zone.objects
            .insert(object.handle, (chunk_idx, in_chunk_idx));
    }

    fn internal_update_object_in_zone(
        &mut self,
        object: VisibilityObjectHandle,
        zone: ZoneHandle,
    ) {
        let object = self.objects.get(object).unwrap();
        let zone = self.zones.get_mut(zone).unwrap();

        let (chunk_idx, in_chunk_idx) = *zone.objects.get(object.handle).unwrap();
        let chunk: &mut PackedBoundingSphereChunk = zone.chunks.get_mut(chunk_idx).unwrap();

        let transform = object.transform.unwrap_or_default();
        if let Some(model) = object.cull_model {
            let model = self.models.get(model).unwrap();
            chunk.update(
                in_chunk_idx,
                BoundingSphere::new(
                    transform.translation + model.bounding_sphere.position * transform.scale,
                    model.bounding_sphere.radius * transform.scale.max_element(),
                ),
            )
        } else {
            chunk.update(
                in_chunk_idx,
                VisibilityObject::default_bounding_sphere(transform),
            );
        }
    }

    fn internal_remove_object_in_zone(
        &mut self,
        object: VisibilityObjectHandle,
        zone: ZoneHandle,
    ) {
        let object = self.objects.get(object).unwrap();
        let zone = self.zones.get_mut(zone).unwrap();

        let (chunk_idx, in_chunk_idx) = zone.objects.remove(object.handle).unwrap();
        let chunk: &mut PackedBoundingSphereChunk = zone.chunks.get_mut(chunk_idx).unwrap();
        chunk.remove(in_chunk_idx);
        if in_chunk_idx < chunk.len() {
            let metadata = chunk.metadata(in_chunk_idx);
            zone.objects
                .insert(metadata.handle, (chunk_idx, in_chunk_idx));
        }
    }
}
