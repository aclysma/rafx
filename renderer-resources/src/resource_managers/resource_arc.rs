use std::fmt::Formatter;
use crossbeam_channel::Sender;
use std::sync::{Weak, Arc};
use std::borrow::Borrow;

//TODO: Maybe this should be an enum of ResourceHash and ResourceIndex

// Hijack ResourceHash for now
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct ResourceId(pub(super) u64);

//
// A reference counted object that sends a signal when it's dropped
//
#[derive(Clone)]
pub(super) struct ResourceWithHash<ResourceT>
where
    ResourceT: Clone,
{
    pub(super) resource: ResourceT,
    pub(super) resource_hash: ResourceId,
}

impl<ResourceT> std::fmt::Debug for ResourceWithHash<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ResourceWithHash")
            .field("resource", &self.resource)
            .field("resource_hash", &self.resource_hash)
            .finish()
    }
}

struct ResourceArcInner<ResourceT>
where
    ResourceT: Clone,
{
    resource: ResourceWithHash<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
}

impl<ResourceT> Drop for ResourceArcInner<ResourceT>
where
    ResourceT: Clone,
{
    fn drop(&mut self) {
        self.drop_tx.send(self.resource.clone());
    }
}

impl<ResourceT> std::fmt::Debug for ResourceArcInner<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ResourceArcInner")
            .field("resource", &self.resource)
            .finish()
    }
}

#[derive(Clone)]
pub struct WeakResourceArc<ResourceT>
where
    ResourceT: Clone,
{
    inner: Weak<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> WeakResourceArc<ResourceT>
where
    ResourceT: Clone,
{
    pub fn upgrade(&self) -> Option<ResourceArc<ResourceT>> {
        if let Some(upgrade) = self.inner.upgrade() {
            Some(ResourceArc { inner: upgrade })
        } else {
            None
        }
    }
}

impl<ResourceT> std::fmt::Debug for WeakResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("WeakResourceArc")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone)]
pub struct ResourceArc<ResourceT>
where
    ResourceT: Clone,
{
    inner: Arc<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> ResourceArc<ResourceT>
where
    ResourceT: Clone,
{
    pub(super) fn new(
        resource: ResourceT,
        resource_hash: ResourceId,
        drop_tx: Sender<ResourceWithHash<ResourceT>>,
    ) -> Self {
        ResourceArc {
            inner: Arc::new(ResourceArcInner {
                resource: ResourceWithHash {
                    resource,
                    resource_hash,
                },
                drop_tx,
            }),
        }
    }

    pub fn get_raw(&self) -> ResourceT {
        self.inner.resource.borrow().resource.clone()
    }

    pub(super) fn get_hash(&self) -> ResourceId {
        self.inner.resource.resource_hash
    }

    pub fn downgrade(&self) -> WeakResourceArc<ResourceT> {
        let inner = Arc::downgrade(&self.inner);
        WeakResourceArc { inner }
    }
}

impl<ResourceT> std::fmt::Debug for ResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ResourceArc")
            .field("inner", &self.inner)
            .finish()
    }
}
