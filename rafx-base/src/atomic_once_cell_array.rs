use core::ptr;
use std::cell::UnsafeCell;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_CAPACITY: usize = isize::MAX as usize;

const IS_INIT_BITMASK_LEN: usize = 2;
const SET_ACQUIRE_FLAG: usize = 1 << 1;
const SET_RELEASE_FLAG: usize = 1 << 0;
const IS_INIT_BITMASK: usize = SET_ACQUIRE_FLAG | SET_RELEASE_FLAG;

const BITS_PER_BYTE: usize = 8;

#[inline(always)]
fn atomic_addr(index: usize) -> (usize, usize) {
    let index = IS_INIT_BITMASK_LEN * index;
    (index / bits_per_atomic(), index % bits_per_atomic())
}

#[inline(always)]
fn atomic_size() -> usize {
    std::mem::size_of::<usize>()
}

#[inline(always)]
fn bits_per_atomic() -> usize {
    atomic_size() * BITS_PER_BYTE
}

enum Index {
    Indices(Box<[AtomicUsize]>),
    Zst(AtomicUsize),
}

/// A fixed-size array with `capacity` determined at run-time. The elements of the array are uninitialized.
/// Elements may be initialized with `set` and then retrieved as a reference with `get`.  Calling `set` is
/// thread-safe. The array will panic if the `capacity` is exceeded, or the `index` is out of range, or the
/// `set` function is called more than once on an index when `T` is not zero-sized. The array will only drop
/// initialized elements.
///
/// # Guarantees
///
/// - The array will not allocate if `capacity` is 0 or if `T` is zero-sized.
/// - The allocated memory will not be `default` initialized.
/// - Elements initialized by `set` are immutable.
/// - The synchronization is `lock-free`.
///
/// # Zero-sized Types
///
/// When `T` is zero-sized, the array does not track individual indices and instead only maintains a count of
/// how many instances of `T` have been `set` in the array. On `drop`, the array will `drop` that many instances
/// of `T`. The array will panic if the number of calls to `set` exceeds the capacity.
pub struct AtomicOnceCellArray<T> {
    data: Box<[MaybeUninit<UnsafeCell<T>>]>,
    indices: Index,
}

