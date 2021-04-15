use crate::geometry::{Frustum, Plane};
use crate::ViewFrustum;
use glam::Mat4;

#[derive(Clone, Debug, PartialEq)]
pub enum Projection {
    Perspective(PerspectiveParameters),
    Orthographic(OrthographicParameters),
    Undefined,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DepthRange {
    Normal,
    Infinite,
    Reverse,
    InfiniteReverse,
}

impl Default for DepthRange {
    fn default() -> Self {
        DepthRange::Normal
    }
}

impl Projection {
    pub fn depth_range(&self) -> DepthRange {
        match &self {
            Projection::Perspective(parameters) => parameters.depth_range,
            Projection::Orthographic(parameters) => parameters.depth_range,
            _ => {
                panic!("`Undefined` is not a Projection.");
            }
        }
    }

    pub fn near_distance(&self) -> f32 {
        match &self {
            Projection::Perspective(parameters) => parameters.near_distance,
            Projection::Orthographic(parameters) => parameters.near_distance,
            _ => {
                panic!("`Undefined` is not a Projection.");
            }
        }
    }

    pub fn far_distance(&self) -> f32 {
        match &self {
            Projection::Perspective(parameters) => parameters.far_distance,
            Projection::Orthographic(parameters) => parameters.far_distance,
            _ => {
                panic!("`Undefined` is not a Projection.");
            }
        }
    }

    pub fn as_rh_mat4(&self) -> Mat4 {
        let (near, far) = if self.depth_range() == DepthRange::Normal
            || self.depth_range() == DepthRange::Infinite
        {
            (self.near_distance(), self.far_distance())
        } else {
            // Swap near & far.
            (self.far_distance(), self.near_distance())
        };

        match &self {
            Projection::Perspective(parameters) => match self.depth_range() {
                DepthRange::Infinite => glam::Mat4::perspective_infinite_rh(
                    parameters.fov_y_radians,
                    parameters.ratio,
                    parameters.near_distance,
                ),
                DepthRange::InfiniteReverse => glam::Mat4::perspective_infinite_reverse_rh(
                    parameters.fov_y_radians,
                    parameters.ratio,
                    parameters.near_distance,
                ),
                _ => glam::Mat4::perspective_rh(
                    parameters.fov_y_radians,
                    parameters.ratio,
                    near,
                    far,
                ),
            },
            Projection::Orthographic(parameters) => glam::Mat4::orthographic_rh(
                parameters.left,
                parameters.right,
                parameters.bottom,
                parameters.top,
                near,
                far,
            ),
            _ => {
                panic!("`Undefined` is not a Projection.");
            }
        }
    }

