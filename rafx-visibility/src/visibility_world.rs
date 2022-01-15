use crate::frustum_culling::collect_visible_objects;
use crate::geometry::{BoundingSphere, Transform};
use crate::internal::VisibilityWorldInternal;
use crate::Projection;
use crossbeam_channel::{unbounded, Receiver, Sender};
use glam::Vec3;
use slotmap::new_key_type;
use std::hash::Hash;

pub type VisibleObjects = Vec<VisibilityResult<VisibilityObjectHandle>>;
pub type VisibleVolumes = Vec<VisibilityResult<VolumeHandle>>;

// Private keys.

new_key_type! { struct ChunkHandle; }

// Public keys.

new_key_type! { pub struct ModelHandle; }
new_key_type! { pub struct ZoneHandle; }
new_key_type! { pub struct VisibilityObjectHandle; }
new_key_type! { pub struct ViewFrustumHandle; }
new_key_type! { pub struct VolumeHandle; }

pub enum AsyncCommand {
    SetObjectTransform(VisibilityObjectHandle, Transform),
    SetObjectZone(VisibilityObjectHandle, Option<ZoneHandle>),
    SetObjectId(VisibilityObjectHandle, u64),
    SetObjectCullModel(VisibilityObjectHandle, Option<ModelHandle>),
    SetViewFrustumZone(ViewFrustumHandle, Option<ZoneHandle>),
    SetViewFrustumTransforms(ViewFrustumHandle, Vec3, Vec3, Vec3),
    SetViewFrustumId(ViewFrustumHandle, u64),
    SetViewFrustumProjection(ViewFrustumHandle, Projection),
    DestroyViewFrustum(ViewFrustumHandle),
    DestroyZone(ZoneHandle),
    DestroyObject(VisibilityObjectHandle),
    DestroyModel(ModelHandle),
    QueuedCommands(Vec<AsyncCommand>),
}

#[derive(Copy, Clone, Default)]
pub struct VisibilityResult<T> {
    pub handle: T,
    pub id: u64,
    //pub bounding_sphere: BoundingSphere,
    pub distance_from_view_frustum: f32,
}

impl<T> VisibilityResult<T> {
    pub fn new(
        handle: T,
        id: u64,
        view_frustum_position: Vec3,
        bounding_sphere: BoundingSphere,
    ) -> Self {
        VisibilityResult {
            handle,
            id,
            //bounding_sphere,
            distance_from_view_frustum: view_frustum_position.distance(bounding_sphere.position),
        }
    }
}

#[derive(Default)]
pub struct VisibilityQuery {
    pub objects: VisibleObjects,
    pub volumes: VisibleVolumes,
}

pub struct VisibilityWorld {
    pub inner: VisibilityWorldInternal,
    sender: Sender<AsyncCommand>,
    receiver: Receiver<AsyncCommand>,
}

pub enum QueryError {
    NoViewFrustumZone,
}

impl VisibilityWorld {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        VisibilityWorld {
            inner: VisibilityWorldInternal::new(),
            receiver,
            sender,
        }
    }

    #[profiling::function]
    pub fn update(&mut self) {
        let mut inner = &mut self.inner;

        for (_, object) in &mut inner.objects {
            object.previous_frame_transform = object.transform;
        }

        {
            profiling::scope!("receiver.try_iter");
            for command in self.receiver.try_iter() {
                VisibilityWorld::handle_command(&mut inner, command);
            }
        }
    }

    pub fn new_async_command_sender(&self) -> Sender<AsyncCommand> {
        self.sender.clone()
    }

    fn handle_command(
        inner: &mut VisibilityWorldInternal,
        command: AsyncCommand,
    ) {
        match command {
            AsyncCommand::SetObjectTransform(object, transform) => {
                inner.set_object_transform(object, transform);
            }
            AsyncCommand::SetObjectZone(object, zone) => {
                inner.set_object_zone(object, zone);
            }
            AsyncCommand::SetObjectId(object, id) => {
                inner.set_object_id(object, id);
            }
            AsyncCommand::SetObjectCullModel(object, cull_model) => {
                inner.set_object_cull_model(object, cull_model);
            }
            AsyncCommand::SetViewFrustumZone(view_frustum, zone) => {
                inner.set_view_frustum_zone(view_frustum, zone);
            }
            AsyncCommand::SetViewFrustumTransforms(view_frustum, eye, look_at, up) => {
                inner.set_view_frustum_transforms(view_frustum, eye, look_at, up);
            }
            AsyncCommand::SetViewFrustumId(view_frustum, id) => {
                inner.set_view_frustum_id(view_frustum, id);
            }
            AsyncCommand::SetViewFrustumProjection(view_frustum, projection) => match projection {
                Projection::Perspective(parameters) => {
                    inner.set_view_frustum_perspective(
                        view_frustum,
                        parameters.fov_y_radians(),
                        parameters.ratio(),
                        parameters.near_distance(),
                        parameters.far_distance(),
                        parameters.depth_range(),
                    );
                }
                Projection::Orthographic(parameters) => {
                    inner.set_view_frustum_orthographic(
                        view_frustum,
                        parameters.left(),
                        parameters.right(),
                        parameters.bottom(),
                        parameters.top(),
                        parameters.near_distance(),
                        parameters.far_distance(),
                        parameters.depth_range(),
                    );
                }
                Projection::Undefined => {
                    panic!("Cannot send `Undefined` projection on View Frustum.");
                }
            },
            AsyncCommand::DestroyViewFrustum(view_frustum) => {
                inner.destroy_view_frustum(view_frustum);
            }
            AsyncCommand::DestroyZone(zone) => {
                inner.destroy_zone(zone);
            }
            AsyncCommand::DestroyObject(object) => {
                inner.destroy_object(object);
            }
            AsyncCommand::DestroyModel(model) => {
                inner.destroy_model(model);
            }
            AsyncCommand::QueuedCommands(commands) => {
                for inner_command in commands {
                    VisibilityWorld::handle_command(inner, inner_command);
                }
            }
        }
    }

    /// Queries visibility for a `ViewFrustum`. The `result` is a `VisibilityQuery`. This function is thread-safe.
    #[profiling::function]
    pub fn query_visibility(
        &self,
        view_frustum: ViewFrustumHandle,
        result: &mut VisibilityQuery,
    ) -> Result<(), QueryError> {
        let zone = {
            let view_frustum_zone = self.inner.view_frustum_zones.get(view_frustum);
            if let Some(zone) = view_frustum_zone {
                Ok(*zone)
            } else {
                return Err(QueryError::NoViewFrustumZone);
            }
        }?;

        let active_view_frustum = self.inner.view_frustums.get(view_frustum).unwrap();
        let view_frustum_position = active_view_frustum.eye_position();
        let frustum = active_view_frustum.acquire_frustum().clone();

        let chunks = self.inner.zones.get(zone).unwrap().chunks.clone();
        for chunk in &chunks {
            collect_visible_objects(chunk, view_frustum_position, &frustum, &mut result.objects)
        }

        Ok(())
    }

    /// Queries shadow casters for a `ViewFrustum` representing a light. The `result` is a `VisibilityQuery`.
    /// The objects in `result` are able to cast shadows into at least one of the `shadowed` frustums.
    /// This function is thread-safe.
    pub fn query_shadow_casters(
        &self,
        _light: ViewFrustumHandle,
        _shadowed: &[ViewFrustumHandle],
        _result: &mut VisibilityQuery,
    ) {
        unimplemented!();
    }
}
