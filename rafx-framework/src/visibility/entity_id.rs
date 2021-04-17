#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct EntityId(u64);

impl Into<u64> for EntityId {
    fn into(self) -> u64 {
        self.0
    }
}

impl EntityId {
    pub fn new(id: u64) -> Self {
        EntityId(id)
    }

    pub fn from<T: Copy>(value: T) -> Self {
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<u64>());
        let id = unsafe { std::mem::transmute_copy(&value) };
        EntityId::new(id)
    }

    pub fn into<T: Copy>(self) -> T {
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<u64>());
        unsafe { std::mem::transmute_copy(&self.0) }
    }
}
