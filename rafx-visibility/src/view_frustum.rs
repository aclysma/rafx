use crate::geometry::Frustum;
use crate::{DepthRange, OrthographicParameters, PerspectiveParameters, Projection, UpdateFrustum};
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct ViewFrustum {
    //NOTE: We could cache the frustum here and RwLock it, but I think we're better off to accept
    // a bit of overhead to recreate it. If it shows up in a profiler, we can put it back in.
    projection: Projection,
    eye_position: Vec3,
    look_at: Vec3,
    up: Vec3,
}

impl ViewFrustum {
    pub const NEAR: usize = 0;
    pub const FAR: usize = 1;
    pub const LEFT: usize = 2;
    pub const RIGHT: usize = 3;
    pub const TOP: usize = 4;
    pub const BOTTOM: usize = 5;

    pub fn empty() -> Self {
        ViewFrustum {
            projection: Projection::Undefined,
            eye_position: Default::default(),
            look_at: Default::default(),
            up: Default::default(),
        }
    }

    pub fn new_perspective(
        eye_position: Vec3,
        look_at: Vec3,
        up: Vec3,
        fov_y_radians: f32,
        ratio: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) -> Self {
        let mut view_frustum = ViewFrustum::empty();
        view_frustum.set_perspective(
            fov_y_radians,
            ratio,
            near_distance,
            far_distance,
            depth_range,
        );
        view_frustum.set_transforms(eye_position, look_at, up);
        view_frustum
    }

    pub fn new_orthographic(
        eye_position: Vec3,
        look_at: Vec3,
        up: Vec3,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) -> Self {
        let mut view_frustum = ViewFrustum::empty();
        view_frustum.set_orthographic(
            left,
            right,
            bottom,
            top,
            near_distance,
            far_distance,
            depth_range,
        );
        view_frustum.set_transforms(eye_position, look_at, up);
        view_frustum
    }

    pub fn get_projection(&self) -> &Projection {
        &self.projection
    }

    pub fn set_perspective(
        &mut self,
        fov_y_radians: f32,
        ratio: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) {
        self.projection = Projection::Perspective(PerspectiveParameters::new(
            fov_y_radians,
            ratio,
            near_distance,
            far_distance,
            depth_range,
        ));
    }

    pub fn set_orthographic(
        &mut self,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) {
        self.projection = Projection::Orthographic(OrthographicParameters::new(
            left,
            right,
            bottom,
            top,
            near_distance,
            far_distance,
            depth_range,
        ));
    }

    pub fn set_transforms(
        &mut self,
        eye_position: Vec3,
        look_at: Vec3,
        up: Vec3,
    ) {
        self.eye_position = eye_position;
        self.look_at = look_at;
        self.up = up;
    }

    pub fn eye_position(&self) -> Vec3 {
        self.eye_position
    }

    pub fn look_at(&self) -> Vec3 {
        self.look_at
    }

    pub fn up(&self) -> Vec3 {
        self.up
    }

    pub fn acquire_frustum(&self) -> Frustum {
        let mut frustum = Frustum::default();
        self.projection.update_frustum(self, &mut frustum);
        frustum
    }
}
