use crate::visibility::view_frustum_arc::{ViewFrustumArc, ViewFrustumObject};
use crate::visibility::visibility_object_arc::{CullModel, VisibilityObject, VisibilityObjectArc};
use crate::visibility::ObjectId;
use parking_lot::{RwLock, RwLockReadGuard};
use rafx_visibility::{ModelHandle, VisibilityWorldArc, ZoneHandle};
use slotmap::SlotMap;
use slotmap::{new_key_type, Key};
use std::ops::Deref;
use std::sync::Arc;

new_key_type! { pub struct VisibilityObjectId; }
new_key_type! { pub struct ViewFrustumObjectId; }

pub(super) type SlotMapArc<K, V> = Arc<RwLock<SlotMap<K, V>>>;

pub struct VisibilityObjectRef<'a>(
    RwLockReadGuard<'a, SlotMap<VisibilityObjectId, VisibilityObject>>,
    VisibilityObjectId,
);

impl<'a> Deref for VisibilityObjectRef<'a> {
    type Target = VisibilityObject;

    fn deref(&self) -> &Self::Target {
        self.0.get(self.1).unwrap()
    }
}

pub struct VisibilityObjectLookup<'a>(
    RwLockReadGuard<'a, SlotMap<VisibilityObjectId, VisibilityObject>>,
);

impl<'a> VisibilityObjectLookup<'a> {
    pub fn object_ref(
        &self,
        id: VisibilityObjectId,
    ) -> &VisibilityObject {
        self.0.get(id).unwrap()
    }
}

#[derive(Clone)]
pub struct VisibilityObjectAllocator {
    view_frustums: SlotMapArc<ViewFrustumObjectId, ViewFrustumObject>,
    objects: SlotMapArc<VisibilityObjectId, VisibilityObject>,
    visibility_world: VisibilityWorldArc,
}

impl VisibilityObjectAllocator {
    pub(super) fn new(visibility_world: VisibilityWorldArc) -> Self {
        VisibilityObjectAllocator {
            visibility_world,
            view_frustums: Default::default(),
            objects: Default::default(),
        }
    }

    pub fn try_destroy_model(
        &self,
        model: ModelHandle,
    ) -> bool {
        let mut inner = self.visibility_world.inner.lock();
        inner.destroy_model(model)
    }

    pub fn new_cull_model(
        &self,
        cull_model: CullModel,
    ) -> Option<ModelHandle> {
        let mut inner = self.visibility_world.inner.lock();
        match cull_model {
            CullModel::Mesh(polygons) => Some(inner.new_model(polygons)),
            CullModel::Sphere(radius) => Some(inner.new_bounding_sphere(radius)),
            CullModel::Quad(width, height) => Some(inner.new_quad(width, height)),
            CullModel::VisibleBounds(bounds) => Some(inner.new_visible_bounds(bounds)),
            CullModel::None => None,
        }
    }

    pub fn new_object(
        &self,
        object_id: ObjectId,
        cull_model: CullModel,
        zone: Option<ZoneHandle>,
    ) -> VisibilityObjectArc {
        let cull_model = self.new_cull_model(cull_model);

        let mut inner = self.visibility_world.inner.lock();

        let handle = inner.new_object();

        let id = self.objects.write().insert(VisibilityObject::new(
            object_id,
            handle.clone(),
            self.visibility_world.new_async_command_sender(),
        ));

        inner.set_object_id(handle, id.data().as_ffi());
        inner.set_object_cull_model(handle, cull_model);
        inner.set_object_zone(handle, zone);

        VisibilityObjectArc::new(id, self.objects.clone())
    }

    pub fn object_lookup(&self) -> VisibilityObjectLookup {
        let guard = self.objects.read();
        VisibilityObjectLookup(guard)
    }

    pub fn object_ref(
        &self,
        id: VisibilityObjectId,
    ) -> VisibilityObjectRef {
        let guard = self.objects.read();
        VisibilityObjectRef(guard, id)
    }

    pub fn new_view_frustum(
        &self,
        static_zone: Option<ZoneHandle>,
        dynamic_zone: Option<ZoneHandle>,
    ) -> ViewFrustumArc {
        assert!(
            static_zone.is_some() || dynamic_zone.is_some(),
            "A ViewFrustumObject requires at least one defined Zone."
        );

        let mut inner = self.visibility_world.inner.lock();
        let mut view_frustums = self.view_frustums.write();

        let static_view_frustum = static_zone.map(|region| {
            let handle = inner.new_view_frustum();
            inner.set_view_frustum_zone(handle, Some(region));
            let id = view_frustums.insert(ViewFrustumObject::new(
                handle,
                self.visibility_world.new_async_command_sender(),
                self.visibility_world.clone(),
            ));
            inner.set_view_frustum_id(handle, id.data().as_ffi());
            id
        });

        let dynamic_view_frustum = dynamic_zone.map(|region| {
            let handle = inner.new_view_frustum();
            inner.set_view_frustum_zone(handle, Some(region));
            let id = view_frustums.insert(ViewFrustumObject::new(
                handle,
                self.visibility_world.new_async_command_sender(),
                self.visibility_world.clone(),
            ));
            inner.set_view_frustum_id(handle, id.data().as_ffi());
            id
        });

        ViewFrustumArc::new(
            static_view_frustum,
            dynamic_view_frustum,
            self.view_frustums.clone(),
            self.visibility_world.clone(),
        )
    }
}
