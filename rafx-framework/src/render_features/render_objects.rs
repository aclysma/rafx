use crate::render_features::render_features_prelude::*;
use rafx_base::slab::{DropSlab, GenericDropSlabKey, RawSlabKey, SlabIndexT};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

pub type RenderObjectsMap<RenderObjectStaticDataT> =
    RenderObjectSetStorage<RenderObjectStaticDataT>;

pub type RenderObjectCount = u32;

type RenderObjectSetKey = GenericDropSlabKey;

/// A reference to a `RenderObject` with a reference-counted pointer. When the last instance of
/// a `RenderObjectHandle` is dropped, the referenced `RenderObject` will be freed from the
/// `RenderObjectSet`. This is returned by `register_render_object` on the `RenderObjectSet`
/// and may be used to query or mutate the `RenderObject` with the `get` or `get_mut` methods.
#[derive(Clone, Debug)]
pub struct RenderObjectHandle {
    render_feature_index: RenderFeatureIndex,
    render_object_set_key: RenderObjectSetKey,
}

/// A weak `RenderObjectHandle`. This is a reference to a `RenderObject` that is not used in
/// reference-counting by the `RenderObjectSet`. Create a `RenderObjectId` with the `as_id()`
/// method on a `RenderObjectHandle`. The `RenderObject` can be queried with the `get_id` method on
/// the `RenderObjectSet`.
#[derive(Copy, Eq, PartialEq, Hash, Clone, Debug)]
pub struct RenderObjectId {
    render_feature_index: RenderFeatureIndex,
    render_object_set_index: SlabIndexT,
}

impl Ord for RenderObjectId {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.render_feature_index
            .cmp(&other.render_feature_index)
            .then(
                self.render_object_set_index
                    .cmp(&other.render_object_set_index),
            )
    }
}

impl PartialOrd for RenderObjectId {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for RenderObjectId {
    fn default() -> Self {
        Self {
            render_feature_index: RenderFeatureIndex::MAX,
            render_object_set_index: SlabIndexT::MAX,
        }
    }
}

impl Eq for RenderObjectHandle {}

impl PartialEq for RenderObjectHandle {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.render_feature_index == other.render_feature_index
            && self.render_object_set_key.index() == other.render_object_set_key.index()
    }
}

impl Hash for RenderObjectHandle {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.render_feature_index.hash(state);
        self.render_object_set_key.index().hash(state);
    }
}

impl RenderObjectHandle {
    fn new(
        render_feature_index: RenderFeatureIndex,
        render_object_id: RenderObjectSetKey,
    ) -> Self {
        RenderObjectHandle {
            render_feature_index,
            render_object_set_key: render_object_id,
        }
    }

    pub fn render_feature_index(&self) -> RenderFeatureIndex {
        self.render_feature_index
    }

    pub fn as_id(&self) -> RenderObjectId {
        RenderObjectId {
            render_feature_index: self.render_feature_index,
            render_object_set_index: self.render_object_set_key.index(),
        }
    }
}

impl RenderObjectId {
    #[inline(always)]
    pub fn render_feature_index(&self) -> RenderFeatureIndex {
        self.render_feature_index
    }
}

pub struct RenderObjectSet<RenderFeatureT: RenderFeature, RenderObjectStaticDataT> {
    storage: Arc<RwLock<RenderObjectsMap<RenderObjectStaticDataT>>>,
    _phantom: PhantomData<RenderFeatureT>,
}

impl<RenderFeatureT: RenderFeature, RenderObjectStaticDataT> Clone
    for RenderObjectSet<RenderFeatureT, RenderObjectStaticDataT>
{
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            _phantom: Default::default(),
        }
    }
}

impl<RenderFeatureT: RenderFeature, RenderObjectStaticDataT>
    RenderObjectSet<RenderFeatureT, RenderObjectStaticDataT>
{
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(RenderObjectSetStorage::new())),
            _phantom: Default::default(),
        }
    }

    pub fn register_render_object(
        &mut self,
        render_object: RenderObjectStaticDataT,
    ) -> RenderObjectHandle {
        let render_object_handle = {
            let mut render_objects = self.write();
            render_objects.register_render_object(RenderFeatureT::feature_index(), render_object)
        };

        render_object_handle
    }

    pub fn read(&self) -> RwLockReadGuard<RenderObjectsMap<RenderObjectStaticDataT>> {
        let registry = &self.storage;
        registry.try_read().unwrap_or_else(move || {
            log::warn!(
                "{} is being written by another thread.",
                std::any::type_name::<RenderObjectsMap<RenderObjectStaticDataT>>()
            );

            registry.read()
        })
    }

    fn write(&mut self) -> RwLockWriteGuard<RenderObjectsMap<RenderObjectStaticDataT>> {
        let registry = &self.storage;
        registry.try_write().unwrap_or_else(move || {
            log::warn!(
                "{} is being read or written by another thread.",
                std::any::type_name::<RenderObjectsMap<RenderObjectStaticDataT>>()
            );

            registry.write()
        })
    }

    #[allow(dead_code)]
    fn feature_index(&self) -> RenderFeatureIndex {
        RenderFeatureT::feature_index()
    }
}

impl<RenderFeatureT: RenderFeature, RenderObjectStaticDataT> Default
    for RenderObjectSet<RenderFeatureT, RenderObjectStaticDataT>
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct RenderObjectSetStorage<RenderObjectStaticDataT> {
    inner: DropSlab<RenderObjectStaticDataT>,
}

impl<RenderObjectStaticDataT> RenderObjectSetStorage<RenderObjectStaticDataT> {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn len(&self) -> RenderObjectCount {
        self.inner.allocated_count() as RenderObjectCount
    }

    pub fn register_render_object(
        &mut self,
        feature_index: RenderFeatureIndex,
        render_object: RenderObjectStaticDataT,
    ) -> RenderObjectHandle {
        self.inner.process_drops();

        let drop_slab_key = self.inner.allocate(render_object);
        RenderObjectHandle::new(feature_index, drop_slab_key.generic_drop_slab_key())
    }

    pub fn get_id(
        &self,
        render_object: &RenderObjectId,
    ) -> &RenderObjectStaticDataT {
        let raw_slab_key =
            RawSlabKey::<RenderObjectStaticDataT>::new(render_object.render_object_set_index);

        self.inner.get_raw(raw_slab_key).unwrap_or_else(|| {
            panic!(
                "{} did not contain id {:?}.",
                std::any::type_name::<RenderObjectsMap<RenderObjectStaticDataT>>(),
                render_object
            )
        })
    }

    pub fn get(
        &self,
        render_object: &RenderObjectHandle,
    ) -> &RenderObjectStaticDataT {
        self.inner
            .get(&render_object.render_object_set_key.drop_slab_key())
            .unwrap_or_else(|| {
                panic!(
                    "{} did not contain handle {:?}.",
                    std::any::type_name::<RenderObjectsMap<RenderObjectStaticDataT>>(),
                    render_object
                )
            })
    }

    pub fn get_mut(
        &mut self,
        render_object: &RenderObjectHandle,
    ) -> &mut RenderObjectStaticDataT {
        self.inner
            .get_mut(&render_object.render_object_set_key.drop_slab_key())
            .unwrap_or_else(|| {
                panic!(
                    "{} did not contain handle {:?}.",
                    std::any::type_name::<RenderObjectsMap<RenderObjectStaticDataT>>(),
                    render_object
                )
            })
    }
}

impl<RenderObjectStaticDataT> Default for RenderObjectSetStorage<RenderObjectStaticDataT> {
    fn default() -> Self {
        Self::new()
    }
}
