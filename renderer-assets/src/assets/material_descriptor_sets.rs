use super::SlotNameLookup;
use ash::prelude::VkResult;
use renderer_resources::DescriptorSetAllocator;
use renderer_resources::DynDescriptorSet;
use renderer_resources::ImageViewResource;
use renderer_resources::ResourceArc;
use std::sync::Arc;

pub struct DynPassMaterialInstance {
    descriptor_sets: Vec<DynDescriptorSet>,
    slot_name_lookup: Arc<SlotNameLookup>,
}

impl DynPassMaterialInstance {
    pub fn new(
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

    pub fn flush(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
    ) -> VkResult<()> {
        for set in &mut self.descriptor_sets {
            set.flush(descriptor_set_allocator)?
        }

        Ok(())
    }

    pub fn set_image(
        &mut self,
        slot_name: &str,
        image_view: &ResourceArc<ImageViewResource>,
    ) {
        if let Some(slot_locations) = self.slot_name_lookup.get(slot_name) {
            for slot_location in slot_locations {
                if let Some(dyn_descriptor_set) = self
                    .descriptor_sets
                    .get_mut(slot_location.layout_index as usize)
                {
                    dyn_descriptor_set.set_image(slot_location.binding_index, image_view);
                }
            }
        }
    }

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    pub fn set_buffer_data<T: Copy + 'static>(
        &mut self,
        slot_name: &str,
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
    pub fn new(passes: Vec<DynPassMaterialInstance>) -> Self {
        DynMaterialInstance { passes }
    }

    pub fn pass(
        &self,
        pass_index: u32,
    ) -> &DynPassMaterialInstance {
        &self.passes[pass_index as usize]
    }

    pub fn flush(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
    ) -> VkResult<()> {
        for pass in &mut self.passes {
            pass.flush(descriptor_set_allocator)?
        }

        Ok(())
    }

    pub fn set_image(
        &mut self,
        slot_name: &str,
        image_view: &ResourceArc<ImageViewResource>,
    ) {
        for pass in &mut self.passes {
            pass.set_image(slot_name, image_view)
        }
    }

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    pub fn set_buffer_data<T: Copy + 'static>(
        &mut self,
        slot_name: &str,
        data: &T,
    ) {
        for pass in &mut self.passes {
            pass.set_buffer_data(slot_name, data)
        }
    }
}
