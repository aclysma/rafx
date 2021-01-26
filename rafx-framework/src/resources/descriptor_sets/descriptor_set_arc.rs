use super::ManagedDescriptorSet;
use crate::{DescriptorSetLayoutResource, ResourceArc};
use crossbeam_channel::Sender;
use rafx_api::{RafxCommandBuffer, RafxDescriptorSetHandle, RafxResult};
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

    // When this object is dropped, send a message to the pool to deallocate this descriptor set
    drop_tx: Sender<RawSlabKey<ManagedDescriptorSet>>,

    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    handle: RafxDescriptorSetHandle,
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
        drop_tx: Sender<RawSlabKey<ManagedDescriptorSet>>,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        handle: RafxDescriptorSetHandle,
    ) -> Self {
        let inner = DescriptorSetArcInner {
            slab_key,
            drop_tx,
            descriptor_set_layout: descriptor_set_layout.clone(),
            handle,
        };

        DescriptorSetArc {
            inner: Arc::new(inner),
        }
    }

    pub fn handle(&self) -> &RafxDescriptorSetHandle {
        &self.inner.handle
    }

    pub fn layout(&self) -> &ResourceArc<DescriptorSetLayoutResource> {
        &self.inner.descriptor_set_layout
    }

    pub fn bind(
        &self,
        command_buffer: &RafxCommandBuffer,
    ) -> RafxResult<()> {
        let descriptor_set_layout = &self.inner.descriptor_set_layout.get_raw();
        command_buffer.cmd_bind_descriptor_set_handle(
            &descriptor_set_layout.root_signature,
            descriptor_set_layout.set_index,
            &self.inner.handle,
        )
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
