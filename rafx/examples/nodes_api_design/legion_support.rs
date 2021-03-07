use legion::Resources;
use legion::World;
use rafx_base::memory::force_to_static_lifetime;
use std::ops::Deref;

//TODO: Add a render resource builder or something that lets someone add extract-only things by
// ref and resources available to all jobs by passing ownership

// static reference is dangerous, must only be used when extracting
pub struct LegionWorld(&'static World);

impl LegionWorld {
    pub unsafe fn new(world: &World) -> Self {
        LegionWorld(force_to_static_lifetime(world))
    }
}

impl Deref for LegionWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

// Safe because we do not mutate legion world
unsafe impl Send for LegionWorld {}
unsafe impl Sync for LegionWorld {}

// static reference is dangerous, must only be used when extracting
pub struct LegionResources(&'static Resources);

impl LegionResources {
    pub unsafe fn new(resources: &Resources) -> Self {
        LegionResources(force_to_static_lifetime(resources))
    }
}

impl Deref for LegionResources {
    type Target = Resources;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

// Safe because we do not mutate legion resources
unsafe impl Send for LegionResources {}
unsafe impl Sync for LegionResources {}
