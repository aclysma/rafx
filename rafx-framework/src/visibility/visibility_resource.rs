use crate::render_features::RenderObjectHandle;
use crate::visibility::view_frustum_arc::ViewFrustumArc;
use crate::visibility::visibility_object_allocator::{
    VisibilityObjectAllocator, VisibilityObjectId,
};
use crate::visibility::visibility_object_arc::{CullModel, VisibilityObjectArc};
use crate::visibility::ObjectId;
use crossbeam_channel::Sender;
use rafx_visibility::geometry::Transform;
use rafx_visibility::{AsyncCommand, ModelHandle, VisibilityObject, VisibilityWorld, ZoneHandle};

pub struct VisibilityObjectInfo<'a> {
    arc: VisibilityObjectArc,
    obj: &'a VisibilityObject,
}

impl<'a> VisibilityObjectInfo<'a> {
    pub fn object_id(&self) -> ObjectId {
        self.arc.object_id()
    }

    pub fn render_objects(&self) -> &[RenderObjectHandle] {
        self.arc.render_objects()
    }

    pub fn transform(&self) -> Transform {
        self.obj.transform.unwrap_or_default()
    }

    pub fn previous_frame_transform(&self) -> Option<Transform> {
        self.obj.previous_frame_transform
    }

    pub fn model_handle(&self) -> &Option<ModelHandle> {
        &self.obj.cull_model
    }
}

pub struct VisibilityResource {
    allocator: VisibilityObjectAllocator,
    commands: Sender<AsyncCommand>,
    dynamic_zone: ZoneHandle,
    static_zone: ZoneHandle,
}

impl VisibilityResource {
    pub fn new() -> Self {
        let mut visibility_world = VisibilityWorld::new();
        let static_zone = visibility_world.inner.new_zone();
        let dynamic_zone = visibility_world.inner.new_zone();
        let commands = visibility_world.new_async_command_sender();
        let allocator = VisibilityObjectAllocator::new(visibility_world);

        VisibilityResource {
            commands,
            static_zone,
            dynamic_zone,
            allocator,
        }
    }

    pub fn world(&self) -> &VisibilityWorld {
        self.allocator.world()
    }

    pub fn update(&mut self) {
        self.allocator.update();
    }

    pub fn register_view_frustum(&mut self) -> ViewFrustumArc {
        self.allocator
            .new_view_frustum(Some(self.static_zone), Some(self.dynamic_zone))
    }

    pub fn register_static_view_frustum(&mut self) -> ViewFrustumArc {
        self.allocator
            .new_view_frustum(Some(self.static_zone), None)
    }

    pub fn register_dynamic_view_frustum(&mut self) -> ViewFrustumArc {
        self.allocator
            .new_view_frustum(None, Some(self.dynamic_zone))
    }

    /// Returns a smart pointer to a handle representing a static object.
    /// A static object is a hint to the visibility world that the object's transform changes rarely.
    /// Most geometry in the world is static -- buildings, trees, rocks, grass, and so on.
    pub fn register_static_object(
        &mut self,
        object_id: ObjectId,
        cull_model: CullModel,
        render_objects: Vec<RenderObjectHandle>,
    ) -> VisibilityObjectArc {
        self.register_object(object_id, cull_model, render_objects, self.static_zone)
    }

    /// Returns a smart pointer to a handle representing a dynamic object.
    /// A dynamic object is a hint to the visibility world that the object's transform changes often.
    /// Characters, projectiles, vehicles, and moving platforms are examples of dynamic geometry.
    pub fn register_dynamic_object(
        &mut self,
        object_id: ObjectId,
        cull_model: CullModel,
        render_objects: Vec<RenderObjectHandle>,
    ) -> VisibilityObjectArc {
        self.register_object(object_id, cull_model, render_objects, self.dynamic_zone)
    }

    fn register_object(
        &mut self,
        object_id: ObjectId,
        cull_model: CullModel,
        render_objects: Vec<RenderObjectHandle>,
        zone: ZoneHandle,
    ) -> VisibilityObjectArc {
        self.allocator
            .new_object(object_id, cull_model, render_objects, Some(zone))
    }

    pub fn visibility_object_arc(
        &self,
        id: VisibilityObjectId,
    ) -> Option<VisibilityObjectArc> {
        self.allocator.object_ref(id)
    }

    pub fn visibility_object_info(
        &self,
        id: VisibilityObjectId,
    ) -> Option<VisibilityObjectInfo> {
        let arc = self.allocator.object_ref(id);
        if let Some(arc) = arc {
            let obj = self
                .world()
                .inner
                .visibility_object(arc.visibility_object_handle());
            if let Some(obj) = obj {
                return Some(VisibilityObjectInfo { arc, obj });
            }
        }

        None
    }
}

impl Drop for VisibilityResource {
    fn drop(&mut self) {
        let _ = self
            .commands
            .send(AsyncCommand::DestroyZone(self.static_zone));
        let _ = self
            .commands
            .send(AsyncCommand::DestroyZone(self.dynamic_zone));
    }
}
