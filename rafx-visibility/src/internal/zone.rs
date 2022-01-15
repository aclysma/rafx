use crate::frustum_culling::PackedBoundingSphereChunk;
use crate::internal::Volume;
use crate::{VisibilityObjectHandle, VolumeHandle};
use slotmap::{SecondaryMap, SlotMap};

pub struct Zone {
    pub(crate) chunks: Vec<PackedBoundingSphereChunk>,
    pub(crate) objects: SecondaryMap<VisibilityObjectHandle, (usize, usize)>,
    pub(crate) volumes: SlotMap<VolumeHandle, Volume>,
}

impl Zone {
    pub fn new() -> Self {
        Zone {
            chunks: Default::default(),
            objects: Default::default(),
            volumes: Default::default(),
        }
    }
}
