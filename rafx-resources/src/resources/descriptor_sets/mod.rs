use crate::vk_description as dsc;
use fnv::FnvHashMap;

mod descriptor_set_arc;
pub use descriptor_set_arc::DescriptorSetArc;

mod dynamic_descriptor_sets;
pub use dynamic_descriptor_sets::DynDescriptorSet;

mod descriptor_set_pool;
use descriptor_set_pool::ManagedDescriptorSetPool;

mod descriptor_set_pool_chunk;
use descriptor_set_pool_chunk::ManagedDescriptorSetPoolChunk;

mod descriptor_set_buffers;
use descriptor_set_buffers::DescriptorLayoutBufferSet;
use descriptor_set_buffers::DescriptorSetPoolRequiredBufferInfo;

mod descriptor_write_set;
pub use descriptor_write_set::create_uninitialized_write_set_for_layout;
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
pub(super) use descriptor_set_allocator_manager::DescriptorSetAllocatorManager;
pub use descriptor_set_allocator_manager::DescriptorSetAllocatorProvider;
pub use descriptor_set_allocator_manager::DescriptorSetAllocatorRef;

const MAX_DESCRIPTORS_PER_POOL: u32 = 64;
const MAX_FRAMES_IN_FLIGHT: usize = rafx_shell_vulkan::MAX_FRAMES_IN_FLIGHT;
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
        dsc::DescriptorType::Sampler => {
            what.bind_samplers = !element_write.has_immutable_sampler;
        }
        dsc::DescriptorType::CombinedImageSampler => {
            what.bind_samplers = !element_write.has_immutable_sampler;
            what.bind_images = true;
        }
        dsc::DescriptorType::SampledImage => {
            what.bind_images = true;
        }
        dsc::DescriptorType::UniformBuffer => {
            what.bind_buffers = true;
        }
        dsc::DescriptorType::StorageBuffer => {
            what.bind_buffers = true;
        }
        _ => unimplemented!(),
    }

    what
}
