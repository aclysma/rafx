use crate::nodes::GenericRenderNodeHandle;
use crate::visibility::visibility_object_allocator::{SlotMapArc, VisibilityObjectId};
use crate::visibility::EntityId;
use crossbeam_channel::Sender;
use glam::{Quat, Vec3};
use rafx_visibility::geometry::Transform;
use rafx_visibility::{
    AsyncCommand, ModelHandle, ObjectHandle, PolygonSoup, VisibleBounds, ZoneHandle,
};
use std::sync::Arc;

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

#[derive(Clone)]
pub struct VisibilityObjectArc {
    inner: Arc<RemoveObjectWhenDropped>,
}

impl VisibilityObjectArc {
    pub(crate) fn new(
        id: VisibilityObjectId,
        storage: SlotMapArc<VisibilityObjectId, VisibilityObject>,
    ) -> Self {
        Self {
            inner: Arc::new(RemoveObjectWhenDropped { id, storage }),
        }
    }

    #[allow(dead_code)]
    pub(super) fn set_zone(
        &self,
        zone: Option<ZoneHandle>,
    ) -> &Self {
        let storage = self.inner.storage.read();
        let object = storage.get(self.inner.id).unwrap();
        object.set_zone(zone);
        self
    }

    pub fn add_feature(
        &self,
        feature: GenericRenderNodeHandle,
    ) -> &Self {
        let mut storage = self.inner.storage.write();
        let object = storage.get_mut(self.inner.id).unwrap();
        object.add_feature(feature);
        self
    }

    pub fn remove_feature(
        &self,
        feature: GenericRenderNodeHandle,
    ) -> &Self {
        let mut storage = self.inner.storage.write();
        let object = storage.get_mut(self.inner.id).unwrap();
        object.remove_feature(feature);
        self
    }

    pub fn set_cull_model(
        &self,
        cull_model: Option<ModelHandle>,
    ) -> &Self {
        let storage = self.inner.storage.read();
        let object = storage.get(self.inner.id).unwrap();
        object.set_cull_model(cull_model);
        self
    }

    pub fn set_transform(
        &self,
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) -> &Self {
        let storage = self.inner.storage.read();
        let object = storage.get(self.inner.id).unwrap();
        object.set_transform(translation, rotation, scale);
        self
    }
}

struct RemoveObjectWhenDropped {
    id: VisibilityObjectId,
    storage: SlotMapArc<VisibilityObjectId, VisibilityObject>,
}

impl Drop for RemoveObjectWhenDropped {
    fn drop(&mut self) {
        // NOTE(dvd): When this inner handle is dropped, we'll remove the key
        // from the storage. That will then destroy the object created in
        // in the visibility world.
        let mut storage = self.storage.write();
        storage.remove(self.id).unwrap();
    }
}

pub struct VisibilityObject {
    commands: Sender<AsyncCommand>,
    handle: ObjectHandle,
    features: Vec<GenericRenderNodeHandle>, // TODO(dvd): This might be better as a SmallVec.
    entity_id: EntityId,
}

impl VisibilityObject {
    pub fn new(
        entity_id: EntityId,
        handle: ObjectHandle,
        commands: Sender<AsyncCommand>,
    ) -> Self {
        Self {
            commands,
            handle,
            entity_id,
            features: Default::default(),
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

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn features(&self) -> &[GenericRenderNodeHandle] {
        &self.features
    }

    pub fn add_feature(
        &mut self,
        feature: GenericRenderNodeHandle,
    ) -> &Self {
        if !self.features.contains(&feature) {
            self.features.push(feature);
        }
        self
    }

    pub fn remove_feature(
        &mut self,
        feature: GenericRenderNodeHandle,
    ) -> &Self {
        if let Some(index) = self.features.iter().position(|value| *value == feature) {
            self.features.swap_remove(index);
        }
        self
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
            .send(AsyncCommand::SetObjectPosition(
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

impl Drop for VisibilityObject {
    fn drop(&mut self) {
        // NOTE(dvd): Destroy the associated Object in the visibility world.
        let _ = self.commands.send(AsyncCommand::DestroyObject(self.handle));
    }
}
