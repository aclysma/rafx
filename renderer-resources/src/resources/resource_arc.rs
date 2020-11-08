use std::fmt::Formatter;
use crossbeam_channel::Sender;
use std::sync::{Weak, Arc};
use std::borrow::Borrow;
use std::hash::Hash;

//TODO: Maybe this should be an enum of ResourceHash and ResourceIndex

// Hijack ResourceHash for now
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ResourceId(pub(crate) u64);

//
// A reference counted object that sends a signal when it's dropped
//
#[derive(Clone)]
pub(crate) struct ResourceWithHash<ResourceT>
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
        self.drop_tx.send(self.resource.clone()).unwrap();
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
    resource_hash: ResourceId,
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

impl<ResourceT> PartialEq for WeakResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.resource_hash == other.resource_hash
    }
}

impl<ResourceT> Eq for WeakResourceArc<ResourceT> where ResourceT: std::fmt::Debug + Clone {}

impl<ResourceT> Hash for WeakResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.resource_hash.hash(state);
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
    pub(crate) fn new(
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
        let resource_hash = self.inner.resource.resource_hash;
        WeakResourceArc {
            inner,
            resource_hash,
        }
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

impl<ResourceT> PartialEq for ResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.inner.resource.resource_hash == other.inner.resource.resource_hash
    }
}

impl<ResourceT> Eq for ResourceArc<ResourceT> where ResourceT: std::fmt::Debug + Clone {}

impl<ResourceT> Hash for ResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
{
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.inner.resource.resource_hash.hash(state);
    }
}
