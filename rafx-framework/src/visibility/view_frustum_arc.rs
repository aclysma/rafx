use crate::visibility::visibility_object_allocator::{SlotMapArc, ViewFrustumObjectId};
use crate::visibility::VisibilityConfig;
use crate::RafxResult;
use crossbeam_channel::Sender;
use glam::Vec3;
use parking_lot::{RwLock, RwLockReadGuard};
use rafx_api::RafxError;
use rafx_visibility::{
    AsyncCommand, Projection, ViewFrustumHandle, VisibilityQuery, VisibilityWorldArc, ZoneHandle,
};
use slotmap::SlotMap;
use std::sync::Arc;

pub type ViewFrustumId = (Option<ViewFrustumObjectId>, Option<ViewFrustumObjectId>);

#[derive(Clone)]
pub struct ViewFrustumArc {
    inner: Arc<RemoveViewFrustumWhenDropped>,
}

impl ViewFrustumArc {
    pub fn new(
        static_view_frustum: Option<ViewFrustumObjectId>,
        dynamic_view_frustum: Option<ViewFrustumObjectId>,
        storage: SlotMapArc<ViewFrustumObjectId, ViewFrustumObject>,
        visibility_world: VisibilityWorldArc,
    ) -> Self {
        Self {
            inner: Arc::new(RemoveViewFrustumWhenDropped {
                static_view_frustum,
                dynamic_view_frustum,
                storage,
                visibility_world,
                visibility_query: Default::default(),
            }),
        }
    }

    pub fn view_frustum_id(&self) -> ViewFrustumId {
        (
            self.inner.static_view_frustum,
            self.inner.dynamic_view_frustum,
        )
    }

    #[allow(dead_code)]
    pub(super) fn set_zone(
        &self,
        static_zone: Option<ZoneHandle>,
        dynamic_zone: Option<ZoneHandle>,
    ) -> &Self {
        let storage = self.inner.storage.read();

        if let Some(static_view_frustum) =
            self.view_frustum(&self.inner.static_view_frustum, &storage)
        {
            static_view_frustum.set_zone(static_zone);
        }

        if let Some(dynamic_view_frustum) =
            self.view_frustum(&self.inner.dynamic_view_frustum, &storage)
        {
            dynamic_view_frustum.set_zone(dynamic_zone);
        }

        self
    }

    pub fn set_transform(
        &self,
        eye: Vec3,
        look_at: Vec3,
        up: Vec3,
    ) -> &Self {
        let storage = self.inner.storage.read();

        if let Some(static_view_frustum) =
            self.view_frustum(&self.inner.static_view_frustum, &storage)
        {
            static_view_frustum.set_transform(eye, look_at, up);
        }

        if let Some(dynamic_view_frustum) =
            self.view_frustum(&self.inner.dynamic_view_frustum, &storage)
        {
            dynamic_view_frustum.set_transform(eye, look_at, up);
        }

        self
    }

    pub fn set_projection(
        &self,
        projection: &Projection,
    ) -> &Self {
        let storage = self.inner.storage.read();

        if let Some(static_view_frustum) =
            self.view_frustum(&self.inner.static_view_frustum, &storage)
        {
            static_view_frustum.set_projection(projection);
        }

        if let Some(dynamic_view_frustum) =
            self.view_frustum(&self.inner.dynamic_view_frustum, &storage)
        {
            dynamic_view_frustum.set_projection(projection);
        }

        self
    }

    pub fn query_visibility(
        &mut self,
        visibility_config: &VisibilityConfig,
    ) -> RafxResult<RwLockReadGuard<VisibilityQuery>> {
        self.inner.visibility_world.update();

        if visibility_config.enable_visibility_update {
            let mut results = self.inner.visibility_query.write();

            results.objects.clear();
            results.volumes.clear();

            let storage = self.inner.storage.read();

            if let Some(static_view_frustum) =
                self.view_frustum(&self.inner.static_view_frustum, &storage)
            {
                static_view_frustum.query_visibility(&mut results)?;
            }

            if let Some(dynamic_view_frustum) =
                self.view_frustum(&self.inner.dynamic_view_frustum, &storage)
            {
                dynamic_view_frustum.query_visibility(&mut results)?;
            }
        }

        Ok(self.inner.visibility_query.read())
    }

    fn view_frustum<'a>(
        &self,
        view_frustum: &Option<ViewFrustumObjectId>,
        storage: &'a SlotMap<ViewFrustumObjectId, ViewFrustumObject>,
    ) -> Option<&'a ViewFrustumObject> {
        view_frustum.as_ref().map(|key| storage.get(*key).unwrap())
    }
}

struct RemoveViewFrustumWhenDropped {
    static_view_frustum: Option<ViewFrustumObjectId>,
    dynamic_view_frustum: Option<ViewFrustumObjectId>,
    storage: SlotMapArc<ViewFrustumObjectId, ViewFrustumObject>,
    visibility_world: VisibilityWorldArc,
    visibility_query: RwLock<VisibilityQuery>,
}

impl Drop for RemoveViewFrustumWhenDropped {
    fn drop(&mut self) {
        // NOTE(dvd): When this inner handle is dropped, we'll remove the key
        // from the storage. That will then destroy the object created in
        // in the visibility world.
        let mut storage = self.storage.write();
        if let Some(id) = self.static_view_frustum {
            storage.remove(id).unwrap();
        }
        if let Some(id) = self.dynamic_view_frustum {
            storage.remove(id).unwrap();
        }
    }
}

pub struct ViewFrustumObject {
    commands: Sender<AsyncCommand>,
    visibility_world: VisibilityWorldArc,
    handle: ViewFrustumHandle,
}

impl ViewFrustumObject {
    pub fn new(
        handle: ViewFrustumHandle,
        commands: Sender<AsyncCommand>,
        visibility_world: VisibilityWorldArc,
    ) -> Self {
        Self {
            handle,
            commands,
            visibility_world,
        }
    }

    #[allow(dead_code)]
    pub(super) fn set_zone(
        &self,
        zone: Option<ZoneHandle>,
    ) -> &Self {
        self.commands
            .send(AsyncCommand::SetViewFrustumZone(self.handle, zone))
            .expect("Unable to send SetViewFrustumZone command.");
        self
    }

    pub fn set_transform(
        &self,
        eye: Vec3,
        look_at: Vec3,
        up: Vec3,
    ) -> &Self {
        self.commands
            .send(AsyncCommand::SetViewFrustumTransforms(
                self.handle,
                eye,
                look_at,
                up,
            ))
            .expect("Unable to send SetViewFrustumTransforms command.");
        self
    }

    pub fn set_projection(
        &self,
        projection: &Projection,
    ) -> &Self {
        self.commands
            .send(AsyncCommand::SetViewFrustumProjection(
                self.handle,
                projection.clone(),
            ))
            .expect("Unable to send SetViewFrustumProjection command.");
        self
    }

    pub fn query_visibility(
        &self,
        results: &mut VisibilityQuery,
    ) -> RafxResult<()> {
        self.visibility_world
            .query_visibility(self.handle, results)
            .map_err(|_err| RafxError::StringError("Unable to query visibility.".to_string()))?;
        Ok(())
    }
}

impl Drop for ViewFrustumObject {
    fn drop(&mut self) {
        let _ = self
            .commands
            .send(AsyncCommand::DestroyViewFrustum(self.handle));
    }
}
