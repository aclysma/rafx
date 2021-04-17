use crate::frustum_culling::PackedBoundingSphereChunk;
use crate::internal::Volume;
use crate::{ObjectHandle, VolumeHandle};
use parking_lot::RwLock;
use slotmap::{SecondaryMap, SlotMap};
use std::sync::Arc;

pub struct Zone {
    pub(crate) chunks: Arc<RwLock<Vec<PackedBoundingSphereChunk>>>,
    pub(crate) objects: SecondaryMap<ObjectHandle, (usize, usize)>,
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
