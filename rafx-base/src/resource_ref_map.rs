use crate::memory::force_to_static_lifetime_mut;
use crate::resource_map::{ReadBorrow, Resource, ResourceMap, WriteBorrow};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Like ResourceMap, but the insertions are temporary. This is an alternative to making a large
/// amount of code generic just to pass down an arbitrary <T>
#[derive(Default)]
pub struct ResourceRefMap<'a> {
    resources: ResourceMap,
    phantom_data: PhantomData<&'a ()>,
}

impl<'a> ResourceRefMap<'a> {
    pub fn new() -> Self {
        ResourceRefMap::default()
    }

    /// Add a type/resource instance to the map
    pub fn insert<R>(
        &mut self,
        r: &'a mut R,
    ) where
        R: Resource,
    {
        unsafe {
            self.resources
                .insert(ResourceRef(force_to_static_lifetime_mut(r)));
        }
    }

    /// Remove a type/resource instance from the map
    pub fn remove<R>(&mut self) -> Option<&'a mut R>
    where
        R: Resource,
    {
        self.resources.remove::<ResourceRef<R>>().map(|x| x.0)
    }

    /// Read-only fetch of a resource. Trying to get a resource that is not in the map is fatal. Use
    /// try_fetch if unsure whether the resource exists. Requesting read access to a resource that
    /// has any concurrently active writer is fatal.
    pub fn fetch<R: Resource>(&self) -> ResourceRefBorrow<R> {
        ResourceRefBorrow(self.resources.fetch::<ResourceRef<R>>())
    }

    /// Read-only fetch of a resource. Requesting read access to a resource that has a concurrently
    /// active writer is fatal. Returns None if the type is not registered.
    pub fn try_fetch<R: Resource>(&self) -> Option<ResourceRefBorrow<R>> {
        self.resources.try_fetch().map(|x| ResourceRefBorrow(x))
    }

    /// Read/Write fetch of a resource. Trying to get a resource that is not in the map is fatal. Use
    /// try_fetch if unsure whether the resource exists. Requesting write access to a resource with
    /// any concurrently active read/write is fatal
    pub fn fetch_mut<R: Resource>(&self) -> ResourceRefBorrowMut<R> {
        ResourceRefBorrowMut(self.resources.fetch_mut::<ResourceRef<R>>())
    }

    /// Read/Write fetch of a resource. Requesting write access to a resource with
    /// any concurrently active read/write is fatal. Returns None if the type is not registered.
    pub fn try_fetch_mut<R: Resource>(&self) -> Option<ResourceRefBorrowMut<R>> {
        self.resources
            .try_fetch_mut()
            .map(|x| ResourceRefBorrowMut(x))
    }

    /// Returns true if the resource is registered.
    pub fn has_value<R>(&self) -> bool
    where
        R: Resource,
    {
        self.resources.has_value::<ResourceRef<R>>()
    }
}

//
// ResourceRef
//

// static reference is dangerous, must only be used when extracting
pub struct ResourceRef<T: 'static>(&'static mut T);

impl<T> ResourceRef<T> {
    pub unsafe fn new(resources: &mut T) -> Self {
        ResourceRef(force_to_static_lifetime_mut(resources))
    }
}

impl<T> Deref for ResourceRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

// unsafe impl<T> Send for ResourceRef<T> {}
// unsafe impl<T> Sync for ResourceRef<T> {}

//
// ResourceRefBorrow
//

// static reference is dangerous, must only be used when extracting
pub struct ResourceRefBorrow<'a, T: Resource>(ReadBorrow<'a, ResourceRef<T>>);

impl<'a, T: Resource> ResourceRefBorrow<'a, T> {
    pub fn new(resource: ReadBorrow<'a, ResourceRef<T>>) -> Self {
        ResourceRefBorrow(resource)
    }
}

impl<'a, T: Resource> Deref for ResourceRefBorrow<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0 .0
    }
}

//
// ResourceRefBorrowMut
//
pub struct ResourceRefBorrowMut<'a, T: Resource>(WriteBorrow<'a, ResourceRef<T>>);

impl<'a, T: Resource> ResourceRefBorrowMut<'a, T> {
    pub fn new(resource: WriteBorrow<'a, ResourceRef<T>>) -> Self {
        ResourceRefBorrowMut(resource)
    }
}

impl<'a, T: Resource> Deref for ResourceRefBorrowMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0 .0
    }
}

impl<'a, T: Resource> DerefMut for ResourceRefBorrowMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0 .0
    }
}

//
// tests
//
#[test]
fn test_extract_resources() {
    let mut resources = ResourceRefMap::default();
    let mut x: i32 = 50;
    resources.insert(&mut x);

    {
        let mut x_borrowed = resources.fetch_mut::<i32>();
        assert_eq!(*x_borrowed, 50);
        *x_borrowed += 10;
    }

    assert_eq!(x, 60);
}