impl<T> AtomicOnceCellArray<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        // NOTE(dvd): The cost of allocation should be the cost to allocate an uninitialized Vec<T>
        // with capacity C and the overhead of allocating a fully initialized Vec<AtomicUsize> with
        // capacity for 2 * C bits.

        if capacity > MAX_CAPACITY {
            panic!("capacity may not exceed {}", MAX_CAPACITY);
        }

        let mut data = Vec::with_capacity(capacity);
        unsafe {
            // SAFETY: This is a Vec of `MaybeUninit` so it's ok to set the len
            // to the capacity. In `set`, `get`, and `Drop` we'll make sure that no code
            // ever gets a &T where T wasn't initialized.
            data.set_len(capacity);
        };

        let indices = if std::mem::size_of::<T>() > 0 && capacity > 0 {
            let (max_atomic_index, _atomic_offset) = atomic_addr(capacity);
            let mut indices = Vec::with_capacity(max_atomic_index + 1);
            for _ in 0..=max_atomic_index {
                indices.push(AtomicUsize::new(0));
            }
            indices
        } else {
            // NOTE(dvd): If T is zero-sized, we can skip allocating the indices.
            Vec::with_capacity(0)
        };

        Self {
            data: data.into_boxed_slice(),
            indices: if indices.capacity() > 0 {
                Index::Indices(indices.into_boxed_slice())
            } else {
                Index::Zst(AtomicUsize::new(0))
            },
        }
    }

    #[inline(always)]
    fn start_set(
        &self,
        indices: &[AtomicUsize],
        index: usize,
    ) -> (usize, usize) {
        // NOTE(dvd): Use `Acquire` to start a protected section.
        let addr = atomic_addr(index);
        let set_acquire_flag = SET_ACQUIRE_FLAG << addr.1;
        match indices[addr.0].fetch_update(Ordering::Acquire, Ordering::Relaxed, |atomic_val| {
            Some(atomic_val | set_acquire_flag)
        }) {
            Ok(atomic_val) => {
                if atomic_val & (IS_INIT_BITMASK << addr.1) > 0 {
                    // SAFETY: Panic if multiple attempts to initialize the same index occur.
                    panic!("index {} cannot be set more than once", index);
                }
            }
            _ => unreachable!(),
        };

        addr
    }

    #[inline(always)]
    fn end_set(
        &self,
        indices: &[AtomicUsize],
        addr: (usize, usize),
    ) {
        // NOTE(dvd): Use `Release` to end a protected section.
        let set_release_flag = SET_RELEASE_FLAG << addr.1;
        match indices[addr.0].fetch_update(Ordering::Release, Ordering::Relaxed, |atomic_val| {
            Some(atomic_val | set_release_flag)
        }) {
            Ok(_) => {}
            _ => unreachable!(),
        };
    }

    pub fn set(
        &self,
        index: usize,
        val: T,
    ) {
        // Initialization is done per-index.

        if index >= self.capacity() {
            // NOTE(dvd): Panic if index is out of range.
            panic!("index {} must be < capacity {}", index, self.capacity());
        }

        if std::mem::size_of::<T>() == 0 {
            // SAFETY: If T is zero-sized, we need a sentinel to prove
            // that a value of T has been moved into this collection.
            // Once we've established that requirement, it's safe to return
            // a reference to that T.
            let num_initialized = self.zst();
            if num_initialized.load(Ordering::Acquire) < self.capacity() {
                // SAFETY: Mark that we'll manually drop `val`.
                let _ = ManuallyDrop::new(val);
                num_initialized.fetch_add(1, Ordering::Release);
            } else {
                // SAFETY: We need to call `drop` for each element added to the
                // container, so we'll track the number of zero-sized types added
                // and call `drop` that many times.
                panic!("capacity overflow");
            }

            // SAFETY: A zero-sized type cannot be initialized more than once because
            // it doesn't have any data to set. With that in mind, we can exit early.
            return;
        }

        // NOTE(dvd): "Acquire" a lock.
        let indices = self.indices();
        let addr = self.start_set(indices, index);

        {
            let maybe_uninit = self.ptr_to_maybe_uninit(index);
            unsafe {
                // SAFETY: If `atomic_val` had neither bits sit, we know that this value
                // is uninitialized & no other thread is trying to initialize it at the same
                // time. If another thread had been trying to initialize it, then the
                // `SET_ACQUIRE_FLAG` would have been set and we would have panicked above.
                // We can therefore safely initialize the `MaybeUninit` value following the
                // example for how to initialize an `UnsafeCell` inside of `MaybeUninit`.
                // https://doc.rust-lang.org/beta/std/cell/struct.UnsafeCell.html#method.raw_get.
                let ptr = AtomicOnceCellArray::maybe_uninit_as_ptr(maybe_uninit);
                AtomicOnceCellArray::unsafe_cell_raw_get(ptr).write(val);
            }
        }

        // NOTE(dvd): "Release" the lock.
        self.end_set(indices, addr);
    }

    pub fn get(
        &self,
        index: usize,
    ) -> &T {
        // Read-only access is provided per-index.

        if index >= self.capacity() {
            // NOTE(dvd): Panic if index is out of range.
            panic!("index {} must be < capacity {}", index, self.capacity());
        }

        if std::mem::size_of::<T>() == 0 {
            let num_initialized = self.zst().load(Ordering::Acquire);
            if num_initialized == 0 {
                // SAFETY: Panic if uninitialized data would be read.
                panic!("index {} is not initialized", index);
            }
        } else {
            let indices = self.indices();

            let (atomic_index, atomic_offset) = atomic_addr(index);
            let atomic_val = indices[atomic_index].load(Ordering::Acquire);

            let is_init_bitmask = IS_INIT_BITMASK << atomic_offset;
            if atomic_val & is_init_bitmask != is_init_bitmask {
                // SAFETY: Panic if uninitialized data would be read.
                panic!("index {} is not initialized", index);
            }
        }

        let maybe_uninit = self.ptr_to_maybe_uninit(index);
        let assume_init = unsafe {
            // SAFETY: We can create a &MaybeUninit because we've initialized the memory
            // in `set`, otherwise we would have panicked above otherwise when checking the bitmask.
            let maybe_uninit_ref = maybe_uninit.as_ref().unwrap();

            // SAFETY: We can then use `assume_init_ref` to get the initialized UnsafeCell<T>.
            AtomicOnceCellArray::maybe_uninit_assume_init_ref(maybe_uninit_ref)
        };

        let val = unsafe {
            // SAFETY: Cast the &UnsafeCell<T> to &T.
            // This is ok because we know that nothing can mutate the underlying index.
            // If something tried to `set` that index, it would panic instead.
            &*assume_init.get()
        };

        val
    }

    pub unsafe fn get_all_unchecked(&self) -> &[T] {
        std::mem::transmute::<&[MaybeUninit<UnsafeCell<T>>], &[T]>(&self.data[..])
    }

    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    fn zst(&self) -> &AtomicUsize {
        if std::mem::size_of::<T>() != 0 {
            unreachable!();
        }

        match &self.indices {
            Index::Indices(_) => {
                unreachable!()
            }
            Index::Zst(num_initialized) => num_initialized,
        }
    }

    #[inline(always)]
    fn indices(&self) -> &Box<[AtomicUsize]> {
        if std::mem::size_of::<T>() == 0 {
            unreachable!();
        }

        match &self.indices {
            Index::Indices(indices) => indices,
            Index::Zst(_) => {
                unreachable!()
            }
        }
    }

    #[inline(always)]
    fn ptr_to_maybe_uninit(
        &self,
        index: usize,
    ) -> *const MaybeUninit<UnsafeCell<T>> {
        unsafe {
            // SAFETY: Index can be cast to `isize` because the stdlib would panic
            // if we tried to allocate a Vec with capacity larger than that.
            self.data.as_ptr().offset(index as isize)
        }
    }

    #[inline(always)]
    fn ptr_to_maybe_uninit_mut(
        &mut self,
        index: usize,
    ) -> *mut MaybeUninit<UnsafeCell<T>> {
        unsafe {
            // SAFETY: Index can be cast to `isize` because the stdlib would panic
            // if we tried to allocate a Vec with capacity larger than that.
            self.data.as_mut_ptr().offset(index as isize)
        }
    }

    #[inline(always)]
    unsafe fn maybe_uninit_as_ptr(
        maybe_uninit: *const MaybeUninit<UnsafeCell<T>>
    ) -> *const UnsafeCell<T> {
        // SAFETY: Equivalent to MaybeUninit::as_ptr, but defined for a ptr instead of &self.
        // https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#method.as_ptr
        maybe_uninit as *const _ as *const UnsafeCell<T>
    }

    #[inline(always)]
    unsafe fn maybe_uninit_as_mut_ptr(
        maybe_uninit: *mut MaybeUninit<UnsafeCell<T>>
    ) -> *mut UnsafeCell<T> {
        // SAFETY: Equivalent to MaybeUninit::as_mut_ptr, but defined for a ptr instead of &mut self.
        // https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#method.as_mut_ptr
        maybe_uninit as *mut _ as *mut UnsafeCell<T>
    }

    #[inline(always)]
    unsafe fn unsafe_cell_raw_get(cell: *const UnsafeCell<T>) -> *mut T {
        // SAFETY: Equivalent to the unstable API UnsafeCell::raw_get defined at
        // https://doc.rust-lang.org/beta/std/cell/struct.UnsafeCell.html#method.raw_get
        cell as *const T as *mut T
    }

    #[inline(always)]
    unsafe fn maybe_uninit_assume_init_ref(
        maybe_uninit: &MaybeUninit<UnsafeCell<T>>
    ) -> &UnsafeCell<T> {
        // SAFETY: Equivalent to the unstable API MaybeUninit::assume_init_ref defined at
        // https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#method.assume_init_ref
        &*maybe_uninit.as_ptr()
    }
}