    pub fn as_lh_mat4(&self) -> Mat4 {
        let (near, far) = if self.depth_range() == DepthRange::Normal
            || self.depth_range() == DepthRange::Infinite
        {
            (self.near_distance(), self.far_distance())
        } else {
            // Swap near & far.
            (self.far_distance(), self.near_distance())
        };

        match &self {
            Projection::Perspective(parameters) => match self.depth_range() {
                DepthRange::Infinite => glam::Mat4::perspective_infinite_lh(
                    parameters.fov_y_radians,
                    parameters.ratio,
                    parameters.near_distance,
                ),
                DepthRange::InfiniteReverse => glam::Mat4::perspective_infinite_reverse_lh(
                    parameters.fov_y_radians,
                    parameters.ratio,
                    parameters.near_distance,
                ),
                _ => glam::Mat4::perspective_lh(
                    parameters.fov_y_radians,
                    parameters.ratio,
                    near,
                    far,
                ),
            },
            Projection::Orthographic(parameters) => glam::Mat4::orthographic_lh(
                parameters.left,
                parameters.right,
                parameters.bottom,
                parameters.top,
                near,
                far,
            ),
            _ => {
                panic!("`Undefined` is not a Projection.");
            }
        }
    }
}

pub(super) trait UpdateFrustum {
    fn update_frustum(
        &self,
        view_frustum: &ViewFrustum,
        frustum: &mut Frustum,
    );
}

impl UpdateFrustum for Projection {
    fn update_frustum(
        &self,
        view_frustum: &ViewFrustum,
        frustum: &mut Frustum,
    ) {
        match &self {
            Projection::Perspective(parameters) => {
                parameters.update_frustum(view_frustum, frustum);
            }
            Projection::Orthographic(parameters) => {
                parameters.update_frustum(view_frustum, frustum);
            }
            _ => {
                panic!("Call `set_perspective` or `set_orthographic` prior to calling `update_frustum`.");
            }
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct OrthographicParameters {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near_distance: f32,
    far_distance: f32,
    depth_range: DepthRange,
}

impl OrthographicParameters {
    pub fn new(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) -> Self {
        OrthographicParameters {
            left,
            right,
            bottom,
            top,
            near_distance,
            far_distance,
            depth_range,
        }
    }

    pub fn left(&self) -> f32 {
        self.left
    }

    pub fn right(&self) -> f32 {
        self.right
    }

    pub fn bottom(&self) -> f32 {
        self.bottom
    }

    pub fn top(&self) -> f32 {
        self.top
    }

    pub fn near_distance(&self) -> f32 {
        self.near_distance
    }

    pub fn far_distance(&self) -> f32 {
        self.far_distance
    }

    pub fn depth_range(&self) -> DepthRange {
        self.depth_range
    }
}

impl UpdateFrustum for OrthographicParameters {
    fn update_frustum(
        &self,
        view_frustum: &ViewFrustum,
        frustum: &mut Frustum,
    ) {
        let eye_position = view_frustum.eye_position();

        let z = (eye_position - view_frustum.look_at()).normalize();
        let x = (view_frustum.up().cross(z)).normalize();
        let y = z.cross(x);

        let near_center = eye_position - z * self.near_distance;
        let far_center = eye_position - z * self.far_distance;

        while frustum.planes.len() < 6 {
            frustum.planes.push(Plane::default());
        }

        frustum.planes[ViewFrustum::NEAR] = Plane::new(-z, near_center);
        frustum.planes[ViewFrustum::FAR] = Plane::new(z, far_center);

        frustum.planes[ViewFrustum::TOP] = Plane::new(-y, near_center + y * self.top);
        frustum.planes[ViewFrustum::BOTTOM] = Plane::new(y, near_center + y * self.bottom);

        frustum.planes[ViewFrustum::LEFT] = Plane::new(x, near_center + x * self.left);
        frustum.planes[ViewFrustum::RIGHT] = Plane::new(-x, near_center + x * self.right);
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct PerspectiveParameters {
    near_distance: f32,
    far_distance: f32,
    ratio: f32,
    fov_y_radians: f32,
    depth_range: DepthRange,
    near_width: f32,
    near_height: f32,
    far_width: f32,
    far_height: f32,
}

impl PerspectiveParameters {
    pub fn new(
        fov_y_radians: f32,
        ratio: f32,
        near_distance: f32,
        far_distance: f32,
        depth_range: DepthRange,
    ) -> Self {
        let tangent = f32::tan(fov_y_radians * 0.5);
        let near_height = 2. * near_distance * tangent;
        let far_height = 2. * far_distance * tangent;
        PerspectiveParameters {
            ratio,
            fov_y_radians,
            near_distance,
            far_distance,
            depth_range,
            near_height,
            far_height,
            near_width: near_height * ratio,
            far_width: far_height * ratio,
        }
    }

    pub fn fov_y_radians(&self) -> f32 {
        self.fov_y_radians
    }

    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    pub fn near_distance(&self) -> f32 {
        self.near_distance
    }

    pub fn far_distance(&self) -> f32 {
        self.far_distance
    }

    pub fn depth_range(&self) -> DepthRange {
        self.depth_range
    }
}

impl UpdateFrustum for PerspectiveParameters {
    fn update_frustum(
        &self,
        view_frustum: &ViewFrustum,
        frustum: &mut Frustum,
    ) {
        let eye_position = view_frustum.eye_position();

        let z = (eye_position - view_frustum.look_at()).normalize();
        let x = (view_frustum.up().cross(z)).normalize();
        let y = z.cross(x);

        let near_center = eye_position - z * self.near_distance;
        let far_center = eye_position - z * self.far_distance;

        while frustum.planes.len() < 6 {
            frustum.planes.push(Plane::default());
        }

        frustum.planes[ViewFrustum::NEAR] = Plane::new(-z, near_center);
        frustum.planes[ViewFrustum::FAR] = Plane::new(z, far_center);

        let half_near_h = self.near_height / 2.;
        let half_near_w = self.near_width / 2.;

        let mut point_on_plane = ((near_center + y * half_near_h) - eye_position).normalize();
        let mut normal = point_on_plane.cross(x);
        frustum.planes[ViewFrustum::TOP] = Plane::new(normal, near_center + y * half_near_h);

        point_on_plane = ((near_center - y * half_near_h) - eye_position).normalize();
        normal = x.cross(point_on_plane);
        frustum.planes[ViewFrustum::BOTTOM] = Plane::new(normal, near_center - y * half_near_h);

        point_on_plane = ((near_center - x * half_near_w) - eye_position).normalize();
        normal = point_on_plane.cross(y);
        frustum.planes[ViewFrustum::LEFT] = Plane::new(normal, near_center - x * half_near_w);

        point_on_plane = ((near_center + x * half_near_w) - eye_position).normalize();
        normal = y.cross(point_on_plane);
        frustum.planes[ViewFrustum::RIGHT] = Plane::new(normal, near_center + x * half_near_w);
    }
}
