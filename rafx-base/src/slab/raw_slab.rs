use std::prelude::v1::*;

use super::SlabIndexT;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A key to a value in a RawSlab
pub struct RawSlabKey<T: Sized> {
    /// Raw location within the slab
    index: SlabIndexT,

    phantom_data: PhantomData<T>,
}

impl<T: Sized> Clone for RawSlabKey<T> {
    fn clone(&self) -> RawSlabKey<T> {
        RawSlabKey {
            index: self.index,
            phantom_data: Default::default(),
        }
    }
}

impl<T: Sized> Copy for RawSlabKey<T> {}

impl<T: Sized> PartialEq for RawSlabKey<T> {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.index == other.index
    }
}

impl<T: Sized> Eq for RawSlabKey<T> {}

impl<T: Sized> Hash for RawSlabKey<T> {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.index.hash(state);
    }
}

impl<T: Sized> std::fmt::Debug for RawSlabKey<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RawSlabKey")
            .field("index", &self.index)
            .finish()
    }
}

impl<T: Sized> RawSlabKey<T> {
    pub fn new(index: SlabIndexT) -> Self {
        RawSlabKey {
            index,
            phantom_data: PhantomData,
        }
    }

    pub fn index(self) -> SlabIndexT {
        self.index
    }
}

/// A very simple, minimalist slab structure. Consider using one of the other slabs instead as they
/// are less error prone for many use-cases.
pub struct RawSlab<T> {
    /// List of Ts, will be tightly packed
    storage: Vec<Option<T>>,

    /// List of unused indexes within the storage
    free_list: Vec<SlabIndexT>,
}

impl<T> Default for RawSlab<T> {
    fn default() -> Self {
        Self::with_capacity(32)
    }
}

impl<T> RawSlab<T> {
    /// Create an empty RawSlab
    pub fn new() -> Self {
        Default::default()
    }

    pub fn clear(&mut self) {
        self.storage.clear();
        self.free_list.clear();
    }

    /// Create an empty but presized RawSlab
    pub fn with_capacity(capacity: SlabIndexT) -> Self {
        let mut storage = Vec::with_capacity(capacity as usize);
        let mut free_list = Vec::with_capacity(capacity as usize);

        // reverse count so index 0 is at the top of the free list
        for index in (0..capacity).rev() {
            storage.push(None);
            free_list.push(index);
        }

        RawSlab { storage, free_list }
    }

    /// Allocate a slot within the raw slab.
    ///
    /// Allocation can cause vectors to be resized. Use `with_capacity` to avoid this.
    pub fn allocate(
        &mut self,
        value: T,
    ) -> RawSlabKey<T> {
        let index = self.free_list.pop();

        if let Some(index) = index {
            // Reuse a free slot
            assert!(self.storage[index as usize].is_none());
            self.storage[index as usize] = Some(value);
            RawSlabKey::new(index)
        } else {
            let index = self.storage.len() as SlabIndexT;
            self.storage.push(Some(value));

            RawSlabKey::new(index)
        }
    }

    pub fn allocate_with_key<F: FnMut(RawSlabKey<T>) -> T>(
        &mut self,
        mut f: F,
    ) -> RawSlabKey<T> {
        let index = self.free_list.pop();

        if let Some(index) = index {
            // Reuse a free slot
            assert!(self.storage[index as usize].is_none());
            let slab_key = RawSlabKey::new(index);
            let value = (f)(slab_key);
            self.storage[index as usize] = Some(value);
            slab_key
        } else {
            let index = self.storage.len() as SlabIndexT;
            let slab_key = RawSlabKey::new(index);
            let value = (f)(slab_key);
            self.storage.push(Some(value));
            slab_key
        }
    }

    /// Free an element in the raw slab. It is fatal to free an element that doesn't exist.
    pub fn free(
        &mut self,
        slab_key: RawSlabKey<T>,
    ) {
        assert!(
            self.storage[slab_key.index as usize].is_some(),
            "tried to free a none value"
        );
        self.storage[slab_key.index as usize] = None;
        self.free_list.push(slab_key.index);
    }

    /// Check if an element exists
    pub fn exists(
        &self,
        slab_key: RawSlabKey<T>,
    ) -> bool {
        self.storage[slab_key.index as usize].is_some()
    }

    /// Try to get the given element
    pub fn get(
        &self,
        slab_key: RawSlabKey<T>,
    ) -> Option<&T> {
        // Non-mutable return value so we can return a ref to the value in the vec

        self.storage[slab_key.index as usize].as_ref()
    }

