use crate::geometry::{BoundingSphere, Frustum};
use crate::{ObjectHandle, VisibilityResult, VisibleObjects};
use glam::{Vec3, Vec4};

#[derive(Default, Copy, Clone)]
pub struct ObjectMetadata {
    pub handle: ObjectHandle,
    pub id: u64,
}

#[derive(Copy, Clone)]
pub struct PackedBoundingSphereChunk {
    spheres: [PackedBoundingSphere; PackedBoundingSphereChunk::CHUNK_SIZE],
    metadata: [ObjectMetadata; PackedBoundingSphereChunk::MAX_LEN],
    len: usize,
}

#[derive(Clone, Copy)]
struct Index(usize);

#[derive(Clone, Copy)]
struct InternalIndex(usize, usize);

impl PackedBoundingSphereChunk {
    pub(crate) const CHUNK_SIZE: usize = 512;

    pub const MAX_LEN: usize =
        PackedBoundingSphereChunk::CHUNK_SIZE * PackedBoundingSphere::NUM_PACKED;

    pub fn new() -> Self {
        PackedBoundingSphereChunk {
            len: 0,
            metadata: [ObjectMetadata::default(); PackedBoundingSphereChunk::MAX_LEN],
            spheres: [PackedBoundingSphere::default(); PackedBoundingSphereChunk::CHUNK_SIZE],
        }
    }

    pub fn len(&self) -> usize {
        assert!(self.len <= PackedBoundingSphereChunk::MAX_LEN);
        self.len
    }

    pub fn add(
        &mut self,
        handle: ObjectHandle,
        id: u64,
        sphere: BoundingSphere,
    ) -> Option<usize> {
        let next_index = Index(self.len());
        let internal_index = self.get_internal_index(next_index);
        return if internal_index.0 < PackedBoundingSphereChunk::CHUNK_SIZE {
            self.metadata[next_index.0] = ObjectMetadata { handle, id };
            self.set_internal(internal_index, sphere);
            self.len += 1;
            Some(next_index.0)
        } else {
            None
        };
    }

    pub fn update_id(
        &mut self,
        index: usize,
        id: u64,
    ) {
        let index = Index(index);
        self.assert_index_valid(index);
        self.metadata[index.0].id = id;
    }

    pub fn update(
        &mut self,
        index: usize,
        sphere: BoundingSphere,
    ) {
        let index = Index(index);
        self.assert_index_valid(index);
        self.set_internal(self.get_internal_index(index), sphere);
    }

    pub fn remove(
        &mut self,
        index: usize,
    ) -> bool {
        let index = Index(index);
        return if index.0 < self.len() {
            // NOTE(dvd): Overwrite with last sphere, starting with the handle.
            self.metadata[index.0] = self.metadata[self.len() - 1];
            let last_sphere = self.get_internal(self.get_internal_index(Index(self.len() - 1)));
            self.set_internal(self.get_internal_index(index), last_sphere);
            self.len -= 1;
            true
        } else {
            false
        };
    }

    pub fn metadata(
        &self,
        index: usize,
    ) -> &ObjectMetadata {
        let index = Index(index);
        self.assert_index_valid(index);
        &self.metadata[index.0]
    }

    #[allow(dead_code)]
    pub fn get(
        &self,
        index: usize,
    ) -> BoundingSphere {
        let index = Index(index);
        self.assert_index_valid(index);
        self.get_internal(self.get_internal_index(index))
    }

    // NOTE(dvd): Does not bounds check against len().
    fn get_internal(
        &self,
        internal_index: InternalIndex,
    ) -> BoundingSphere {
        PackedBoundingSphereChunk::assert_internal_index_valid(internal_index);
        let packed_sphere: &PackedBoundingSphere = &self.spheres[internal_index.0];
        packed_sphere.get(internal_index.1)
    }

    // NOTE(dvd): Does not bounds check against len().
    fn set_internal(
        &mut self,
        internal_index: InternalIndex,
        sphere: BoundingSphere,
    ) {
        PackedBoundingSphereChunk::assert_internal_index_valid(internal_index);
        let packed_sphere: &mut PackedBoundingSphere = &mut self.spheres[internal_index.0];
        packed_sphere.set(internal_index.1, sphere);
    }

    #[inline(always)]
    fn assert_index_valid(
        &self,
        index: Index,
    ) {
        assert!(index.0 < self.len);
    }

    #[inline(always)]
    fn get_internal_index(
        &self,
        index: Index,
    ) -> InternalIndex {
        InternalIndex(
            index.0 / PackedBoundingSphere::NUM_PACKED,
            index.0 % PackedBoundingSphere::NUM_PACKED,
        )
    }

    #[inline(always)]
    fn assert_internal_index_valid(internal_index: InternalIndex) {
        assert!(internal_index.0 < PackedBoundingSphereChunk::CHUNK_SIZE);
    }
}

pub(crate) fn collect_visible_objects(
    chunk: &PackedBoundingSphereChunk,
    view_frustum_position: Vec3,
    frustum: &Frustum,
    results: &mut VisibleObjects,
) {
    return if frustum.planes.len() == 6 {
        let mut planes = [Vec4::ZERO; 6];
        planes[0] = frustum.planes[0].normal;
        planes[1] = frustum.planes[1].normal;
        planes[2] = frustum.planes[2].normal;
        planes[3] = frustum.planes[3].normal;
        planes[4] = frustum.planes[4].normal;
        planes[5] = frustum.planes[5].normal;

        collect_visible_objects_fast(chunk, view_frustum_position, &planes, results);
    } else {
        // TODO(dvd): Write other methods if we need non-standard frustum.
        panic!(
            "Implement new find_visible_objects_in_vec for frustum with {} planes.",
            frustum.planes.len()
        );
    };
}

