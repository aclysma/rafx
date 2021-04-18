use crate::atomic_once_cell_array::AtomicOnceCellArray;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fixed-size stack with `capacity` determined at run-time. The elements of the stack are uninitialized.
/// Elements may be initialized with `push` and then retrieved as a reference with `get`. `push` returns the
/// index of the element in the stack. The caller may reserve space for multiple elements with `reserve_uninit`.
/// Reserved elements must be initialized with `set`. Calling `push`, `reserve_uninit`, or `set` is thread-safe.
/// The stack will panic if the `capacity` is exceeded, or the `index` is out of range, or the `set` function is
/// called more than once on an index when `T` is not zero-sized, or if the `set` function is called on an index
/// that was not reserved by a prior call to `reserve_uninit`. The stack will only drop initialized elements.
///
/// # Guarantees
///
/// - The stack will not allocate if `capacity` is 0 or if `T` is zero-sized.
/// - The allocated memory will not be `default` initialized.
/// - Elements initialized by `push` or `set` are immutable.
/// - The synchronization is `lock-free`.
///
/// # Zero-sized Types
///
/// When `T` is zero-sized, the stack does not track individual indices and instead only maintains a count of
/// how many instances of `T` have been `set` in the stack. On `drop`, the stack will `drop` that many instances
/// of `T`. The stack will panic if the number of calls to `set` exceeds the capacity.
pub struct AtomicOnceCellStack<T> {
    data: AtomicOnceCellArray<T>,
    last_index: AtomicUsize,
}

impl<T> AtomicOnceCellStack<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        // SAFETY: `data` will catch any issues with <T> or capacity.
        Self {
            data: AtomicOnceCellArray::with_capacity(capacity),
            last_index: AtomicUsize::new(0),
        }
    }

    pub fn push(
        &self,
        val: T,
    ) -> usize {
        // SAFETY: `data` will catch index out of bounds or multiple attempted writes.
        let last_len = self.last_index.fetch_add(1, Ordering::Relaxed);
        self.data.set(last_len, val);
        last_len
    }

    pub fn reserve_uninit(
        &self,
        num_to_reserve: usize,
    ) -> usize {
        let last_len = self.last_index.fetch_add(num_to_reserve, Ordering::Relaxed);

        if last_len + num_to_reserve > self.capacity() {
            // SAFETY: `push` and `set` will catch any incorrect indexing,
            // but there's no sense in waiting to blow up some indeterminate time
            // in the future if we know that this is a problem right now.
            panic!(
                "len {} + num_to_reserve {} must be <= capacity {}",
                last_len,
                num_to_reserve,
                self.capacity()
            );
        }

        last_len
    }

    pub fn set(
        &self,
        index: usize,
        val: T,
    ) {
        if index < self.len() {
            // SAFETY: `data` will catch multiple attempted writes.
            self.data.set(index, val);
        } else {
            // SAFETY:
            panic!(
                "index {} must be < len {} (did you forget to `reserve_uninit` first?)",
                index,
                self.capacity()
            );
        }
    }

    pub fn get(
        &self,
        index: usize,
    ) -> &T {
        // SAFETY: `data` will catch index out of bounds or attempts to read uninitialized memory.
        self.data.get(index)
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn len(&self) -> usize {
        self.last_index.load(Ordering::Acquire)
    }

    pub fn iter(&self) -> Iter<T> {
        self.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a AtomicOnceCellStack<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

pub struct Iter<'a, T> {
    source: &'a AtomicOnceCellStack<T>,
    next_index: usize,
}

impl<'a, T> Iter<'a, T> {
    #[inline]
    pub fn new(source: &'a AtomicOnceCellStack<T>) -> Self {
        Self {
            source,
            next_index: 0,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index < self.source.len() {
            let index = self.next_index;
            self.next_index += 1;
            Some(self.source.get(index))
        } else {
            None
        }
    }
}
