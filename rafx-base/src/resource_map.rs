//! Allows placing resources (i.e. "global" data) in a dictionary and looking it up by type. The data
//! could be "global" systems, component storages, component factories, etc.
//!
//! This implements a type system for expressing read/write dependencies. Many readers and single
//! writers are allowed, but not both at the same time. This is checked at runtime, not compile time.
//!
//! Lots of inspiration taken from `shred` for how to create a type system
//! to express read/write dependencies

//
// ResourceId
//
use std::any::TypeId;
use std::marker::PhantomData;
use std::prelude::v1::*;

use downcast_rs::Downcast;
use fnv::FnvHashMap as HashMap;

use crate::trust_cell::{Ref, RefMut, TrustCell};

/// Every type can be converted to a `ResourceId`. The ResourceId is used to look up the type's value
/// in the `ResourceMap`
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceId {
    type_id: TypeId,
}

impl ResourceId {
    /// Creates a new resource id from a given type.
    #[inline]
    pub fn new<T: 'static>() -> Self {
        ResourceId {
            type_id: std::any::TypeId::of::<T>(),
        }
    }
}

/// Any data that can be stored in the ResourceMap must implement this trait. There is a blanket
/// implementation provided for all compatible types
pub trait Resource: Downcast + Send + Sync + 'static {}

impl<T> Resource for T where T: Downcast + Send + Sync {}

// Used for downcastic
mod __resource_mopafy_scope {
    #![allow(clippy::all)]

    use super::Resource;

    downcast_rs::impl_downcast!(Resource);
}

/// Builder for creating a ResourceMap
pub struct ResourceMapBuilder {
    /// The ResourceMap being built
    resource_map: ResourceMap,
}

impl ResourceMapBuilder {
    /// Creates an empty builder
    pub fn new() -> Self {
        ResourceMapBuilder {
            resource_map: ResourceMap::new(),
        }
    }

    /// Builder-style API that adds the resource to the map
    pub fn with_resource<R>(
        mut self,
        r: R,
    ) -> Self
    where
        R: Resource,
    {
        self.resource_map.insert(r);
        self
    }

    /// Adds the resource to the map
    pub fn insert<R>(
        &mut self,
        r: R,
    ) where
        R: Resource,
    {
        self.resource_map.insert(r);
    }

    /// Consume this builder, returning the resource map
    pub fn build(self) -> ResourceMap {
        self.resource_map
    }
}

/// A key-value structure. The key is a type, and the value is a single object of that type
#[derive(Default)]
pub struct ResourceMap {
    resources: HashMap<ResourceId, TrustCell<Box<dyn Resource>>>,
}

impl ResourceMap {
    /// Creates an empty resource map
    pub fn new() -> Self {
        ResourceMap {
            resources: HashMap::default(),
        }
    }

    /// Add a type/resource instance to the map
    pub fn insert<R>(
        &mut self,
        r: R,
    ) where
        R: Resource,
    {
        self.insert_by_id(ResourceId::new::<R>(), r);
    }

    /// Remove a type/resource instance from the map
    pub fn remove<R>(&mut self) -> Option<R>
    where
        R: Resource,
    {
        self.remove_by_id(ResourceId::new::<R>())
    }

    fn insert_by_id<R>(
        &mut self,
        id: ResourceId,
        r: R,
    ) where
        R: Resource,
    {
        self.resources.insert(id, TrustCell::new(Box::new(r)));
    }

    fn remove_by_id<R>(
        &mut self,
        id: ResourceId,
    ) -> Option<R>
    where
        R: Resource,
    {
        self.resources
            .remove(&id)
            .map(TrustCell::into_inner)
            .map(|x: Box<dyn Resource>| x.downcast())
            .map(|x: Result<Box<R>, _>| x.ok().unwrap())
            .map(|x| *x)
    }

    fn unwrap_resource<R>(resource: Option<R>) -> R {
        if resource.is_none() {
            let name = core::any::type_name::<R>();
            // Tried to fetch or fetch_mut on a resource that is not registered.
            panic!("Resource not found: {}", name);
        }

        resource.unwrap()
    }