fn collect_visible_objects_fast(
    chunk: &PackedBoundingSphereChunk,
    view_frustum_position: Vec3,
    planes: &[Vec4; 6],
    results: &mut VisibleObjects,
) {
    let mut query = Query {
        view_frustum_position,
        spheres: &chunk.spheres[0],
        handle_index: 0,
        bitmask: 0,
        metadata: &chunk.metadata,
        results,
    };

    let len = chunk.len();
    if len == 0 {
        return;
    }

    let last_index = (len - 1) / PackedBoundingSphere::NUM_PACKED;

    if last_index > 0 {
        // NOTE(dvd): Check as many spheres as possible in a fast loop.
        query.handle_index = 0;
        for index in 0..last_index {
            query.spheres = &chunk.spheres[index];
            query.bitmask = query.spheres.is_contained_by(planes);
            query.try_push_visibility_result(0);
            query.try_push_visibility_result(1);
            query.handle_index += PackedBoundingSphere::NUM_PACKED;
        }
    }

    // NOTE(dvd): Check the last 2 spheres.
    query.handle_index = last_index * PackedBoundingSphere::NUM_PACKED;
    query.spheres = &chunk.spheres[last_index];
    query.bitmask = query.spheres.is_contained_by(planes);
    query.try_push_visibility_result(0);
    if len & 1 == 0 {
        // NOTE(dvd): If len is even, the last packed sphere has 2 spheres in it.
        query.try_push_visibility_result(1);
    }
}

struct Query<'a> {
    pub view_frustum_position: Vec3,
    pub spheres: &'a PackedBoundingSphere,
    pub handle_index: usize,
    pub bitmask: i32,
    pub metadata: &'a [ObjectMetadata; PackedBoundingSphereChunk::MAX_LEN],
    pub results: &'a mut VisibleObjects,
}

impl Query<'_> {
    #[inline(always)]
    fn try_push_visibility_result(
        &mut self,
        packed_index: usize,
    ) {
        if (self.bitmask & (1 << packed_index)) > 0 {
            let object = self.metadata[self.handle_index + packed_index];
            self.results.push(VisibilityResult::new(
                object.handle,
                object.id,
                self.view_frustum_position,
                self.spheres.get(packed_index),
            ));
        }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, PartialEq)]
struct PackedBoundingSphere {
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub z1: f32,
    pub z2: f32,
    pub radius1: f32,
    pub radius2: f32,
}

impl PackedBoundingSphere {
    pub const NUM_PACKED: usize = 2;

    pub fn set(
        &mut self,
        index: usize,
        sphere: BoundingSphere,
    ) {
        PackedBoundingSphere::assert_index_valid(index);
        if index == 0 {
            self.x1 = sphere.position.x;
            self.y1 = sphere.position.y;
            self.z1 = sphere.position.z;
            self.radius1 = sphere.radius;
        } else {
            self.x2 = sphere.position.x;
            self.y2 = sphere.position.y;
            self.z2 = sphere.position.z;
            self.radius2 = sphere.radius;
        };
    }

    pub fn get(
        &self,
        index: usize,
    ) -> BoundingSphere {
        PackedBoundingSphere::assert_index_valid(index);
        return if index == 0 {
            BoundingSphere {
                position: Vec3::new(self.x1, self.y1, self.z1),
                radius: self.radius1,
            }
        } else {
            BoundingSphere {
                position: Vec3::new(self.x2, self.y2, self.z2),
                radius: self.radius2,
            }
        };
    }

    #[inline(always)]
    pub fn is_contained_by(
        &self,
        planes: &[Vec4; 6],
    ) -> i32 {
        let mut bitmask = 0;
        bitmask |= (PackedBoundingSphere::is_contained_by_simd(
            planes,
            self.x1,
            self.y1,
            self.z1,
            self.radius1,
        ) as i32)
            << 0;
        bitmask |= (PackedBoundingSphere::is_contained_by_simd(
            planes,
            self.x2,
            self.y2,
            self.z2,
            self.radius2,
        ) as i32)
            << 1;
        bitmask
    }

    #[inline(always)]
    fn is_contained_by_simd(
        planes: &[Vec4; 6],
        spx: f32,
        spy: f32,
        spz: f32,
        radius: f32,
    ) -> bool {
        let p1 = planes[0];
        let p2 = planes[1];
        let p3 = planes[2];
        let p4 = planes[3];
        let p5 = planes[4];
        let p6 = planes[5];

        let mut bitmask = 0;
        bitmask |= ((p1.w + p1.x * spx + p1.y * spy + p1.z * spz + radius <= 0.) as i32) << 0;
        bitmask |= ((p2.w + p2.x * spx + p2.y * spy + p2.z * spz + radius <= 0.) as i32) << 1;
        bitmask |= ((p3.w + p3.x * spx + p3.y * spy + p3.z * spz + radius <= 0.) as i32) << 2;
        bitmask |= ((p4.w + p4.x * spx + p4.y * spy + p4.z * spz + radius <= 0.) as i32) << 3;
        bitmask |= ((p5.w + p5.x * spx + p5.y * spy + p5.z * spz + radius <= 0.) as i32) << 4;
        bitmask |= ((p6.w + p6.x * spx + p6.y * spy + p6.z * spz + radius <= 0.) as i32) << 5;
        return bitmask <= 0;
    }

    #[inline(always)]
    fn assert_index_valid(index: usize) {
        debug_assert!(index < PackedBoundingSphere::NUM_PACKED);
    }
}
