use crate::geometry::{BoundingSphere, Transform};
use crate::{ModelHandle, VisibilityObjectHandle, ZoneHandle};

#[derive(Default, Clone)]
pub struct VisibilityObject {
    // The handle given to this object by the visibility system
    pub handle: VisibilityObjectHandle,
    // The opaque object ID (i.e. a pointer or ECS ID)
    pub id: u64,
    pub zone: Option<ZoneHandle>,
    pub cull_model: Option<ModelHandle>,
    pub transform: Option<Transform>,
    // This is updated before processing commands in VisibilityWorld::update
    pub previous_frame_transform: Option<Transform>,
}

impl VisibilityObject {
    pub fn new(
        id: u64,
        handle: VisibilityObjectHandle,
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
