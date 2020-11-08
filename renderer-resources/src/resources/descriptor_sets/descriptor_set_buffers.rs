use crate::resources::descriptor_sets::{DescriptorSetElementKey, MAX_DESCRIPTORS_PER_POOL};
use crate::vk_description as dsc;
use ash::prelude::*;
use ash::vk;
use fnv::FnvHashMap;
use renderer_shell_vulkan::{VkBuffer, VkDeviceContext};
use std::mem::ManuallyDrop;

//
// Metadata about a buffer for a particular descriptor in a descriptor layout
//
#[derive(Clone)]
pub(super) struct DescriptorSetPoolRequiredBufferInfo {
    pub(super) dst_element: DescriptorSetElementKey,
    pub(super) descriptor_type: dsc::DescriptorType,
    pub(super) per_descriptor_size: u32,
    pub(super) per_descriptor_stride: u32,
}

//
// Creates and manages the internal buffers for a single binding within a descriptor pool chunk
//
pub(super) struct DescriptorBindingBufferSet {
    pub(super) buffer: ManuallyDrop<VkBuffer>,
    pub(super) buffer_info: DescriptorSetPoolRequiredBufferInfo,
}

impl DescriptorBindingBufferSet {
    fn new(
        device_context: &VkDeviceContext,
        buffer_info: &DescriptorSetPoolRequiredBufferInfo,
    ) -> VkResult<Self> {
        //This is the only one we support right now
        assert!(buffer_info.descriptor_type == dsc::DescriptorType::UniformBuffer);

        let buffer = VkBuffer::new(
            device_context,
            renderer_shell_vulkan::vk_mem::MemoryUsage::CpuToGpu,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            (buffer_info.per_descriptor_stride * MAX_DESCRIPTORS_PER_POOL) as u64,
        )?;

        Ok(DescriptorBindingBufferSet {
            buffer: ManuallyDrop::new(buffer),
            buffer_info: buffer_info.clone(),
        })
    }
}

//
// Creates and manages the internal buffers for a descriptor pool chunk
//
pub(super) struct DescriptorLayoutBufferSet {
    pub(super) buffer_sets: FnvHashMap<DescriptorSetElementKey, DescriptorBindingBufferSet>,
}

impl DescriptorLayoutBufferSet {
    pub(super) fn new(
        device_context: &VkDeviceContext,
        buffer_infos: &[DescriptorSetPoolRequiredBufferInfo],
    ) -> VkResult<Self> {
        let mut buffer_sets: FnvHashMap<DescriptorSetElementKey, DescriptorBindingBufferSet> =
            Default::default();
        for buffer_info in buffer_infos {
            let buffer = DescriptorBindingBufferSet::new(device_context, &buffer_info)?;
            buffer_sets.insert(buffer_info.dst_element, buffer);
        }

        Ok(DescriptorLayoutBufferSet { buffer_sets })
    }
}
