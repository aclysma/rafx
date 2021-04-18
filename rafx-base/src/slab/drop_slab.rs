use super::RawSlab;
use super::RawSlabKey;
use super::SlabIndexT;
use crossbeam_channel::{Receiver, Sender};
use std::marker::PhantomData;
use std::sync::Arc;

/// Wraps a RawSlab with reference counting handles. When the handle is dropped it sends a message
/// to the slab. We process these messages to remove old elements
pub struct DropSlab<T> {
    raw_slab: RawSlab<T>,
    drop_tx: Sender<SlabIndexT>,
    drop_rx: Receiver<SlabIndexT>,
}

impl<T> Default for DropSlab<T> {
    fn default() -> Self {
        Self::with_capacity(32)
    }
}

impl<T> DropSlab<T> {
    /// Create an empty RawSlab
    pub fn new() -> Self {
        Default::default()
    }

    /// Create an empty but presized RawSlab
    pub fn with_capacity(capacity: SlabIndexT) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();
        Self {
            raw_slab: RawSlab::with_capacity(capacity),
            drop_tx,
            drop_rx,
        }
    }

    pub fn process_drops(&mut self) {
        for slab_index in self.drop_rx.try_iter() {
            let raw_slab_key = RawSlabKey::<T>::new(slab_index);
            self.raw_slab.free(raw_slab_key);
        }
    }

    pub fn allocate(
        &mut self,
        value: T,
    ) -> DropSlabKey<T> {
        let slab_key = self.raw_slab.allocate(value);
        DropSlabKey::new(slab_key.index(), self.drop_tx.clone())
    }

    pub fn get(
        &self,
        slab_key: &DropSlabKey<T>,
    ) -> Option<&T> {
        self.get_raw(RawSlabKey::new(slab_key.index()))
    }

    pub fn get_mut(
        &mut self,
        slab_key: &DropSlabKey<T>,
    ) -> Option<&mut T> {
        self.get_raw_mut(RawSlabKey::new(slab_key.index()))
    }

    pub fn get_raw(
        &self,
        raw_slab_key: RawSlabKey<T>,
    ) -> Option<&T> {
        self.raw_slab.get(raw_slab_key)
    }

    pub fn get_raw_mut(
        &mut self,
        raw_slab_key: RawSlabKey<T>,
    ) -> Option<&mut T> {
        self.raw_slab.get_mut(raw_slab_key)
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &T> {
        self.raw_slab.iter().map(move |(_, value)| value)
    }

    pub fn iter_values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.raw_slab.iter_mut().map(move |(_, value)| value)
    }

    // Have not needed these yet
    /*
    pub fn iter(&self) -> impl Iterator<Item = (RawDropSlabKey, &T)> {
        let drop_tx = self.drop_tx.clone();
        self.raw_slab.iter().map(move |(key, value)| {
            (
                DropSlabKey::new(key, drop_tx.clone()),
                value
            )
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (RawDropSlabKey, &mut T)> {
        let drop_tx = self.drop_tx.clone();
        self.raw_slab.iter_mut().map(move |(key, value)| {
            (
                DropSlabKey::new(key, drop_tx.clone()),
                value
            )
        })
    }

    pub fn iter_raw(&self) -> impl Iterator<Item = (RawSlabKey<T>, &T)> {
        self.raw_slab.iter()
    }

    pub fn iter_raw_mut(&mut self) -> impl Iterator<Item = (RawSlabKey<T>, &mut T)> {
        self.raw_slab.iter_mut()
    }
    */

    pub fn allocated_count(&self) -> usize {
        self.raw_slab.allocated_count()
    }

    pub fn storage_size(&self) -> usize {
        self.raw_slab.storage_size()
    }
}

pub struct RawDropSlabKeyInner {
    raw_slab_index: SlabIndexT,
    drop_tx: Sender<SlabIndexT>,
}

impl Drop for RawDropSlabKeyInner {
    fn drop(&mut self) {
        // Not a problem if the rx closed, it would have destroyed the contained objects
        let _ = self.drop_tx.send(self.raw_slab_index);
    }
}

pub struct DropSlabKey<T> {
    inner: Arc<RawDropSlabKeyInner>,
    _phantom: PhantomData<T>,
}

impl<T> Clone for DropSlabKey<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: Default::default(),
        }
    }
}

impl<T> std::fmt::Debug for DropSlabKey<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DropSlabKey")
            .field("index", &self.inner.raw_slab_index)
            .finish()
    }
}

impl<T> DropSlabKey<T> {
    fn new(
        raw_slab_index: SlabIndexT,
        drop_tx: Sender<SlabIndexT>,
    ) -> Self {
        let inner = RawDropSlabKeyInner {
            raw_slab_index,
            drop_tx,
        };

        Self {
            inner: Arc::new(inner),
            _phantom: Default::default(),
        }
    }

    pub fn index(&self) -> SlabIndexT {
        self.inner.raw_slab_index
    }

    pub fn generic_drop_slab_key(&self) -> GenericDropSlabKey {
        GenericDropSlabKey::new(self.inner.clone())
    }
}

pub struct GenericDropSlabKey {
    inner: Arc<RawDropSlabKeyInner>,
}

impl Clone for GenericDropSlabKey {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl std::fmt::Debug for GenericDropSlabKey {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("GenericDropSlabKey")
            .field("index", &self.inner.raw_slab_index)
            .finish()
    }
}

impl GenericDropSlabKey {
    fn new(inner: Arc<RawDropSlabKeyInner>) -> Self {
        Self { inner }
    }

    pub fn index(&self) -> SlabIndexT {
        self.inner.raw_slab_index
    }

    pub fn drop_slab_key<T>(&self) -> DropSlabKey<T> {
        DropSlabKey {
            inner: self.inner.clone(),
            _phantom: Default::default(),
        }
    }
}
