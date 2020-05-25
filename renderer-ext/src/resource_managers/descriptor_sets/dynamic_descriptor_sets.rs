use super::DescriptorSetArc;
use super::DescriptorSetWriteSet;
use super::DescriptorSetWriteBuffer;
use super::SlabKeyDescriptorSetWriteSet;
use super::SlabKeyDescriptorSetWriteBuffer;
use super::DescriptorSetElementKey;
use crate::resource_managers::resource_lookup::{ResourceArc, ImageViewResource};
use crate::resource_managers::asset_lookup::SlotNameLookup;
use crossbeam_channel::Sender;
use std::sync::Arc;

pub struct DynDescriptorSet {
    descriptor_set: DescriptorSetArc,
    write_set: DescriptorSetWriteSet,

    write_set_tx: Sender<SlabKeyDescriptorSetWriteSet>,
    write_buffer_tx: Sender<SlabKeyDescriptorSetWriteBuffer>,

    //dirty: FnvHashSet<DescriptorSetElementKey>,
    pending_write_set: DescriptorSetWriteSet,
    pending_write_buffer: DescriptorSetWriteBuffer,
}

impl DynDescriptorSet {
    pub(super) fn new(
        write_set: DescriptorSetWriteSet,
        descriptor_set: DescriptorSetArc,
        write_set_tx: Sender<SlabKeyDescriptorSetWriteSet>,
        write_buffer_tx: Sender<SlabKeyDescriptorSetWriteBuffer>,
    ) -> Self {
        DynDescriptorSet {
            descriptor_set,
            write_set,
            write_set_tx,
            write_buffer_tx,
            //dirty: Default::default(),
            pending_write_set: Default::default(),
            pending_write_buffer: Default::default(),
        }
    }

    pub fn descriptor_set(&self) -> &DescriptorSetArc {
        &self.descriptor_set
    }

    //TODO: Make a commit-like API so that it's not so easy to forget to call flush
    pub fn flush(&mut self) {
        if !self.pending_write_set.elements.is_empty() {
            let mut pending_write_set = Default::default();
            std::mem::swap(&mut pending_write_set, &mut self.pending_write_set);

            let pending_descriptor_set_write = SlabKeyDescriptorSetWriteSet {
                write_set: pending_write_set,
                slab_key: self.descriptor_set.inner.slab_key,
            };

            log::trace!("Sending a set write");
            self.write_set_tx.send(pending_descriptor_set_write);
        }

        if !self.pending_write_buffer.elements.is_empty() {
            let mut pending_write_buffer = Default::default();
            std::mem::swap(&mut pending_write_buffer, &mut self.pending_write_buffer);

            let pending_descriptor_set_write = SlabKeyDescriptorSetWriteBuffer {
                write_buffer: pending_write_buffer,
                slab_key: self.descriptor_set.inner.slab_key,
            };

            log::trace!("Sending a buffer write");
            self.write_buffer_tx.send(pending_descriptor_set_write);
        }
    }

    pub fn set_image(
        &mut self,
        binding_index: u32,
        image_view: ResourceArc<ImageViewResource>,
    ) {
        self.set_image_array_element(binding_index, 0, image_view)
    }

    pub fn set_image_array_element(
        &mut self,
        binding_index: u32,
        array_index: usize,
        image_view: ResourceArc<ImageViewResource>,
    ) {
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            //dst_array_element: 0
        };

