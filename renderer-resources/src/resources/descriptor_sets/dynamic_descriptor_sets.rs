use super::DescriptorSetArc;
use super::DescriptorSetElementKey;
use super::DescriptorSetWriteSet;
use crate::resources::descriptor_sets::descriptor_write_set::{
    DescriptorSetWriteElementBufferData, DescriptorSetWriteElementImageValue,
};
use crate::resources::descriptor_sets::DescriptorSetAllocator;
use crate::resources::resource_lookup::{DescriptorSetLayoutResource, ImageViewResource};
use crate::resources::ResourceArc;
use ash::prelude::VkResult;
use std::fmt::Formatter;

//TODO: Create a builder that is not initialized, this will help avoid forgetting to call flush
// as well as prevent double-allocating (allocating a descriptor set based on a material instance
// just to immediately modify one part of it and reallocate it)
pub struct DynDescriptorSet {
    // Hash to the descriptor set layout. We use the hash to quickly look up the layout and we
    // assume the pool for the layout will already exist in the descriptor set manager
    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,

    // The actual descriptor set
    descriptor_set: DescriptorSetArc,

    // A full copy of the data the descriptor set has been assigned
    write_set: DescriptorSetWriteSet,

    // As we add modifications to the set, we will insert them here. They are merged with write_set
    // when we finally flush the descriptor set
    pending_write_set: DescriptorSetWriteSet,

    has_been_flushed: bool,
}

impl std::fmt::Debug for DynDescriptorSet {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DynDescriptorSet")
            .field("descriptor_set", &self.descriptor_set)
            .finish()
    }
}

impl Drop for DynDescriptorSet {
    fn drop(&mut self) {
        if !self.has_been_flushed {
            panic!("A descriptor set was dropped without being flushed");
        }
    }
}

impl DynDescriptorSet {
    pub(super) fn new(
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        descriptor_set: DescriptorSetArc,
        write_set: DescriptorSetWriteSet,
    ) -> Self {
        DynDescriptorSet {
            descriptor_set_layout: descriptor_set_layout.clone(),
            descriptor_set,
            write_set,
            pending_write_set: Default::default(),
            has_been_flushed: false,
        }
    }

    pub fn descriptor_set(&self) -> &DescriptorSetArc {
        &self.descriptor_set
    }

    //TODO: Make a commit-like API so that it's not so easy to forget to call flush
    pub fn flush(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
    ) -> VkResult<()> {
        if !self.pending_write_set.elements.is_empty() {
            let mut pending_write_set = Default::default();
            std::mem::swap(&mut pending_write_set, &mut self.pending_write_set);

            self.write_set.copy_from(&pending_write_set);

            // create it
            let new_descriptor_set = descriptor_set_allocator.create_descriptor_set_with_writes(
                &self.descriptor_set_layout,
                pending_write_set,
            )?;

            log::trace!(
                "DynDescriptorSet::flush {:?} -> {:?}",
                self.descriptor_set,
                new_descriptor_set
            );
            self.descriptor_set = new_descriptor_set;
        }

        self.has_been_flushed = true;
        Ok(())
    }

    pub fn set_image(
        &mut self,
        binding_index: u32,
        image_view: &ResourceArc<ImageViewResource>,
    ) {
        self.set_image_array_element(
            binding_index,
            0,
            DescriptorSetWriteElementImageValue::Resource(image_view.clone()),
        )
    }

    pub fn set_images(
        &mut self,
        binding_index: u32,
        image_views: &[Option<&ResourceArc<ImageViewResource>>],
    ) {
        for (index, image_view) in image_views.iter().enumerate() {
            if let Some(image_view) = image_view.as_ref() {
                self.set_image_array_element(
                    binding_index,
                    index,
                    DescriptorSetWriteElementImageValue::Resource((*image_view).clone()),
                )
            }
        }
    }

    pub fn set_image_at_index(
        &mut self,
        binding_index: u32,
        array_index: usize,
        image_view: &ResourceArc<ImageViewResource>,
    ) {
        self.set_image_array_element(
            binding_index,
            array_index,
            DescriptorSetWriteElementImageValue::Resource(image_view.clone()),
        )
    }

    fn set_image_array_element(
        &mut self,
        binding_index: u32,
        array_index: usize,
        image_view: DescriptorSetWriteElementImageValue,
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

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    pub fn set_buffer_data<T: Copy + 'static>(
        &mut self,
        binding_index: u32,
        data: &T,
    ) {
        self.set_buffer_data_array_element(binding_index, 0, data)
    }

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    pub fn set_buffer_data_at_index<T: Copy + 'static>(
        &mut self,
        binding_index: u32,
        array_index: usize,
        data: &T,
    ) {
        self.set_buffer_data_array_element(binding_index, array_index, data)
    }

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    fn set_buffer_data_array_element<T: Copy + 'static>(
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
                if let Some(element_image) = element.buffer_info.get_mut(array_index) {
                    element_image.buffer = Some(DescriptorSetWriteElementBufferData::Data(data));
                    self.pending_write_set.elements.insert(key, element.clone());
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
