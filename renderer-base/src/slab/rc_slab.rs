use std::prelude::v1::*;

use super::GenSlab;
use super::GenSlabKey;
use super::SlabIndexT;
use std::sync::Arc;
use std::sync::Weak;

/// A key to access values in RcSlab
pub struct RcSlabEntry<T> {
    slab_key: Arc<GenSlabKey<T>>,
}

impl<T> RcSlabEntry<T> {
    pub fn new(slab_key: GenSlabKey<T>) -> Self {
        RcSlabEntry {
            slab_key: Arc::new(slab_key),
        }
    }

    pub fn downgrade(&self) -> WeakSlabEntry<T> {
        WeakSlabEntry::new(self)
    }
}

impl<T> Clone for RcSlabEntry<T> {
    fn clone(&self) -> Self {
        RcSlabEntry::<T> {
            slab_key: Arc::clone(&self.slab_key),
        }
    }
}

impl<T> PartialEq for RcSlabEntry<T> {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.slab_key == other.slab_key
    }
}

impl<T> Eq for RcSlabEntry<T> {}

impl<T> std::hash::Hash for RcSlabEntry<T> {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.slab_key.hash(state);
    }
}

impl<T> std::fmt::Debug for RcSlabEntry<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        (*self.slab_key).fmt(f)
    }
}

/// A key to access values in RcSlab
pub struct WeakSlabEntry<T> {
    slab_key: Weak<GenSlabKey<T>>,
}

impl<T> WeakSlabEntry<T> {
    pub fn new(slab_entry: &RcSlabEntry<T>) -> Self {
        WeakSlabEntry {
            slab_key: Arc::downgrade(&slab_entry.slab_key),
        }
    }

    pub fn upgrade(&self) -> Option<RcSlabEntry<T>> {
        Some(RcSlabEntry {
            slab_key: self.slab_key.upgrade()?,
        })
    }

    pub fn can_upgrade(&self) -> bool {
        //self.slab_key.weak_count()
        unimplemented!()
    }
}

//impl<T> std::clone::Clone for WeakSlabEntry<T> {
//    fn clone(&self) -> Self {
//        WeakSlabEntry::<T> {
//            slab_key = Weak::<T>::clone(&self.slab_key)
//        }
//    }
//}

//TODO: Would it be safe to simply use RawSlab here? The current API might make it impossible to end
// up with stale keys

/// A GenSlab where rather than explicitly calling allocate/free, allocate returns a reference-counted
/// handle. Update() must be called regularly. This frees elements that are no longer referenced.
///
/// You must call update to flush any old values. There are a few reasons why this design was chosen:
/// - Mutating any state within RcSlab can be tricky
/// - Don't want overhead of RcSlabKey keeping a pointer back to its owner.
pub struct RcSlab<T> {
    slab: GenSlab<T>,
    entries: Vec<RcSlabEntry<T>>,
}

impl<T> Default for RcSlab<T> {
    fn default() -> Self {
        Self::with_capacity(32)
    }
}

impl<T> RcSlab<T> {
    /// Returns an empty RcSlab
    pub fn new() -> Self {
        Default::default()
    }

    /// Return an empty but presized RcSlab
    pub fn with_capacity(capacity: SlabIndexT) -> Self {
        let entries = Vec::with_capacity(capacity as usize);

        RcSlab::<T> {
            slab: GenSlab::<T>::with_capacity(capacity),
            entries,
        }
    }

    /// Allocates a slot, returning a SlabEntry. Elements in this slab are reference-counted.
    /// Unreferenced elements are removed when update() is called
    ///
    /// Allocation can cause vectors to be resized. Use `with_capacity` to avoid this.
    pub fn allocate(
        &mut self,
        value: T,
    ) -> RcSlabEntry<T> {
        let key = self.slab.allocate(value);
        let entry = RcSlabEntry::new(key);
        self.entries.push(entry.clone());
        entry
    }

    /// Returns true if the entry exists. If it doesn't exist, it implies you've used the
    /// wrong key with the wrong slab
    pub fn exists(
        &self,
        slab_entry: &RcSlabEntry<T>,
    ) -> bool {
        self.slab.exists(&*slab_entry.slab_key)
    }

    /// Get the element associated with the given key
    pub fn get(
        &self,
        slab_entry: &RcSlabEntry<T>,
    ) -> &T {
        self.slab.get(&*slab_entry.slab_key).unwrap()
    }

    /// Get the element associated with the given key
    pub fn get_mut(
        &mut self,
        slab_entry: &RcSlabEntry<T>,
    ) -> &mut T {
        self.slab.get_mut(&*slab_entry.slab_key).unwrap()
    }

    /// Iterate all values
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slab.iter()
    }

    /// Iterate all values
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slab.iter_mut()
    }

    /// Return count of allocated values
    pub fn count(&self) -> usize {
        self.slab.count()
    }

    /// Must be called regularly to detect and remove values that are no longer referenced
    pub fn update(&mut self) {
        for index in (0..self.entries.len()).rev() {
            if Arc::strong_count(&self.entries[index].slab_key) == 1 {
                self.slab.free(&self.entries[index].slab_key);
                self.entries.swap_remove(index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestStruct {
        value: u32,
    }

    impl TestStruct {
        fn new(value: u32) -> Self {
            TestStruct { value }
        }
    }

    #[test]
    fn test_rc_allocate_deallocate_one() {
        let mut pool = RcSlab::<TestStruct>::new();
        let value = TestStruct::new(123);
        {
            let _entry = pool.allocate(value);
            assert_eq!(1, pool.count());
        }

        assert_eq!(1, pool.count());
        pool.update();
        assert_eq!(0, pool.count());
    }

    #[test]
    fn test_rc_get_success() {
        let mut pool = RcSlab::<TestStruct>::new();
        let mut keys = vec![];

        for i in 0..10 {
            let value = TestStruct::new(i);
            let key = pool.allocate(value);
            keys.push(key);
        }

        assert_eq!(10, pool.count());
        assert_eq!(5, pool.get(&keys[5]).value);
    }

    #[test]
    fn test_rc_get_mut_success() {
        let mut pool = RcSlab::<TestStruct>::new();
        let mut keys = vec![];

        for i in 0..10 {
            let value = TestStruct::new(i);
            let key = pool.allocate(value);
            keys.push(key);
        }

        assert_eq!(10, pool.count());
        assert_eq!(5, pool.get_mut(&keys[5]).value);
    }
}
