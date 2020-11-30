use super::ManagedDescriptorSet;
use ash::vk;
use crossbeam_channel::Sender;
use rafx_base::slab::RawSlabKey;
use std::fmt::Formatter;
use std::sync::Arc;

//
// Reference counting mechanism to keep descriptor sets allocated
//

// Data internal to the DescriptorSetArc
pub(super) struct DescriptorSetArcInner {
    // Unique ID of the descriptor set
    pub(super) slab_key: RawSlabKey<ManagedDescriptorSet>,

    // Cache the raw descriptor set here
    pub(super) descriptor_set: vk::DescriptorSet,

    // When this object is dropped, send a message to the pool to deallocate this descriptor set
    drop_tx: Sender<RawSlabKey<ManagedDescriptorSet>>,
}

impl Drop for DescriptorSetArcInner {
    fn drop(&mut self) {
        self.drop_tx.send(self.slab_key).unwrap();
    }
}

impl std::fmt::Debug for DescriptorSetArcInner {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DescriptorSetArcInner")
            .field("slab_key", &self.slab_key)
            .field("raw", &self.descriptor_set)
            .finish()
    }
}

#[derive(Clone)]
pub struct DescriptorSetArc {
    pub(super) inner: Arc<DescriptorSetArcInner>,
}

impl DescriptorSetArc {
    pub(super) fn new(
        slab_key: RawSlabKey<ManagedDescriptorSet>,
        descriptor_set: vk::DescriptorSet,
        drop_tx: Sender<RawSlabKey<ManagedDescriptorSet>>,
    ) -> Self {
        let inner = DescriptorSetArcInner {
            slab_key,
            descriptor_set,
            drop_tx,
        };

        DescriptorSetArc {
            inner: Arc::new(inner),
        }
    }

    pub fn get(&self) -> vk::DescriptorSet {
        self.inner.descriptor_set
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
