use crate::render_features::RenderObjectHandle;
use crate::visibility::visibility_object_allocator::VisibilityObjectId;
use crate::visibility::ObjectId;
use crossbeam_channel::Sender;
use glam::{Quat, Vec3};
use rafx_visibility::geometry::Transform;
use rafx_visibility::{
    AsyncCommand, ModelHandle, PolygonSoup, VisibilityObjectHandle, VisibleBounds, ZoneHandle,
};
use slotmap::Key;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

pub enum CullModel {
    Mesh(PolygonSoup),
    VisibleBounds(VisibleBounds),
    Sphere(f32),
    Quad(f32, f32),
    None,
}

impl CullModel {
    pub fn mesh(polygon_soup: PolygonSoup) -> CullModel {
        CullModel::Mesh(polygon_soup)
    }

    pub fn visible_bounds(model: VisibleBounds) -> CullModel {
        CullModel::VisibleBounds(model)
    }

    pub fn sphere(radius: f32) -> CullModel {
        CullModel::Sphere(radius)
    }

    pub fn quad(
        width: f32,
        height: f32,
    ) -> CullModel {
        CullModel::Quad(width, height)
    }

    pub fn none() -> CullModel {
        CullModel::None
    }
}

pub struct VisibilityObjectArcInner {
    object: VisibilityObjectRaii,
    // This corresponds to the key in VisibilityObjectAllocator::visibility_objects. It's set after
    // we insert VisibilityObjectArc into the slot map
    visibility_object_id: AtomicU64,
    drop_tx: Sender<VisibilityObjectId>,
}

impl Drop for VisibilityObjectArcInner {
    fn drop(&mut self) {
        let _ = self
            .drop_tx
            .send(VisibilityObjectId::from(slotmap::KeyData::from_ffi(
                self.visibility_object_id.load(Ordering::Relaxed),
            )));
    }
}

pub struct VisibilityObjectWeakArcInner {
    inner: Weak<VisibilityObjectArcInner>,
}

impl VisibilityObjectWeakArcInner {
    pub fn upgrade(&self) -> Option<VisibilityObjectArc> {
        self.inner
            .upgrade()
            .map(|inner| VisibilityObjectArc { inner })
    }
}

#[derive(Clone)]
pub struct VisibilityObjectArc {
    inner: Arc<VisibilityObjectArcInner>,
}

impl VisibilityObjectArc {
    pub(crate) fn new(
        object: VisibilityObjectRaii,
        drop_tx: Sender<VisibilityObjectId>,
    ) -> Self {
        Self {
            inner: Arc::new(VisibilityObjectArcInner {
                object,
                visibility_object_id: AtomicU64::default(),
                drop_tx,
            }),
        }
    }

    pub fn downgrade(&self) -> VisibilityObjectWeakArcInner {
        VisibilityObjectWeakArcInner {
            inner: Arc::downgrade(&self.inner),
        }
    }

    pub(super) fn set_visibility_object_id(
        &self,
        visibility_object_id: VisibilityObjectId,
    ) {
        self.inner
            .visibility_object_id
            .store(visibility_object_id.data().as_ffi(), Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub(super) fn set_zone(
        &self,
        zone: Option<ZoneHandle>,
    ) -> &Self {
        self.inner.object.set_zone(zone);
        self
    }

    pub fn object_id(&self) -> ObjectId {
        self.inner.object.object_id()
    }

    // This is
    pub fn visibility_object_handle(&self) -> VisibilityObjectHandle {
        self.inner.object.handle
    }

    pub fn render_objects(&self) -> &[RenderObjectHandle] {
        &self.inner.object.render_objects()
    }

    pub fn set_cull_model(
        &self,
        cull_model: Option<ModelHandle>,
    ) -> &Self {
        self.inner.object.set_cull_model(cull_model);
        self
    }

    pub fn set_transform(
        &self,
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) -> &Self {
        self.inner
            .object
            .set_transform(translation, rotation, scale);
        self
    }
}

// An RAII object for a visibility VisibilityObjectHandle
pub struct VisibilityObjectRaii {
    commands: Sender<AsyncCommand>,
    handle: VisibilityObjectHandle,
    object_id: ObjectId,
    render_objects: Vec<RenderObjectHandle>, // TODO(dvd): This might be better as a SmallVec.
}

impl Drop for VisibilityObjectRaii {
    fn drop(&mut self) {
        // NOTE(dvd): Destroy the associated Object in the visibility world.
        let _ = self.commands.send(AsyncCommand::DestroyObject(self.handle));
    }
}

impl VisibilityObjectRaii {
    pub fn new(
        object_id: ObjectId,
        render_objects: Vec<RenderObjectHandle>,
        handle: VisibilityObjectHandle,
        commands: Sender<AsyncCommand>,
    ) -> Self {
        Self {
            commands,
            handle,
            object_id,
            render_objects,
        }
    }

    #[allow(dead_code)]
    pub(super) fn set_zone(
        &self,
        zone: Option<ZoneHandle>,
    ) -> &Self {
        self.commands
            .send(AsyncCommand::SetObjectZone(self.handle, zone))
            .expect("Unable to send SetObjectZone command.");
        self
    }

    pub fn object_id(&self) -> ObjectId {
        self.object_id
    }

    pub fn render_objects(&self) -> &[RenderObjectHandle] {
        &self.render_objects
    }

    pub fn set_cull_model(
        &self,
        cull_model: Option<ModelHandle>,
    ) -> &Self {
        self.commands
            .send(AsyncCommand::SetObjectCullModel(self.handle, cull_model))
            .expect("Unable to send SetObjectCullModel command.");
        self
    }

    pub fn set_transform(
        &self,
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) -> &Self {
        self.commands
            .send(AsyncCommand::SetObjectTransform(
                self.handle,
                Transform {
                    translation,
                    rotation,
                    scale,
                },
            ))
            .expect("Unable to send SetObjectPosition command.");
        self
    }
}
