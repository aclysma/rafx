use std::hash::Hash;

/// An opaque 64-bit handle used as a unique identifier for objects in the game world
/// that are visible or otherwise relevant to the renderer's pipeline. Each `VisibilityObject`
/// is associated with a specific `ObjectId`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ObjectId(u64);

impl Into<u64> for ObjectId {
    fn into(self) -> u64 {
        self.0
    }
}

impl ObjectId {
    pub fn new(id: u64) -> Self {
        ObjectId(id)
    }

    pub fn from<T: 'static + Copy + Hash + Eq + PartialEq>(value: T) -> Self {
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<u64>());
        let id = unsafe { std::mem::transmute_copy(&value) };
        ObjectId::new(id)
    }

    pub fn into<T: 'static + Copy + Hash + Eq + PartialEq>(self) -> T {
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<u64>());
        unsafe { std::mem::transmute_copy(&self.0) }
    }
}
