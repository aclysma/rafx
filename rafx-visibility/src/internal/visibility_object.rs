use crate::geometry::{BoundingSphere, Transform};
use crate::{ModelHandle, ObjectHandle, ZoneHandle};

#[derive(Default, Copy, Clone)]
pub struct VisibilityObject {
    pub handle: ObjectHandle,
    pub id: u64,
    pub zone: Option<ZoneHandle>,
    pub cull_model: Option<ModelHandle>,
    pub transform: Transform,
}

impl VisibilityObject {
    pub fn new(
        id: u64,
        handle: ObjectHandle,
    ) -> Self {
        VisibilityObject {
            id,
            handle,
            ..Default::default()
        }
    }

    pub fn default_bounding_sphere(transform: Transform) -> BoundingSphere {
        // NOTE(dvd): Default size chosen to fit a 1x1 quad.
        BoundingSphere::new(transform.translation, 1.42 * transform.scale.max_element())
    }
}
