use crate::render_features::RenderObjectHandle;
use crate::visibility::view_frustum_arc::{ViewFrustumArc, ViewFrustumRaii};
use crate::visibility::visibility_object_arc::{
    CullModel, VisibilityObjectArc, VisibilityObjectRaii, VisibilityObjectWeakArcInner,
};
use crate::visibility::ObjectId;
use crossbeam_channel::{Receiver, Sender};
use rafx_visibility::{ModelHandle, VisibilityWorld, ZoneHandle};
use slotmap::SlotMap;
use slotmap::{new_key_type, Key};

new_key_type! { pub struct VisibilityObjectId; }
new_key_type! { pub struct ViewFrustumObjectId; }

pub struct VisibilityObjectAllocator {
    visibility_objects: SlotMap<VisibilityObjectId, VisibilityObjectWeakArcInner>,
    visibility_world: VisibilityWorld,
    visibility_object_drop_tx: Sender<VisibilityObjectId>,
    visibility_object_drop_rx: Receiver<VisibilityObjectId>,
}

impl VisibilityObjectAllocator {
    pub(super) fn new(visibility_world: VisibilityWorld) -> Self {
        let (visibility_object_drop_tx, visibility_object_drop_rx) = crossbeam_channel::unbounded();

        VisibilityObjectAllocator {
            visibility_world,
            visibility_object_drop_tx,
            visibility_object_drop_rx,
            visibility_objects: Default::default(),
        }
    }

    pub(super) fn world(&self) -> &VisibilityWorld {
        &self.visibility_world
    }

    pub fn update(&mut self) {
        for id in self.visibility_object_drop_rx.try_iter() {
            self.visibility_objects.remove(id);
        }
        self.visibility_world.update();
    }

    pub fn try_destroy_model(
        &mut self,
        model: ModelHandle,
    ) -> bool {
        self.visibility_world.inner.destroy_model(model)
    }

    pub fn new_cull_model(
        &mut self,
        cull_model: CullModel,
    ) -> Option<ModelHandle> {
        let inner = &mut self.visibility_world.inner;
        match cull_model {
            CullModel::Mesh(polygons) => Some(inner.new_model(polygons)),
            CullModel::Sphere(radius) => Some(inner.new_bounding_sphere(radius)),
            CullModel::Quad(width, height) => Some(inner.new_quad(width, height)),
            CullModel::VisibleBounds(bounds) => Some(inner.new_visible_bounds(bounds)),
            CullModel::None => None,
        }
    }

    pub fn new_object(
        &mut self,
        object_id: ObjectId,
        cull_model: CullModel,
        render_objects: Vec<RenderObjectHandle>,
        zone: Option<ZoneHandle>,
    ) -> VisibilityObjectArc {
        let cull_model = self.new_cull_model(cull_model);
        let handle = self.visibility_world.inner.new_object();

        let object = VisibilityObjectArc::new(
            VisibilityObjectRaii::new(
                object_id,
                render_objects,
                handle.clone(),
                self.visibility_world.new_async_command_sender(),
            ),
            self.visibility_object_drop_tx.clone(),
        );
        let id = self.visibility_objects.insert(object.downgrade());
        object.set_visibility_object_id(id);

        let inner = &mut self.visibility_world.inner;
        inner.set_object_id(handle, id.data().as_ffi());
        inner.set_object_cull_model(handle, cull_model);
        inner.set_object_zone(handle, zone);
        object
    }

    pub fn object_ref(
        &self,
        id: VisibilityObjectId,
    ) -> Option<VisibilityObjectArc> {
        self.visibility_objects
            .get(id)
            .map(|x| x.upgrade())
            .flatten()
    }

    pub fn new_view_frustum(
        &mut self,
        static_zone: Option<ZoneHandle>,
        dynamic_zone: Option<ZoneHandle>,
    ) -> ViewFrustumArc {
        assert!(
            static_zone.is_some() || dynamic_zone.is_some(),
            "A ViewFrustumObject requires at least one defined Zone."
        );

        // We don't currently use the ID
        const UNUSED_ID: u64 = u64::MAX;

        let async_command_sender = self.visibility_world.new_async_command_sender();
        let inner = &mut self.visibility_world.inner;
        let static_view_frustum = static_zone.map(|region| {
            let handle = inner.new_view_frustum();
            inner.set_view_frustum_zone(handle, Some(region));
            inner.set_view_frustum_id(handle, UNUSED_ID);
            let static_object = ViewFrustumRaii::new(handle, async_command_sender);

            static_object
        });

        let async_command_sender = self.visibility_world.new_async_command_sender();
        let inner = &mut self.visibility_world.inner;
        let dynamic_view_frustum = dynamic_zone.map(|region| {
            let handle = inner.new_view_frustum();
            inner.set_view_frustum_zone(handle, Some(region));
            inner.set_view_frustum_id(handle, UNUSED_ID);
            let dynamic_object = ViewFrustumRaii::new(handle, async_command_sender);
            dynamic_object
        });

        ViewFrustumArc::new(static_view_frustum, dynamic_view_frustum)
    }
}
