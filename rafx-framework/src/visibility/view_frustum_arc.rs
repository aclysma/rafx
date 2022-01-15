use crate::visibility::{VisibilityConfig, VisibilityResource};
use crate::RafxResult;
use crossbeam_channel::Sender;
use glam::Vec3;
use parking_lot::{RwLock, RwLockReadGuard};
use rafx_api::RafxError;
use rafx_visibility::{AsyncCommand, Projection, ViewFrustumHandle, VisibilityQuery, ZoneHandle};
use std::sync::Arc;

pub type ViewFrustumId = (Option<ViewFrustumHandle>, Option<ViewFrustumHandle>);

struct ViewFrustumArcInner {
    visibility_query: RwLock<VisibilityQuery>,
    static_view_frustum: Option<ViewFrustumRaii>,
    dynamic_view_frustum: Option<ViewFrustumRaii>,
}

#[derive(Clone)]
pub struct ViewFrustumArc {
    inner: Arc<ViewFrustumArcInner>,
}

impl ViewFrustumArc {
    pub fn new(
        static_view_frustum: Option<ViewFrustumRaii>,
        dynamic_view_frustum: Option<ViewFrustumRaii>,
    ) -> Self {
        Self {
            inner: Arc::new(ViewFrustumArcInner {
                visibility_query: Default::default(),
                static_view_frustum,
                dynamic_view_frustum,
            }),
        }
    }

    pub fn view_frustum_id(&self) -> ViewFrustumId {
        (
            self.inner.static_view_frustum.as_ref().map(|x| x.handle),
            self.inner.dynamic_view_frustum.as_ref().map(|x| x.handle),
        )
    }

    #[allow(dead_code)]
    pub(super) fn set_zone(
        &self,
        static_zone: Option<ZoneHandle>,
        dynamic_zone: Option<ZoneHandle>,
    ) -> &Self {
        if let Some(static_view_frustum) = &self.inner.static_view_frustum {
            static_view_frustum.set_zone(static_zone);
        }

        if let Some(dynamic_view_frustum) = &self.inner.dynamic_view_frustum {
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
        if let Some(static_view_frustum) = &self.inner.static_view_frustum {
            static_view_frustum.set_transform(eye, look_at, up);
        }

        if let Some(dynamic_view_frustum) = &self.inner.dynamic_view_frustum {
            dynamic_view_frustum.set_transform(eye, look_at, up);
        }

        self
    }

    pub fn set_projection(
        &self,
        projection: &Projection,
    ) -> &Self {
        if let Some(static_view_frustum) = &self.inner.static_view_frustum {
            static_view_frustum.set_projection(projection);
        }

        if let Some(dynamic_view_frustum) = &self.inner.dynamic_view_frustum {
            dynamic_view_frustum.set_projection(projection);
        }

        self
    }

    pub fn query_visibility(
        &self,
        visibility_resource: &VisibilityResource,
        visibility_config: &VisibilityConfig,
    ) -> RafxResult<RwLockReadGuard<VisibilityQuery>> {
        if visibility_config.enable_visibility_update {
            let mut results = self.inner.visibility_query.write();

            results.objects.clear();
            results.volumes.clear();
            if let Some(static_view_frustum) = &self.inner.static_view_frustum {
                static_view_frustum.query_visibility(visibility_resource, &mut results)?;
            }

            if let Some(dynamic_view_frustum) = &self.inner.dynamic_view_frustum {
                dynamic_view_frustum.query_visibility(visibility_resource, &mut results)?;
            }
        }

        Ok(self.inner.visibility_query.read())
    }
}

// An RAII object for a ViewFrustumHandle
pub struct ViewFrustumRaii {
    commands: Sender<AsyncCommand>,
    handle: ViewFrustumHandle,
}

impl Drop for ViewFrustumRaii {
    fn drop(&mut self) {
        let _ = self
            .commands
            .send(AsyncCommand::DestroyViewFrustum(self.handle));
    }
}

impl ViewFrustumRaii {
    pub fn new(
        handle: ViewFrustumHandle,
        commands: Sender<AsyncCommand>,
    ) -> Self {
        Self { handle, commands }
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
        visibility_resource: &VisibilityResource,
        results: &mut VisibilityQuery,
    ) -> RafxResult<()> {
        visibility_resource
            .world()
            .query_visibility(self.handle, results)
            .map_err(|_err| RafxError::StringError("Unable to query visibility.".to_string()))?;
        Ok(())
    }
}
