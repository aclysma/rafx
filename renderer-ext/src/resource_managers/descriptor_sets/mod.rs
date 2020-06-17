use ash::vk;
use renderer_base::slab::{RawSlabKey, RawSlab};
use crossbeam_channel::{Sender, Receiver};
use std::fmt::Formatter;
use std::sync::Arc;
use std::collections::VecDeque;
use renderer_shell_vulkan::{VkDeviceContext, VkDescriptorPoolAllocator, VkBuffer, VkResourceDropSink};
use ash::prelude::VkResult;
use fnv::{FnvHashMap, FnvHashSet};
use super::ResourceHash;
use crate::pipeline_description as dsc;
use ash::version::DeviceV1_0;
use crate::resource_managers::ResourceManager;
use crate::pipeline::pipeline::{
    DescriptorSetLayoutWithSlotName, MaterialInstanceSlotAssignment, MaterialInstanceAsset,
};
//use crate::upload::InProgressUploadPollResult::Pending;
use crate::resource_managers::asset_lookup::{
    SlotNameLookup, LoadedAssetLookupSet, LoadedMaterialPass, LoadedMaterialInstance,
    LoadedMaterial,
};
use atelier_assets::loader::handle::AssetHandle;
use crate::resource_managers::resource_lookup::{
    DescriptorSetLayoutResource, ImageViewResource, ResourceLookupSet,
};
use crate::pipeline_description::{DescriptorType, DescriptorSetLayoutBinding};
use std::mem::ManuallyDrop;
use arrayvec::ArrayVec;

mod descriptor_set_arc;
pub use descriptor_set_arc::DescriptorSetArc;

mod dynamic_descriptor_sets;
pub use dynamic_descriptor_sets::DynDescriptorSet;
pub use dynamic_descriptor_sets::DynPassMaterialInstance;
pub use dynamic_descriptor_sets::DynMaterialInstance;

mod descriptor_set_pool;
use descriptor_set_pool::RegisteredDescriptorSetPool;

mod descriptor_set_pool_chunk;
use descriptor_set_pool_chunk::RegisteredDescriptorSetPoolChunk;

mod descriptor_set_buffers;
use descriptor_set_buffers::DescriptorSetPoolRequiredBufferInfo;
use descriptor_set_buffers::DescriptorLayoutBufferSet;

mod descriptor_write_set;
use descriptor_write_set::DescriptorSetWriteElementImage;
use descriptor_write_set::DescriptorSetWriteElementBufferDataBufferRef;
use descriptor_write_set::DescriptorSetWriteElementBufferData;
use descriptor_write_set::DescriptorSetWriteElementBuffer;
use descriptor_write_set::DescriptorSetElementWrite;
use descriptor_write_set::DescriptorSetElementKey;
use descriptor_write_set::DescriptorSetWriteSet;
pub use descriptor_write_set::create_uninitialized_write_set_for_layout;
pub use descriptor_write_set::create_uninitialized_write_sets_for_material_pass;
pub use descriptor_write_set::create_write_sets_for_material_instance_pass;
pub use descriptor_write_set::apply_material_instance_slot_assignment;

mod descriptor_set_manager;
pub use descriptor_set_manager::RegisteredDescriptorSetPoolManager;
pub use descriptor_set_manager::RegisteredDescriptorSetPoolManagerMetrics;
pub use descriptor_set_manager::RegisteredDescriptorSetPoolMetrics;

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
struct RegisteredDescriptorSet {
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

fn subtract_from_frame_in_flight_index(
    index: FrameInFlightIndex,
    value: u32,
) -> FrameInFlightIndex {
    (value + MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32 - index) % MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32
}

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
