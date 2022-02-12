use fnv::FnvHashMap;

mod descriptor_set_layout;
pub use descriptor_set_layout::DescriptorSetLayout;
pub use descriptor_set_layout::DescriptorSetLayoutBinding;

mod descriptor_set_arc;
pub use descriptor_set_arc::DescriptorSetArc;

mod dynamic_descriptor_sets;
pub use dynamic_descriptor_sets::DescriptorSetBindings;
pub use dynamic_descriptor_sets::DynDescriptorSet;

mod descriptor_set_pool;
use descriptor_set_pool::ManagedDescriptorSetPool;

mod descriptor_set_pool_chunk;
pub use descriptor_set_pool_chunk::DescriptorSetWriter;
pub use descriptor_set_pool_chunk::DescriptorSetWriterContext;
use descriptor_set_pool_chunk::ManagedDescriptorSetPoolChunk;

mod descriptor_set_buffers;
use descriptor_set_buffers::DescriptorLayoutBufferSet;
use descriptor_set_buffers::DescriptorSetPoolRequiredBufferInfo;

mod descriptor_write_set;
pub use descriptor_write_set::create_uninitialized_write_set_for_layout;
pub use descriptor_write_set::DescriptorSetBindingKey;
pub use descriptor_write_set::DescriptorSetElementKey;
pub use descriptor_write_set::DescriptorSetElementWrite;
pub use descriptor_write_set::DescriptorSetWriteElementBuffer;
pub use descriptor_write_set::DescriptorSetWriteElementBufferData;
pub use descriptor_write_set::DescriptorSetWriteElementBufferDataBufferRef;
pub use descriptor_write_set::DescriptorSetWriteElementImage;
pub use descriptor_write_set::DescriptorSetWriteElementImageValue;
pub use descriptor_write_set::DescriptorSetWriteSet;

mod descriptor_set_allocator;
pub use descriptor_set_allocator::DescriptorSetAllocator;
pub use descriptor_set_allocator::DescriptorSetAllocatorMetrics;
pub use descriptor_set_allocator::DescriptorSetInitializer;
pub use descriptor_set_allocator::DescriptorSetPoolMetrics;

mod descriptor_set_allocator_manager;
use crate::{DescriptorSetLayoutResource, ResourceArc};
pub(super) use descriptor_set_allocator_manager::DescriptorSetAllocatorManager;
pub use descriptor_set_allocator_manager::DescriptorSetAllocatorProvider;
pub use descriptor_set_allocator_manager::DescriptorSetAllocatorRef;
use rafx_api::RafxResourceType;

const MAX_FRAMES_IN_FLIGHT: usize = crate::MAX_FRAMES_IN_FLIGHT;
const MAX_FRAMES_IN_FLIGHT_PLUS_1: usize = MAX_FRAMES_IN_FLIGHT + 1;

// A set of write to buffers that back a descriptor set
#[derive(Debug, Default, Clone)]
pub struct DescriptorSetWriteBuffer {
    pub elements: FnvHashMap<DescriptorSetElementKey, Vec<u8>>,
}

// Slab keys to identify descriptors can carry a payload. Anything we'd want to store per descriptor
// set can go here, but don't have anything yet
struct ManagedDescriptorSet {
    //write_set: DescriptorSetWriteSet,
}

// We need to delay dropping descriptor sets for MAX_FRAMES_IN_FLIGHT frames
type FrameInFlightIndex = u32;

fn add_to_frame_in_flight_index(
    index: FrameInFlightIndex,
    value: u32,
) -> FrameInFlightIndex {
    (index + value) % MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32
}

// fn subtract_from_frame_in_flight_index(
//     index: FrameInFlightIndex,
//     value: u32,
// ) -> FrameInFlightIndex {
//     (value + MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32 - index) % MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32
// }

#[derive(Default, Debug)]
pub struct WhatToBind {
    pub bind_samplers: bool,
    pub bind_images: bool,
    pub bind_buffers: bool,
}

pub fn what_to_bind(element_write: &DescriptorSetElementWrite) -> WhatToBind {
    let mut what = WhatToBind::default();

    // See https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkWriteDescriptorSet.html
    match element_write.descriptor_type {
        RafxResourceType::SAMPLER => {
            what.bind_samplers = !element_write.has_immutable_sampler;
        }
        RafxResourceType::COMBINED_IMAGE_SAMPLER => {
            what.bind_samplers = !element_write.has_immutable_sampler;
            what.bind_images = true;
        }
        RafxResourceType::TEXTURE => {
            what.bind_images = true;
        }
        RafxResourceType::TEXTURE_READ_WRITE => {
            what.bind_images = true;
        }
        RafxResourceType::UNIFORM_BUFFER => {
            what.bind_buffers = true;
        }
        RafxResourceType::BUFFER => {
            what.bind_buffers = true;
        }
        RafxResourceType::BUFFER_READ_WRITE => {
            what.bind_buffers = true;
        }
        _ => {
            unimplemented!(
                "what_to_bind not implemented for descriptor type {:?}",
                element_write.descriptor_type
            );
        }
    }

    what
}

pub fn get_descriptor_set_element_write(
    descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
    key: &DescriptorSetElementKey,
) -> Option<DescriptorSetElementWrite> {
    for binding in &descriptor_set_layout
        .get_raw()
        .descriptor_set_layout_def
        .bindings
    {
        let element_count = binding.resource.element_count_normalized() as usize;
        if key.dst_binding != binding.resource.binding || key.array_index >= element_count {
            continue;
        }

        return Some(DescriptorSetElementWrite {
            has_immutable_sampler: binding.immutable_samplers.is_some(),
            descriptor_type: binding.resource.resource_type,
            image_info: DescriptorSetWriteElementImage::default(),
            buffer_info: DescriptorSetWriteElementBuffer::default(),
        });
    }

    None
}
