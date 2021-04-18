use crossbeam_channel::{bounded, Receiver, Sender};
use std::ops::{Deref, DerefMut};

pub struct OwnedPool<T> {
    free: Vec<T>,
    num_borrowed: usize,
    receiver: Receiver<T>,
    sender: Sender<T>,
    reset_fn: fn(&mut T),
    init_fn: fn() -> T,
}

impl<T> OwnedPool<T> {
    pub fn with_capacity(
        pool_size: usize,
        init_fn: fn() -> T,
        reset_fn: fn(&mut T),
    ) -> Self {
        let (sender, receiver) = bounded(pool_size);
        Self {
            free: Vec::with_capacity(pool_size),
            num_borrowed: 0,
            init_fn,
            reset_fn,
            sender,
            receiver,
        }
    }

    pub fn borrow(&mut self) -> Pooled<T> {
        if let Some(mut pooled_val) = self.free.pop() {
            self.num_borrowed += 1;
            (self.reset_fn)(&mut pooled_val);
            Pooled {
                inner: Some(pooled_val),
                sender: self.sender.clone(),
            }
        } else if self.num_borrowed < self.free.capacity() {
            self.num_borrowed += 1;
            Pooled {
                inner: Some((self.init_fn)()),
                sender: self.sender.clone(),
            }
        } else {
            panic!("Cannot borrow more than `pool_size` entries.");
        }
    }

    pub fn try_recv(&mut self) {
        for pooled_val in self.receiver.try_iter() {
            self.free.push(pooled_val);
            self.num_borrowed -= 1;
        }
    }
}

pub struct Pooled<T> {
    inner: Option<T>,
    sender: Sender<T>,
}

impl<T> Pooled<T> {
    pub fn as_ref(&self) -> &T {
        self.inner.as_ref().unwrap()
    }

    pub fn as_mut(&mut self) -> &mut T {
        self.inner.as_mut().unwrap()
    }
}

impl<T> Deref for Pooled<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<T> DerefMut for Pooled<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl<T> Drop for Pooled<T> {
    fn drop(&mut self) {
        let inner = std::mem::take(&mut self.inner);
        let _ = self.sender.send(inner.unwrap());
    }
}
