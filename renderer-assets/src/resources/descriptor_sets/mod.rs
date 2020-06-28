use fnv::FnvHashMap;
use crate::vk_description as dsc;

mod descriptor_set_arc;
pub use descriptor_set_arc::DescriptorSetArc;

mod dynamic_descriptor_sets;
pub use dynamic_descriptor_sets::DynDescriptorSet;
pub use dynamic_descriptor_sets::DynPassMaterialInstance;
pub use dynamic_descriptor_sets::DynMaterialInstance;

mod descriptor_set_pool;
use descriptor_set_pool::ManagedDescriptorSetPool;

mod descriptor_set_pool_chunk;
use descriptor_set_pool_chunk::ManagedDescriptorSetPoolChunk;

mod descriptor_set_buffers;
use descriptor_set_buffers::DescriptorSetPoolRequiredBufferInfo;
use descriptor_set_buffers::DescriptorLayoutBufferSet;

mod descriptor_write_set;
pub use descriptor_write_set::DescriptorSetWriteElementImage;
pub use descriptor_write_set::DescriptorSetWriteElementBufferDataBufferRef;
pub use descriptor_write_set::DescriptorSetWriteElementBufferData;
pub use descriptor_write_set::DescriptorSetWriteElementBuffer;
pub use descriptor_write_set::DescriptorSetElementWrite;
pub use descriptor_write_set::DescriptorSetElementKey;
pub use descriptor_write_set::DescriptorSetWriteSet;
pub use descriptor_write_set::create_uninitialized_write_set_for_layout;
pub use descriptor_write_set::create_uninitialized_write_sets_for_material_pass;
pub use descriptor_write_set::create_write_sets_for_material_instance_pass;
pub use descriptor_write_set::apply_material_instance_slot_assignment;

mod descriptor_set_allocator;
pub use descriptor_set_allocator::DescriptorSetAllocator;
pub use descriptor_set_allocator::DescriptorSetAllocatorMetrics;
pub use descriptor_set_allocator::DescriptorSetPoolMetrics;

mod descriptor_set_allocator_manager;
pub(super) use descriptor_set_allocator_manager::DescriptorSetAllocatorManager;
pub use descriptor_set_allocator_manager::DescriptorSetAllocatorRef;
pub use descriptor_set_allocator_manager::DescriptorSetAllocatorProvider;

const MAX_DESCRIPTORS_PER_POOL: u32 = 64;
const MAX_FRAMES_IN_FLIGHT: usize = renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT;
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
    bind_samplers: bool,
    bind_images: bool,
    bind_buffers: bool,
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
        _ => unimplemented!(),
    }

    what
}
