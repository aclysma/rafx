use std::collections::HashMap;
use std::hash::Hash;

use super::RcSlab;
use super::RcSlabEntry;
use super::SlabIndexT;
use super::WeakSlabEntry;

/// An RcSlab where every value has a unique key. A typical usecase for this would be loading assets
/// and wanting to quickly determine if a certain asset already exists. Update() must be called regularly.
/// This frees elements that are no longer referenced.
pub struct KeyedRcSlab<KeyT: Eq + Hash, ValueT> {
    /// Underlying RcSlab for containing the data
    slab: RcSlab<ValueT>,

    /// Lookup structure for associating slab entries with arbitrary keys
    lookup: HashMap<KeyT, WeakSlabEntry<ValueT>>,
}

impl<KeyT: Eq + Hash, ValueT> Default for KeyedRcSlab<KeyT, ValueT> {
    fn default() -> Self {
        Self::with_capacity(32)
    }
}

impl<KeyT: Eq + Hash, ValueT> KeyedRcSlab<KeyT, ValueT> {
    /// Create an empty KeyedRcSlab
    pub fn new() -> Self {
        Default::default()
    }

    /// Create an empty but presized KeyedRcSlab
    pub fn with_capacity(capacity: SlabIndexT) -> Self {
        KeyedRcSlab::<KeyT, ValueT> {
            slab: RcSlab::with_capacity(capacity),
            lookup: HashMap::with_capacity(capacity as usize),
        }
    }

    //TODO: An API more like HashMap with entry() might be useful here

    /// Allocate a slot. If the element already exists, it is simply returned. If it doesn't exist,
    /// it's allocated a slot in the slab.
    pub fn allocate(
        &mut self,
        key: KeyT,
        value: ValueT,
    ) -> RcSlabEntry<ValueT> {
        match self.find(&key) {
            Some(ptr) => ptr,
            None => {
                let ptr = self.slab.allocate(value);
                self.lookup.insert(key, ptr.downgrade());
                ptr
            }
        }
    }

    /// Allocate a slot. If the element already exists, it is simply returned. If it doesn't exist,
    /// the provided callback is called and the return value goes into a slot in the slab.
    ///
    /// Allocation can cause vectors to be resized. Use `with_capacity` to avoid this.
    pub fn allocate_with<F: FnOnce() -> ValueT>(
        &mut self,
        key: KeyT,
        insert_fn: F,
    ) -> RcSlabEntry<ValueT> {
        match self.find(&key) {
            Some(ptr) => ptr,
            None => {
                let ptr = self.slab.allocate(insert_fn());
                self.lookup.insert(key, ptr.downgrade());
                ptr
            }
        }
    }

    /// True if the element exists
    pub fn exists(
        &self,
        slab_entry: &RcSlabEntry<ValueT>,
    ) -> bool {
        self.slab.exists(slab_entry)
    }

    /// Get the element via slab key
    pub fn get(
        &self,
        slab_entry: &RcSlabEntry<ValueT>,
    ) -> &ValueT {
        self.slab.get(slab_entry)
    }

    /// Get the element via slab key
    pub fn get_mut(
        &mut self,
        slab_entry: &RcSlabEntry<ValueT>,
    ) -> &mut ValueT {
        self.slab.get_mut(slab_entry)
    }

    /// Find the slab_entry for the given key, or None if it doesn't exist
    pub fn find(
        &self,
        key: &KeyT,
    ) -> Option<RcSlabEntry<ValueT>> {
        self.lookup.get(key)?.upgrade()
    }

    /// Iterate all ValueTs
    pub fn iter(&self) -> impl Iterator<Item = &ValueT> {
        self.slab.iter()
    }

    /// Iterate all ValueTs
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ValueT> {
        self.slab.iter_mut()
    }

    /// Returns the number of allocated values
    pub fn count(&self) -> usize {
        self.slab.count()
    }

    /// Must be called regularly to detect and remove values that are no longer referenced
    pub fn update(&mut self) {
        // This could drop data that is no longer referenced anywhere
        self.slab.update();

        // Drop any data from the hash map that can't be upgraded
        self.lookup.retain(|_k, v| v.upgrade().is_some());
    }
}
