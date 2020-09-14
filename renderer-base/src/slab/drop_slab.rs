use super::RawSlab;
use super::RawSlabKey;
use crossbeam_channel::{Sender, Receiver};
use super::SlabIndexT;
use std::sync::Arc;

pub struct DropSlabKeyInner<T> {
    raw_slab_key: RawSlabKey<T>,
    drop_tx: Sender<RawSlabKey<T>>,
}

impl<T: Sized> Drop for DropSlabKeyInner<T> {
    fn drop(&mut self) {
        // Not a problem if the rx closed, it would have destroyed the contained objects
        let _ = self.drop_tx.send(self.raw_slab_key);
    }
}

pub struct DropSlabKey<T> {
    inner: Arc<DropSlabKeyInner<T>>,
}

impl<T: Sized> Clone for DropSlabKey<T> {
    fn clone(&self) -> DropSlabKey<T> {
        DropSlabKey {
            inner: self.inner.clone(),
        }
    }
}

// Ideally we don't need this, would need to decide if drop_tx should be included
/*
impl<T: Sized> PartialEq for DropSlabKey<T> {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.raw_slab_key.index() == other.raw_slab_key.index()
    }
}

impl<T: Sized> Eq for DropSlabKey<T> {}

impl<T: Sized> Hash for DropSlabKey<T> {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.raw_slab_key.hash(state);
    }
}
*/

impl<T: Sized> std::fmt::Debug for DropSlabKey<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RawSlabKey")
            .field("index", &self.inner.raw_slab_key.index())
            .finish()
    }
}

impl<T: Sized> DropSlabKey<T> {
    pub fn new(
        raw_slab_key: RawSlabKey<T>,
        drop_tx: Sender<RawSlabKey<T>>,
    ) -> Self {
        let inner = DropSlabKeyInner {
            raw_slab_key,
            drop_tx,
        };

        DropSlabKey {
            inner: Arc::new(inner),
        }
    }

    pub fn index(&self) -> SlabIndexT {
        self.inner.raw_slab_key.index()
    }
}

/// Wraps a RawSlab with reference counting handles. When the handle is dropped it sends a message
/// to the slab. We process these messages to remove old elements
pub struct DropSlab<T> {
    raw_slab: RawSlab<T>,
    drop_tx: Sender<RawSlabKey<T>>,
    drop_rx: Receiver<RawSlabKey<T>>,
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
        DropSlab {
            raw_slab: RawSlab::with_capacity(capacity),
            drop_tx,
            drop_rx,
        }
    }

    pub fn process_drops(&mut self) {
        for slab_key in self.drop_rx.try_iter() {
            self.raw_slab.free(slab_key);
        }
    }

    pub fn allocate(
        &mut self,
        value: T,
    ) -> DropSlabKey<T> {
        let slab_key = self.raw_slab.allocate(value);
        DropSlabKey::new(slab_key, self.drop_tx.clone())
    }

    pub fn get(
        &self,
        slab_key: &DropSlabKey<T>,
    ) -> Option<&T> {
        self.raw_slab.get(slab_key.inner.raw_slab_key)
    }

    pub fn get_raw(
        &self,
        raw_slab_key: RawSlabKey<T>,
    ) -> Option<&T> {
        self.raw_slab.get(raw_slab_key)
    }

    pub fn get_mut(
        &mut self,
        slab_key: &DropSlabKey<T>,
    ) -> Option<&mut T> {
        self.raw_slab.get_mut(slab_key.inner.raw_slab_key)
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
    pub fn iter(&self) -> impl Iterator<Item = (DropSlabKey<T>, &T)> {
        let drop_tx = self.drop_tx.clone();
        self.raw_slab.iter().map(move |(key, value)| {
            (
                DropSlabKey::new(key, drop_tx.clone()),
                value
            )
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (DropSlabKey<T>, &mut T)> {
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
