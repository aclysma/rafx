use crate::visibility::view_frustum_arc::ViewFrustumArc;
use crate::visibility::visibility_object_allocator::{
    VisibilityObjectAllocator, VisibilityObjectId, VisibilityObjectRef,
};
use crate::visibility::visibility_object_arc::{CullModel, VisibilityObjectArc};
use crate::visibility::EntityId;
use crossbeam_channel::Sender;
use rafx_visibility::{AsyncCommand, VisibilityWorldArc, ZoneHandle};
use std::sync::Arc;

pub struct VisibilityRegion {
    allocator: Arc<VisibilityObjectAllocator>,
    commands: Sender<AsyncCommand>,
    dynamic_zone: ZoneHandle,
    static_zone: ZoneHandle,
}

impl VisibilityRegion {
    pub fn new() -> Self {
        let visibility_world = VisibilityWorldArc::new();
        let allocator = Arc::new(VisibilityObjectAllocator::new(visibility_world.clone()));

        let mut inner = visibility_world.inner.lock();
        let static_zone = inner.new_zone();
        let dynamic_zone = inner.new_zone();

        VisibilityRegion {
            commands: visibility_world.new_async_command_sender(),
            static_zone,
            dynamic_zone,
            allocator,
        }
    }

    pub fn register_view_frustum(&self) -> ViewFrustumArc {
        self.allocator
            .new_view_frustum(Some(self.static_zone), Some(self.dynamic_zone))
    }

    pub fn register_static_view_frustum(&self) -> ViewFrustumArc {
        self.allocator
            .new_view_frustum(Some(self.static_zone), None)
    }

    pub fn register_dynamic_view_frustum(&self) -> ViewFrustumArc {
        self.allocator
            .new_view_frustum(None, Some(self.dynamic_zone))
    }

    /// Returns a smart pointer to a handle representing a static object.
    /// A static object is a hint to the visibility world that the object's transform changes rarely.
    /// Most geometry in the world is static -- buildings, trees, rocks, grass, and so on.
    pub fn register_static_object(
        &self,
        entity_id: EntityId,
        cull_model: CullModel,
    ) -> VisibilityObjectArc {
        self.register_object(entity_id, cull_model, self.static_zone)
    }

    /// Returns a smart pointer to a handle representing a dynamic object.
    /// A dynamic object is a hint to the visibility world that the object's transform changes often.
    /// Characters, projectiles, vehicles, and moving platforms are examples of dynamic geometry.
    pub fn register_dynamic_object(
        &self,
        entity_id: EntityId,
        cull_model: CullModel,
    ) -> VisibilityObjectArc {
        self.register_object(entity_id, cull_model, self.dynamic_zone)
    }

    fn register_object(
        &self,
        entity_id: EntityId,
        cull_model: CullModel,
        zone: ZoneHandle,
    ) -> VisibilityObjectArc {
        self.allocator.new_object(entity_id, cull_model, Some(zone))
    }

    pub fn object_ref(
        &self,
        id: VisibilityObjectId,
    ) -> VisibilityObjectRef {
        self.allocator.object_ref(id)
    }
}

impl Drop for VisibilityRegion {
    fn drop(&mut self) {
        let _ = self
            .commands
            .send(AsyncCommand::DestroyZone(self.static_zone));
        let _ = self
            .commands
            .send(AsyncCommand::DestroyZone(self.dynamic_zone));
    }
}