    /// Try to get the given element
    pub fn get_mut(
        &mut self,
        slab_key: RawSlabKey<T>,
    ) -> Option<&mut T> {
        // Mutable reference, and we don't want the caller messing with the Option in the vec,
        // so create a new Option with a mut ref to the value in the vec
        self.storage[slab_key.index as usize].as_mut()
    }

    /// Iterate all values
    pub fn iter(&self) -> impl Iterator<Item = (RawSlabKey<T>, &T)> {
        self.storage
            .iter()
            .enumerate()
            .filter(|(_, value)| value.is_some())
            .map(|(index, value)| (RawSlabKey::new(index as u32), value.as_ref().unwrap()))
    }

    /// Iterate all values
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (RawSlabKey<T>, &mut T)> {
        self.storage
            .iter_mut()
            .enumerate()
            .filter(|(_, value)| value.is_some())
            .map(|(index, value)| (RawSlabKey::new(index as u32), value.as_mut().unwrap()))
    }

    /// Return count of allocated Ts
    pub fn allocated_count(&self) -> usize {
        self.storage.len() - self.free_list.len()
    }

    pub fn storage_size(&self) -> usize {
        self.storage.len()
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

    // Check that trivial allocate/delete works
    #[test]
    fn test_allocate_deallocate_one() {
        let mut pool = RawSlab::<TestStruct>::new();
        let value = TestStruct::new(123);
        let key = pool.allocate(value);

        assert_eq!(1, pool.allocated_count());
        pool.free(key);
        assert_eq!(0, pool.allocated_count());
    }

    #[test]
    #[should_panic(expected = "tried to free a none value")]
    fn test_double_free() {
        let mut pool = RawSlab::<TestStruct>::new();
        let value = TestStruct::new(123);
        let key = pool.allocate(value);

        assert_eq!(1, pool.allocated_count());
        pool.free(key);
        assert_eq!(0, pool.allocated_count());
        pool.free(key);
    }

    // Check that allocation/deallocation in order works
    #[test]
    fn test_allocate_deallocate_fifo() {
        let mut pool = RawSlab::<TestStruct>::new();
        let mut keys = vec![];

        for i in 0..1000 {
            let value = TestStruct::new(i);
            let key = pool.allocate(value);
            keys.push(key);
        }

        assert_eq!(1000, pool.allocated_count());

        for k in keys {
            pool.free(k);
        }

        assert_eq!(0, pool.allocated_count());
    }

    #[test]
    fn test_allocate_deallocate_lifo() {
        let mut pool = RawSlab::<TestStruct>::new();
        let mut keys = vec![];

        for i in 0..1000 {
            let value = TestStruct::new(i);
            let key = pool.allocate(value);
            keys.push(key);
        }

        assert_eq!(1000, pool.allocated_count());

        for i in (0..keys.len()).rev() {
            pool.free(keys[i]);
        }

        assert_eq!(0, pool.allocated_count());
    }

    #[test]
    fn test_get_success() {
        let mut pool = RawSlab::<TestStruct>::new();
        let mut keys = vec![];

        for i in 0..10 {
            let value = TestStruct::new(i);
            let key = pool.allocate(value);
            keys.push(key);
        }

        assert_eq!(10, pool.allocated_count());
        assert_eq!(5, pool.get(keys[5]).unwrap().value);
    }

    #[test]
    fn test_get_fail_out_of_range() {
        let mut pool = RawSlab::<TestStruct>::new();
        let value = TestStruct::new(123);
        let key = pool.allocate(value);
        assert_eq!(1, pool.allocated_count());

        assert!(pool.get(key).is_some());

        pool.free(key);
        assert_eq!(0, pool.allocated_count());

        assert!(pool.get(key).is_none());
    }

    #[test]
    fn test_get_mut_success() {
        let mut pool = RawSlab::<TestStruct>::new();
        let mut keys = vec![];

        for i in 0..10 {
            let value = TestStruct::new(i);
            let key = pool.allocate(value);
            keys.push(key);
        }

        assert_eq!(10, pool.allocated_count());
        assert_eq!(5, pool.get_mut(keys[5]).unwrap().value);
    }

    #[test]
    fn test_get_mut_fail_out_of_range() {
        let mut pool = RawSlab::<TestStruct>::new();
        let value = TestStruct::new(123);
        let key = pool.allocate(value);
        assert_eq!(1, pool.allocated_count());

        assert!(pool.get_mut(key).is_some());

        pool.free(key);
        assert_eq!(0, pool.allocated_count());

        assert!(pool.get_mut(key).is_none());
    }
}
