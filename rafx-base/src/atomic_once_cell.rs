use core::ptr;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU8, Ordering};

const SET_ACQUIRE_FLAG: u8 = 1 << 1;
const SET_RELEASE_FLAG: u8 = 1 << 0;
const IS_INIT_BITMASK: u8 = SET_ACQUIRE_FLAG | SET_RELEASE_FLAG;

/// A thread-safe container that does not require default initialization. The cell may be initialized
/// with `set` and then retrieved as a reference with `get`.  Calling `set` is thread-safe. The cell
/// will panic if the `set` function is called more than once. The cell will only drop initialized elements.
///
/// # Guarantees
///
/// - The allocated memory will not be `default` initialized.
/// - Elements initialized by `set` are immutable.
/// - The synchronization is `lock-free`.
pub struct AtomicOnceCell<T> {
    data: MaybeUninit<UnsafeCell<T>>,
    is_initialized: AtomicU8,
}

impl<T> Default for AtomicOnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AtomicOnceCell<T> {
    pub fn new() -> Self {
        Self {
            data: MaybeUninit::uninit(),
            is_initialized: AtomicU8::new(0),
        }
    }

    #[inline(always)]
    fn start_set(&self) {
        // NOTE(dvd): Use `Acquire` to start a protected section.
        match self
            .is_initialized
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |atomic_val| {
                Some(atomic_val | SET_ACQUIRE_FLAG)
            }) {
            Ok(atomic_val) => {
                if atomic_val & IS_INIT_BITMASK > 0 {
                    // SAFETY: Panic if multiple attempts to initialize the same index occur.
                    panic!("cannot be set more than once");
                }
            }
            _ => unreachable!(),
        };
    }

    #[inline(always)]
    fn end_set(&self) {
        // NOTE(dvd): Use `Release` to end the protected section.
        match self
            .is_initialized
            .fetch_update(Ordering::Release, Ordering::Relaxed, |atomic_val| {
                Some(atomic_val | SET_RELEASE_FLAG)
            }) {
            Ok(_) => {}
            _ => unreachable!(),
        };
    }

    pub fn set(
        &self,
        val: T,
    ) {
        // NOTE(dvd): "Acquire" a lock.
        self.start_set();

        {
            let maybe_uninit = self.ptr_to_maybe_uninit();
            unsafe {
                // SAFETY: If `atomic_val` had neither bits sit, we know that this value
                // is uninitialized & no other thread is trying to initialize it at the same
                // time. If another thread had been trying to initialize it, then the
                // `SET_ACQUIRE_FLAG` would have been set and we would have panicked above.
                // We can therefore safely initialize the `MaybeUninit` value following the
                // example for how to initialize an `UnsafeCell` inside of `MaybeUninit`.
                // https://doc.rust-lang.org/beta/std/cell/struct.UnsafeCell.html#method.raw_get.
                let ptr = AtomicOnceCell::maybe_uninit_as_ptr(maybe_uninit);
                AtomicOnceCell::unsafe_cell_raw_get(ptr).write(val);
            }
        }

        // NOTE(dvd): "Release" the lock.
        self.end_set();
    }

    pub fn get(&self) -> &T {
        let is_initialized = self.is_initialized.load(Ordering::Acquire);
        if is_initialized == 0 {
            // SAFETY: Panic if uninitialized data would be read.
            panic!("not initialized");
        }

        let maybe_uninit = self.ptr_to_maybe_uninit();
        let assume_init = unsafe {
            // SAFETY: We can create a &MaybeUninit because we've initialized the memory
            // in `set`, otherwise we would have panicked above otherwise when checking the bitmask.
            let maybe_uninit_ref = maybe_uninit.as_ref().unwrap();

            // SAFETY: We can then use `assume_init_ref` to get the initialized UnsafeCell<T>.
            AtomicOnceCell::maybe_uninit_assume_init_ref(maybe_uninit_ref)
        };

        let val = unsafe {
            // SAFETY: Cast the &UnsafeCell<T> to &T.
            // This is ok because we know that nothing can mutate the underlying index.
            // If something tried to `set` that index, it would panic instead.
            &*assume_init.get()
        };

        val
    }

    #[inline(always)]
    fn ptr_to_maybe_uninit(&self) -> *const MaybeUninit<UnsafeCell<T>> {
        &self.data as *const MaybeUninit<UnsafeCell<T>>
    }

    #[inline(always)]
    fn ptr_to_maybe_uninit_mut(&mut self) -> *mut MaybeUninit<UnsafeCell<T>> {
        &mut self.data as *mut MaybeUninit<UnsafeCell<T>>
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

impl<T> Drop for AtomicOnceCell<T> {
    fn drop(&mut self) {
        // SAFETY: We don't need to be concerned about any set that conceptually occurs while the
        // `drop` in progress because `drop` takes a &mut self so no other code has a &self.

        let atomic_val = self.is_initialized.load(Ordering::Relaxed);
        let is_initialized = atomic_val & IS_INIT_BITMASK == IS_INIT_BITMASK;

        if is_initialized {
            let maybe_uninit = self.ptr_to_maybe_uninit_mut();
            unsafe {
                // SAFETY: If the bitmask has both bits set, this index is definitely initialized.
                ptr::drop_in_place(AtomicOnceCell::maybe_uninit_as_mut_ptr(maybe_uninit))
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

unsafe impl<T> Sync for AtomicOnceCell<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::sync::mpsc::{Receiver, Sender};
    use std::{panic, thread};

    struct DroppableElement {
        id: usize,
        sender: Option<Sender<usize>>,
    }

    impl DroppableElement {
        pub fn new(
            id: usize,
            sender: Option<&Sender<usize>>,
        ) -> Self {
            Self {
                id,
                sender: sender.map(|sender| sender.clone()),
            }
        }
    }

    impl Drop for DroppableElement {
        fn drop(&mut self) {
            if let Some(sender) = &self.sender {
                let _ = sender.send(self.id);
            }
        }
    }

    fn default_drop() -> (AtomicOnceCell<DroppableElement>, Receiver<usize>) {
        let array = AtomicOnceCell::new();

        let receiver = {
            let (sender, receiver) = mpsc::channel();
            array.set(DroppableElement::new(0, Some(&sender)));
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
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0], 0);
    }

    #[test]
    fn test_drop_panic() {
        let (array, receiver) = default_drop();

        assert_eq!(receiver.try_recv().ok(), None);

        let result = thread::spawn(move || {
            array.set(DroppableElement::new(1, None)); // NOTE(dvd): `array` panics here.
        })
        .join();

        assert!(result.is_err());

        let indices = receiver.iter().collect::<Vec<_>>();
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0], 0);
    }

    #[test]
    fn test_drop_thread() {
        let (array, receiver) = default_drop();

        assert_eq!(receiver.try_recv().ok(), None);

        let result = thread::spawn(move || {
            assert_eq!(array.get().id, 0);
            // NOTE(dvd): `array` is dropped here.
        })
        .join();

        assert!(result.is_ok());

        let indices = receiver.iter().collect::<Vec<_>>();
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0], 0);
    }

    struct PanicOnDropElement {
        _id: u32,
    }

    impl Drop for PanicOnDropElement {
        fn drop(&mut self) {
            panic!("element dropped");
        }
    }

    fn default_panic_on_drop() -> AtomicOnceCell<PanicOnDropElement> {
        AtomicOnceCell::new()
    }

    #[test]
    fn test_drop_no_panic() {
        let array = default_panic_on_drop();
        std::mem::drop(array);
    }

    fn default_i32() -> AtomicOnceCell<i32> {
        AtomicOnceCell::new()
    }

    #[test]
    fn test_set_0() {
        let array = default_i32();
        array.set(7);
        assert_eq!(array.get(), &7);
    }

    #[test]
    #[should_panic(expected = "cannot be set more than once")]
    fn test_set_0_twice() {
        let array = default_i32();
        array.set(12);
        assert_eq!(array.get(), &12);
        array.set(-2);
    }

    #[test]
    #[should_panic(expected = "not initialized")]
    fn test_get_0_uninitialized() {
        let array = default_i32();
        array.get();
    }

    // NOTE(dvd): The zero-sized T variant of the struct requires separate tests.

    struct ZeroSizedType {}

    fn default_zst() -> AtomicOnceCell<ZeroSizedType> {
        AtomicOnceCell::new()
    }

    #[test]
    fn test_zst_set_7() {
        let array = default_zst();
        array.set(ZeroSizedType {});
        array.get();
    }

    #[test]
    #[should_panic(expected = "not initialized")]
    fn test_zst_get_7_uninitialized() {
        let array = default_zst();

        // NOTE(dvd): See comment on `test_zst_get_0_uninitialized_private_type`.
        array.get();
    }

    mod zst_lifetime {
        struct PrivateInnerZst {}

        pub struct CannotConstructZstLifetime<'a, T> {
            _guard: PrivateInnerZst,
            _phantom: std::marker::PhantomData<&'a T>,
        }
    }

    #[test]
    #[should_panic(expected = "not initialized")]
    fn test_zst_get_0_uninitialized_lifetime<'a>() {
        use zst_lifetime::CannotConstructZstLifetime;

        let array = AtomicOnceCell::new();

        // NOTE(dvd): See comment on `test_zst_get_0_uninitialized_private_type`.
        let _val: &CannotConstructZstLifetime<'a, u32> = array.get();
    }

    mod zst_private {
        struct PrivateInnerZst {}

        pub struct CannotConstructZstInner(PrivateInnerZst);
    }

    #[test]
    #[should_panic(expected = "not initialized")]
    fn test_zst_get_0_uninitialized_private_type() {
        use zst_private::CannotConstructZstInner;

        let array = AtomicOnceCell::new();

        // NOTE(dvd): Even though T is zero-sized, we must have
        // a proof that the user could construct T, otherwise
        // this container would allow the user to get a &T that
        // they aren't supposed to have -- e.g. due to a private
        // zero-sized member in T, or a lifetime requirement.
        let _val: &CannotConstructZstInner = array.get();
    }

    enum Void {}

    #[test]
    #[should_panic(expected = "not initialized")]
    fn test_zst_get_0_uninitialized_void() {
        let array = AtomicOnceCell::new();

        // NOTE(dvd): See comment on `test_zst_get_0_uninitialized_private_type`.
        let _val: &Void = array.get();
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
        let array = AtomicOnceCell::new();
        array.set(ObservableZstDrop::new());
        assert_eq!(get_counter(), 1);

        std::mem::drop(array);
        assert_eq!(get_counter(), 0);
    }
}