impl<T> Drop for AtomicOnceCellArray<T> {
    fn drop(&mut self) {
        // SAFETY: We don't need to be concerned about any set that conceptually occurs while the
        // `drop` in progress because `drop` takes a &mut self so no other code has a &self.

        if std::mem::size_of::<T>() == 0 {
            // SAFETY: This is the same behavior for a Vec of zero-sized types.
            let num_initialized = self.zst().load(Ordering::Relaxed);
            for _ in 0..num_initialized {
                let maybe_uninit = self.ptr_to_maybe_uninit_mut(0);
                unsafe {
                    ptr::drop_in_place(AtomicOnceCellArray::maybe_uninit_as_mut_ptr(maybe_uninit))
                }
            }
            return;
        }

        for index in 0..self.capacity() {
            let is_initialized = {
                let indices = self.indices();

                let (atomic_index, atomic_offset) = atomic_addr(index);
                let atomic_val = indices[atomic_index].load(Ordering::Relaxed);

                let is_init_bitmask = IS_INIT_BITMASK << atomic_offset;

                atomic_val & is_init_bitmask == is_init_bitmask
            };

            if is_initialized {
                let maybe_uninit = self.ptr_to_maybe_uninit_mut(index);
                unsafe {
                    // SAFETY: If the bitmask has both bits set, this index is definitely initialized.
                    ptr::drop_in_place(AtomicOnceCellArray::maybe_uninit_as_mut_ptr(maybe_uninit))
                }
            } else {
                // SAFETY: If the bitmask only has the high bit set (the set was in progress),
                // we won't drop it, so the value T that was moved into `set` will get leaked just
                // like mem::forget (which is safe).
                // If the bitmask has both bits unset, that index doesn't need to be dropped
                // because it's definitely uninitialized.
            }
        }
    }
}