        if let Some(element) = self.write_set.elements.get_mut(&key) {
            let what_to_bind = super::what_to_bind(element);
            if what_to_bind.bind_images {
                if let Some(element_image) = element.image_info.get_mut(array_index) {
                    element_image.image_view = Some(image_view);

                    self.pending_write_set.elements.insert(key, element.clone());

                //self.dirty.insert(key);
                } else {
                    log::warn!("Tried to set image index {} but it did not exist. The image array is {} elements long.", array_index, element.image_info.len());
                }
            } else {
                // This is not necessarily an error if the user is binding with a slot name (although not sure
                // if that's the right approach long term)
                //log::warn!("Tried to bind an image to a descriptor set where the type does not accept an image", array_index)
            }
        } else {
            log::warn!("Tried to set image on a binding index that does not exist");
        }
    }

    pub fn set_buffer_data<T: Copy>(
        &mut self,
        binding_index: u32,
        data: &T,
    ) {
        self.set_buffer_data_array_element(binding_index, 0, data)
    }

    fn set_buffer_data_array_element<T: Copy>(
        &mut self,
        binding_index: u32,
        array_index: usize,
        data: &T,
    ) {
        //TODO: Verify that T's size matches the buffer

        // Not supporting array indices yet
        assert!(array_index == 0);
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            //dst_array_element: 0
        };

        if let Some(element) = self.write_set.elements.get_mut(&key) {
            let what_to_bind = super::what_to_bind(element);
            if what_to_bind.bind_buffers {
                let data = renderer_shell_vulkan::util::any_as_bytes(data).into();
                if element.buffer_info.len() > array_index {
                    self.pending_write_buffer.elements.insert(key, data);
                } else {
                    log::warn!("Tried to set buffer data for index {} but it did not exist. The buffer array is {} elements long.", array_index, element.buffer_info.len());
                }
            } else {
                // This is not necessarily an error if the user is binding with a slot name (although not sure
                // if that's the right approach long term)
                //log::warn!("Tried to bind an image to a descriptor set where the type does not accept an image", array_index)
            }
        } else {
            log::warn!("Tried to set buffer data on a binding index that does not exist");
        }
    }
}

pub struct DynPassMaterialInstance {
    descriptor_sets: Vec<DynDescriptorSet>,
    slot_name_lookup: Arc<SlotNameLookup>,
}

impl DynPassMaterialInstance {
    pub(super) fn new(
        descriptor_sets: Vec<DynDescriptorSet>,
        slot_name_lookup: Arc<SlotNameLookup>,
    ) -> Self {
        DynPassMaterialInstance {
            descriptor_sets,
            slot_name_lookup,
        }
    }

    pub fn descriptor_set_layout(
        &self,
        layout_index: u32,
    ) -> &DynDescriptorSet {
        &self.descriptor_sets[layout_index as usize]
    }

    pub fn flush(&mut self) {
        for set in &mut self.descriptor_sets {
            set.flush()
        }
    }

    pub fn set_image(
        &mut self,
        slot_name: &String,
        image_view: ResourceArc<ImageViewResource>,
    ) {
        if let Some(slot_locations) = self.slot_name_lookup.get(slot_name) {
            for slot_location in slot_locations {
                if let Some(dyn_descriptor_set) = self
                    .descriptor_sets
                    .get_mut(slot_location.layout_index as usize)
                {
                    dyn_descriptor_set.set_image(slot_location.binding_index, image_view.clone());
                }
            }
        }
    }

    pub fn set_buffer_data<T: Copy>(
        &mut self,
        slot_name: &String,
        data: &T,
    ) {
        if let Some(slot_locations) = self.slot_name_lookup.get(slot_name) {
            for slot_location in slot_locations {
                if let Some(dyn_descriptor_set) = self
                    .descriptor_sets
                    .get_mut(slot_location.layout_index as usize)
                {
                    dyn_descriptor_set.set_buffer_data(slot_location.binding_index, data);
                }
            }
        }
    }
}

pub struct DynMaterialInstance {
    passes: Vec<DynPassMaterialInstance>,
}

impl DynMaterialInstance {
    pub(super) fn new(passes: Vec<DynPassMaterialInstance>) -> Self {
        DynMaterialInstance { passes }
    }

    pub fn pass(
        &self,
        pass_index: u32,
    ) -> &DynPassMaterialInstance {
        &self.passes[pass_index as usize]
    }

    pub fn flush(&mut self) {
        for pass in &mut self.passes {
            pass.flush()
        }
    }

    pub fn set_image(
        &mut self,
        slot_name: &String,
        image_view: &ResourceArc<ImageViewResource>,
    ) {
        for pass in &mut self.passes {
            pass.set_image(slot_name, image_view.clone())
        }
    }

    pub fn set_buffer_data<T: Copy>(
        &mut self,
        slot_name: &String,
        data: &T,
    ) {
        for pass in &mut self.passes {
            pass.set_buffer_data(slot_name, data)
        }
    }
}
