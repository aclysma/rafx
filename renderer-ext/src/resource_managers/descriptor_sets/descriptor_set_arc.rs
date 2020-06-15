use ash::vk;
use super::RegisteredDescriptorSet;
use renderer_base::slab::RawSlabKey;
use crossbeam_channel::Sender;
use std::fmt::Formatter;
use std::sync::Arc;
use crate::resource_managers::ResourceManager;

//
// Reference counting mechanism to keep descriptor sets allocated
//

// Data internal to the DescriptorSetArc
pub(super) struct DescriptorSetArcInner {
    // Unique ID of the descriptor set
    pub(super) slab_key: RawSlabKey<RegisteredDescriptorSet>,

    // We can't cache a single vk::DescriptorSet here because the correct one to use will be
    // dependent on the current frame in flight index. But to make lookups fast, we can cache the
    // three possible descriptor sets
    pub(super) descriptor_sets_per_frame: Vec<vk::DescriptorSet>,

    // When this object is dropped, send a message to the pool to deallocate this descriptor set
    drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>,
}

impl std::fmt::Debug for DescriptorSetArcInner {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DescriptorSetArcInner")
            .field("slab_key", &self.slab_key)
            .finish()
    }
}

#[derive(Clone)]
pub struct DescriptorSetArc {
    pub(super) inner: Arc<DescriptorSetArcInner>,
}

impl DescriptorSetArc {
    pub(super) fn new(
        slab_key: RawSlabKey<RegisteredDescriptorSet>,
        descriptor_sets_per_frame: Vec<vk::DescriptorSet>,
        drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>,
    ) -> Self {
        let inner = DescriptorSetArcInner {
            slab_key,
            descriptor_sets_per_frame,
            drop_tx,
        };

        DescriptorSetArc {
            inner: Arc::new(inner),
        }
    }

    pub fn get_raw_for_cpu_write(
        &self,
        resource_manager: &ResourceManager,
    ) -> vk::DescriptorSet {
        //self.inner.descriptor_sets_per_frame[resource_manager.registered_descriptor_sets.frame_in_flight_index as usize]
        resource_manager
            .registered_descriptor_sets
            .descriptor_set_for_cpu_write(self)
    }

    pub fn get_raw_for_gpu_read(
        &self,
        resource_manager: &ResourceManager,
    ) -> vk::DescriptorSet {
        //self.inner.descriptor_sets_per_frame[resource_manager.registered_descriptor_sets.frame_in_flight_index as usize]
        resource_manager
            .registered_descriptor_sets
            .descriptor_set_for_gpu_read(self)
    }
}

impl std::fmt::Debug for DescriptorSetArc {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DescriptorSetArc")
            .field("inner", &self.inner)
            .finish()
    }
}