    /// Read-only fetch of a resource. Trying to get a resource that is not in the map is fatal. Use
    /// try_fetch if unsure whether the resource exists. Requesting read access to a resource that
    /// has any concurrently active writer is fatal.
    pub fn fetch<R: Resource>(&self) -> ReadBorrow<R> {
        let result = self.try_fetch();
        Self::unwrap_resource(result)
    }

    /// Read-only fetch of a resource. Requesting read access to a resource that has a concurrently
    /// active writer is fatal. Returns None if the type is not registered.
    pub fn try_fetch<R: Resource>(&self) -> Option<ReadBorrow<R>> {
        let res_id = ResourceId::new::<R>();

        self.resources.get(&res_id).map(|r| ReadBorrow {
            inner: Ref::map(r.borrow(), Box::as_ref),
            phantom: PhantomData,
        })
    }

    /// Read/Write fetch of a resource. Trying to get a resource that is not in the map is fatal. Use
    /// try_fetch if unsure whether the resource exists. Requesting write access to a resource with
    /// any concurrently active read/write is fatal
    pub fn fetch_mut<R: Resource>(&self) -> WriteBorrow<R> {
        let result = self.try_fetch_mut();
        Self::unwrap_resource(result)
    }

    /// Read/Write fetch of a resource. Requesting write access to a resource with
    /// any concurrently active read/write is fatal. Returns None if the type is not registered.
    pub fn try_fetch_mut<R: Resource>(&self) -> Option<WriteBorrow<R>> {
        let res_id = ResourceId::new::<R>();

        self.resources.get(&res_id).map(|r| WriteBorrow::<R> {
            inner: RefMut::map(r.borrow_mut(), Box::as_mut),
            phantom: PhantomData,
        })
    }

    /// Returns true if the resource is registered.
    pub fn has_value<R>(&self) -> bool
    where
        R: Resource,
    {
        self.has_value_raw(ResourceId::new::<R>())
    }

    fn has_value_raw(
        &self,
        id: ResourceId,
    ) -> bool {
        self.resources.contains_key(&id)
    }

    /// Iterate all ResourceIds within the dictionary
    pub fn keys(&self) -> impl Iterator<Item = &ResourceId> {
        self.resources.iter().map(|x| x.0)
    }
}

/// DataRequirement base trait, which underlies Read<T> and Write<T> requests
pub trait DataRequirement<'a> {
    type Borrow: DataBorrow;

    fn fetch(resource_map: &'a ResourceMap) -> Self::Borrow;
}

// Implementation for () required because we build support for (), (A), (A, B), (A, B, ...) inductively
impl<'a> DataRequirement<'a> for () {
    type Borrow = ();

    fn fetch(_: &'a ResourceMap) -> Self::Borrow {}
}

/// This type represents requesting read access to T. If T is not registered, trying to fill this
/// request will be fatal
pub struct Read<T: Resource> {
    phantom_data: PhantomData<T>,
}

/// Same as `Read`, but will return None rather than being fatal
pub type ReadOption<T> = Option<Read<T>>;

impl<'a, T: Resource> DataRequirement<'a> for Read<T> {
    type Borrow = ReadBorrow<'a, T>;

    fn fetch(resource_map: &'a ResourceMap) -> Self::Borrow {
        resource_map.fetch::<T>()
    }
}

impl<'a, T: Resource> DataRequirement<'a> for Option<Read<T>> {
    type Borrow = Option<ReadBorrow<'a, T>>;

    fn fetch(resource_map: &'a ResourceMap) -> Self::Borrow {
        resource_map.try_fetch::<T>()
    }
}

/// This type represents requesting write access to T. If T is not registered, trying to fill this
/// request will be fatal
pub struct Write<T: Resource> {
    phantom_data: PhantomData<T>,
}

/// Same as `Write`, but will return None rather than being fatal
pub type WriteOption<T> = Option<Write<T>>;

impl<'a, T: Resource> DataRequirement<'a> for Write<T> {
    type Borrow = WriteBorrow<'a, T>;