unsafe impl<T> Sync for AtomicOnceCellArray<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{mpsc, Arc};
    use std::{panic, thread};

    struct DroppableElement {
        id: usize,
        sender: Sender<usize>,
    }

    impl DroppableElement {
        pub fn new(
            id: usize,
            sender: &Sender<usize>,
        ) -> Self {
            Self {
                id,
                sender: sender.clone(),
            }
        }
    }

    impl Drop for DroppableElement {
        fn drop(&mut self) {
            let _ = self.sender.send(self.id);
        }
    }

    fn default_drop() -> (AtomicOnceCellArray<DroppableElement>, Receiver<usize>) {
        let array = AtomicOnceCellArray::with_capacity(10);

        let receiver = {
            let (sender, receiver) = mpsc::channel();
            array.set(3, DroppableElement::new(3, &sender));
            array.set(6, DroppableElement::new(6, &sender));
            receiver
        };

        (array, receiver)
    }

    #[test]
    fn test_drop() {
        let (array, receiver) = default_drop();

        assert_eq!(receiver.try_recv().ok(), None);

        // NOTE(dvd): `array` is dropped here.
        std::mem::drop(array);

        let indices = receiver.iter().collect::<Vec<_>>();
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0], 3);
        assert_eq!(indices[1], 6);
    }

    #[test]
    fn test_drop_panic() {
        let (array, receiver) = default_drop();

        assert_eq!(receiver.try_recv().ok(), None);

        let result = thread::spawn(move || {
            array.get(4); // NOTE(dvd): `array` panics here.
        })
        .join();

        assert!(result.is_err());

        let indices = receiver.iter().collect::<Vec<_>>();
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0], 3);
        assert_eq!(indices[1], 6);
    }

    #[test]
    fn test_drop_thread() {
        let (array, receiver) = default_drop();

        assert_eq!(receiver.try_recv().ok(), None);

        let result = thread::spawn(move || {
            assert_eq!(array.get(6).id, 6);
            // NOTE(dvd): `array` is dropped here.
        })
        .join();

        assert!(result.is_ok());

        let indices = receiver.iter().collect::<Vec<_>>();
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0], 3);
        assert_eq!(indices[1], 6);
    }

    struct PanicOnDropElement {
        _id: u32,
    }

    impl Drop for PanicOnDropElement {
        fn drop(&mut self) {
            panic!("element dropped");
        }
    }

    fn default_panic_on_drop() -> AtomicOnceCellArray<PanicOnDropElement> {
        AtomicOnceCellArray::with_capacity(10)
    }

    #[test]
    fn test_drop_no_panic() {
        let array = default_panic_on_drop();
        std::mem::drop(array);
    }

    fn default_unallocated_bool() -> AtomicOnceCellArray<bool> {
        AtomicOnceCellArray::with_capacity(0)
    }

    #[test]
    fn test_unallocated() {
        let array = default_unallocated_bool();
        assert_eq!(array.capacity(), 0);
    }

    #[test]
    #[should_panic(expected = "index 0 must be < capacity 0")]
    fn test_set_0_unallocated() {
        let array = default_unallocated_bool();
        array.set(0, true);
    }

    #[test]
    #[should_panic(expected = "index 0 must be < capacity 0")]
    fn test_get_0_unallocated() {
        let array = default_unallocated_bool();
        array.get(0);
    }

    fn default_i32() -> AtomicOnceCellArray<i32> {
        AtomicOnceCellArray::with_capacity(10)
    }

    #[test]
    fn test_set_0() {
        let array = default_i32();
        array.set(0, 7);
        assert_eq!(array.get(0), &7);
    }

    #[test]
    #[should_panic(expected = "index 0 cannot be set more than once")]
    fn test_set_0_twice() {
        let array = default_i32();
        array.set(0, 12);
        assert_eq!(array.get(0), &12);
        array.set(0, -2);
    }

    #[test]
    #[should_panic(expected = "index 0 is not initialized")]
    fn test_get_0_uninitialized() {
        let array = default_i32();
        array.get(0);
    }

    #[test]
    fn test_set_3() {
        let array = default_i32();
        assert_eq!(array.capacity(), 10);
        array.set(3, 8658);
        assert_eq!(array.get(3), &8658);
        assert_eq!(array.capacity(), 10);
    }

    #[test]
    #[should_panic(expected = "index 3 cannot be set more than once")]
    fn test_set_3_twice() {
        let array = default_i32();
        array.set(3, 12);
        assert_eq!(array.get(3), &12);
        array.set(3, -2);
    }

    #[test]
    #[should_panic(expected = "index 3 is not initialized")]
    fn test_get_3_uninitialized() {
        let array = default_i32();
        array.get(3);
    }

    #[test]
    fn test_set_capacity() {
        let array = default_i32();
        array.set(array.capacity() - 1, 4663);
        assert_eq!(array.get(array.capacity() - 1), &4663);
    }

    #[test]
    #[should_panic(expected = "index 9 cannot be set more than once")]
    fn test_set_capacity_twice() {
        let array = default_i32();
        array.set(array.capacity() - 1, -25);
        assert_eq!(array.get(array.capacity() - 1), &-25);
        array.set(array.capacity() - 1, 745);
    }

    #[test]
    #[should_panic(expected = "index 9 is not initialized")]
    fn test_get_capacity_uninitialized() {
        let array = default_i32();
        array.get(array.capacity() - 1);
    }

    #[test]
    #[should_panic(expected = "index 11 must be < capacity 10")]
    fn test_get_index_out_of_range() {
        let array = default_i32();
        array.get(array.capacity() + 1);
    }

    #[test]
    #[should_panic(expected = "index 11 must be < capacity 10")]
    fn test_set_index_out_of_range() {
        let array = default_i32();
        array.set(array.capacity() + 1, 0);
    }

    #[test]
    fn test_set_parallel() {
        let array = Arc::new(default_i32());

        let mut join_handles = Vec::with_capacity(array.capacity());
        for index in 0..array.capacity() {
            let array = array.clone();
            join_handles.push(thread::spawn(move || {
                array.set(index, index as i32 * 2);
            }));
        }

        for join_handle in join_handles {
            join_handle.join().unwrap()
        }

        for index in 0..array.capacity() {
            assert_eq!(array.get(index), &(index as i32 * 2));
        }
    }

    #[test]
    #[should_panic]
    fn test_set_parallel_panic() {
        let array = Arc::new(AtomicOnceCellArray::with_capacity(100));

        let mut join_handles = Vec::with_capacity(array.capacity());
        for index in 0..array.capacity() {
            let array = array.clone();
            join_handles.push(thread::spawn(move || {
                array.set(index, index as i32 * 2);
            }));
        }

        for index in 0..array.capacity() {
            assert_eq!(array.get(index), &(index as i32 * 2));
        }
    }

    // NOTE(dvd): The zero-sized T variant of the struct requires separate tests.

    struct ZeroSizedType {}

    fn default_zst() -> AtomicOnceCellArray<ZeroSizedType> {
        AtomicOnceCellArray::with_capacity(10)
    }

    #[test]
    fn test_zst_capacity() {
        let array = default_zst();
        assert_eq!(array.capacity(), 10);
    }

    #[test]
    fn test_zst_set_7() {
        let array = default_zst();
        array.set(7, ZeroSizedType {});
        array.get(7);
    }

    #[test]
    fn test_zst_set_1_2_3() {
        let array = default_zst();
        array.set(1, ZeroSizedType {});
        array.set(2, ZeroSizedType {});
        array.set(3, ZeroSizedType {});
    }

    #[test]
    fn test_zst_set_1_2_get_3() {
        let array = default_zst();
        array.set(1, ZeroSizedType {});
        array.get(1);
        array.set(2, ZeroSizedType {});
        array.get(2);

        // NOTE(dvd): T is zero-sized, so no init is required
        // as long as at least one value of T has been `set`
        // in the array.
        array.get(3);
    }

    #[test]
    fn test_zst_set_7_twice() {
        let array = default_zst();
        array.set(7, ZeroSizedType {});
        array.get(7);

        // NOTE(dvd): T is zero-sized, so a `set` is actually
        // a NOP and cannot change the contents of the memory.
        array.set(7, ZeroSizedType {});
    }

    #[test]
    #[should_panic(expected = "index 7 is not initialized")]
    fn test_zst_get_7_uninitialized() {
        let array = default_zst();

        // NOTE(dvd): See comment on `test_zst_get_0_uninitialized_private_type`.
        array.get(7);
    }

    mod zst_lifetime {
        struct PrivateInnerZst {}

        pub struct CannotConstructZstLifetime<'a, T> {
            _guard: PrivateInnerZst,
            _phantom: std::marker::PhantomData<&'a T>,
        }
    }

    #[test]
    #[should_panic(expected = "index 0 is not initialized")]
    fn test_zst_get_0_uninitialized_lifetime<'a>() {
        use zst_lifetime::CannotConstructZstLifetime;

        let array = AtomicOnceCellArray::with_capacity(1);

        // NOTE(dvd): See comment on `test_zst_get_0_uninitialized_private_type`.
        let _val: &CannotConstructZstLifetime<'a, u32> = array.get(0);
    }

    mod zst_private {
        struct PrivateInnerZst {}

        pub struct CannotConstructZstInner(PrivateInnerZst);
    }

    #[test]
    #[should_panic(expected = "index 0 is not initialized")]
    fn test_zst_get_0_uninitialized_private_type() {
        use zst_private::CannotConstructZstInner;

        let array = AtomicOnceCellArray::with_capacity(1);

        // NOTE(dvd): Even though T is zero-sized, we must have
        // a proof that the user could construct T, otherwise
        // this container would allow the user to get a &T that
        // they aren't supposed to have -- e.g. due to a private
        // zero-sized member in T, or a lifetime requirement.
        let _val: &CannotConstructZstInner = array.get(0);
    }

    enum Void {}

    #[test]
    #[should_panic(expected = "index 0 is not initialized")]
    fn test_zst_get_0_uninitialized_void() {
        let array = AtomicOnceCellArray::with_capacity(1);

        // NOTE(dvd): See comment on `test_zst_get_0_uninitialized_private_type`.
        let _val: &Void = array.get(0);
    }

    #[test]
    #[should_panic(expected = "index 11 must be < capacity 10")]
    fn test_zst_get_index_out_of_range() {
        let array = default_zst();
        array.get(array.capacity() + 1);
    }

    #[test]
    #[should_panic(expected = "index 11 must be < capacity 10")]
    fn test_zst_set_index_out_of_range() {
        let array = default_zst();
        array.set(array.capacity() + 1, ZeroSizedType {});
    }

    #[test]
    fn test_zst_set_parallel() {
        let array = Arc::new(default_zst());

        let mut join_handles = Vec::with_capacity(array.capacity());
        for index in 0..array.capacity() {
            let array = array.clone();
            join_handles.push(thread::spawn(move || {
                array.set(index, ZeroSizedType {});
            }));
        }

        for join_handle in join_handles {
            join_handle.join().unwrap()
        }

        for index in 0..array.capacity() {
            array.get(index);
        }
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn test_zst_overflow() {
        let array = AtomicOnceCellArray::with_capacity(4);
        for _ in 0..=array.capacity() {
            array.set(2, ZeroSizedType {});
        }
    }

    #[test]
    #[should_panic(expected = "capacity may not exceed")]
    fn test_zst_invalid_capacity() {
        let array = AtomicOnceCellArray::with_capacity(MAX_CAPACITY + 1);
        array.set(0, ZeroSizedType {});
    }

    #[test]
    fn test_zst_observable_drop() {
        mod zst_drop {
            // IMPORTANT(dvd): This mod is defined inside of the function because
            // the use of a static atomic here is a hilarious race condition if
            // multiple tests try to use the `ObservableZstDrop`. The reason why
            // we can't put a reference to the counter inside of the zero-sized type
            // is because then it wouldn't be zero-sized anymore.

            use std::sync::atomic::{AtomicU32, Ordering};

            static ATOMIC_COUNTER: AtomicU32 = AtomicU32::new(0);

            struct PrivateInnerZst {}

            pub struct ObservableZstDrop(PrivateInnerZst);

            impl ObservableZstDrop {
                pub fn new() -> Self {
                    assert_eq!(std::mem::size_of::<Self>(), 0);
                    ATOMIC_COUNTER.fetch_add(1, Ordering::Relaxed);
                    ObservableZstDrop(PrivateInnerZst {})
                }
            }

            impl Drop for ObservableZstDrop {
                fn drop(&mut self) {
                    ATOMIC_COUNTER.fetch_sub(1, Ordering::Relaxed);
                }
            }

            pub fn get_counter() -> u32 {
                ATOMIC_COUNTER.load(Ordering::Relaxed)
            }
        }

        use zst_drop::{get_counter, ObservableZstDrop};

        assert_eq!(get_counter(), 0);
        let array = AtomicOnceCellArray::with_capacity(5);
        for index in 0..5 {
            array.set(index, ObservableZstDrop::new());
        }
        assert_eq!(get_counter(), 5);

        std::mem::drop(array);
        assert_eq!(get_counter(), 0);
    }
}