    fn fetch(resource_map: &'a ResourceMap) -> Self::Borrow {
        resource_map.fetch_mut::<T>()
    }
}

impl<'a, T: Resource> DataRequirement<'a> for Option<Write<T>> {
    type Borrow = Option<WriteBorrow<'a, T>>;

    fn fetch(resource_map: &'a ResourceMap) -> Self::Borrow {
        resource_map.try_fetch_mut::<T>()
    }
}

/// Borrow base trait. This base trait is required to allow inductively composing tuples of ReadBorrow/WriteBorrow
/// i.e. (), (A), (A, B), (A, B, ...) inductively
pub trait DataBorrow {}

// Implementation for () required because we build support for (), (A), (A, B), (A, B, ...) inductively
impl DataBorrow for () {}

/// Represents a filled read-only request for T
pub struct ReadBorrow<'a, T> {
    inner: Ref<'a, dyn Resource>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> DataBorrow for ReadBorrow<'a, T> {}
impl<'a, T> DataBorrow for Option<ReadBorrow<'a, T>> {}

impl<'a, T> std::ops::Deref for ReadBorrow<'a, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.downcast_ref().unwrap()
    }
}

impl<'a, T> Clone for ReadBorrow<'a, T> {
    fn clone(&self) -> Self {
        ReadBorrow {
            inner: self.inner.clone(),
            phantom: PhantomData,
        }
    }
}

/// Represents a filled read/write request for T
pub struct WriteBorrow<'a, T> {
    inner: RefMut<'a, dyn Resource>,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> DataBorrow for WriteBorrow<'a, T> {}
impl<'a, T> DataBorrow for Option<WriteBorrow<'a, T>> {}

impl<'a, T> std::ops::Deref for WriteBorrow<'a, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.downcast_ref().unwrap()
    }
}

impl<'a, T> std::ops::DerefMut for WriteBorrow<'a, T>
where
    T: Resource,
{
    fn deref_mut(&mut self) -> &mut T {
        self.inner.downcast_mut().unwrap()
    }
}

// This macro is used to inductively build tuples i.e. (), (A), (A, B), (A, B, ...) inductively
macro_rules! impl_data {
    ( $($ty:ident),* ) => {

        //
        // Make tuples containing DataBorrow types implement DataBorrow
        //
        impl<$($ty),*> DataBorrow for ( $( $ty , )* )
        where $( $ty : DataBorrow ),*
        {

        }

        //
        // Make tuples containing DataRequirement types implement DataBorrow. Additionally implement
        // fetch
        //
        impl<'a, $($ty),*> DataRequirement<'a> for ( $( $ty , )* )
        where $( $ty : DataRequirement<'a> ),*
        {
            type Borrow = ( $( <$ty as DataRequirement<'a>>::Borrow, )* );

            fn fetch(resource_map: &'a ResourceMap) -> Self::Borrow {
                #![allow(unused_variables)]
                ( $( <$ty as DataRequirement<'a>>::fetch(resource_map), )* )
            }
        }
    };
}

mod impl_data {
    #![cfg_attr(rustfmt, rustfmt_skip)]

    use super::*;

    // Build tuples for DataBorrow/DataRequirement i.e. (), (A), (A, B), (A, B, ...) inductively
    impl_data!(A);
    impl_data!(A, B);
    impl_data!(A, B, C);
    impl_data!(A, B, C, D);
    impl_data!(A, B, C, D, E);
    impl_data!(A, B, C, D, E, F);
    impl_data!(A, B, C, D, E, F, G);
    impl_data!(A, B, C, D, E, F, G, H);
    impl_data!(A, B, C, D, E, F, G, H, I);
    impl_data!(A, B, C, D, E, F, G, H, I, J);
    // May be extended as needed, but this seems like enough
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
    // impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
}
