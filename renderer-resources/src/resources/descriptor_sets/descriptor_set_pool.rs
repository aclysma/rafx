use super::ManagedDescriptorSet;
use super::ManagedDescriptorSetPoolChunk;
use super::{
    DescriptorSetArc, DescriptorSetElementKey, DescriptorSetPoolRequiredBufferInfo,
    DescriptorSetWriteSet, FrameInFlightIndex, MAX_DESCRIPTORS_PER_POOL, MAX_FRAMES_IN_FLIGHT,
};
use crate::resources::resource_lookup::DescriptorSetLayoutResource;
use crate::resources::ResourceArc;
use crate::vk_description as dsc;
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk;
use crossbeam_channel::{Receiver, Sender};
use renderer_base::slab::{RawSlab, RawSlabKey};
use renderer_shell_vulkan::{
    VkBuffer, VkDescriptorPoolAllocator, VkDeviceContext, VkResourceDropSink,
};
use std::collections::VecDeque;
use std::mem::ManuallyDrop;

struct PendingDescriptorSetDrop {
    slab_key: RawSlabKey<ManagedDescriptorSet>,
    live_until_frame: u32,
}

//TODO: This does not implement any form of defragmentation or trimming - it grows up to the high
// watermark of concurrent number of allocated descriptor sets and remains that size
pub(super) struct ManagedDescriptorSetPool {
    // Keeps track of descriptor sets that are in use
    pub(super) slab: RawSlab<ManagedDescriptorSet>,

    // Used to allow DescriptorSetArc to trigger dropping descriptor sets
    drop_tx: Sender<RawSlabKey<ManagedDescriptorSet>>,
    drop_rx: Receiver<RawSlabKey<ManagedDescriptorSet>>,

    // Used to create new pools
    descriptor_pool_allocator: VkDescriptorPoolAllocator,

    // The layout of descriptor sets that this pool contains
    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,

    // Defers dropping buffers
    //TODO: We defer dropping descriptor sets so we may not need to defer dropping buffers
    buffer_drop_sink: VkResourceDropSink<ManuallyDrop<VkBuffer>>,

    // Metadata about buffers that back data in descriptor sets (this is an opt-in feature per binding)
    buffer_infos: Vec<DescriptorSetPoolRequiredBufferInfo>,

    // The chunks that make up the pool. We allocate in batches as the pool becomes empty
    chunks: Vec<ManagedDescriptorSetPoolChunk>,

    // The drops that we will process later. This allows us to defer dropping bindings until
    // MAX_FRAMES_IN_FLIGHT frames have passed
    pending_drops: VecDeque<PendingDescriptorSetDrop>,
}

impl ManagedDescriptorSetPool {
    pub fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        //
        // This is a little gross but it creates the pool sizes required for the
        // DescriptorPoolCreateInfo passed into create_descriptor_pool. Do it here once instead of
        // in the allocator callback
        //
        let mut descriptor_counts = vec![0; dsc::DescriptorType::count()];
        for desc in &descriptor_set_layout
            .get_raw()
            .descriptor_set_layout_def
            .descriptor_set_layout_bindings
        {
            let ty: vk::DescriptorType = desc.descriptor_type.into();
            assert!(desc.descriptor_count > 0);
            descriptor_counts[ty.as_raw() as usize] +=
                MAX_DESCRIPTORS_PER_POOL * desc.descriptor_count;
        }

        let mut pool_sizes = Vec::with_capacity(dsc::DescriptorType::count());
        for (descriptor_type, count) in descriptor_counts.into_iter().enumerate() {
            if count > 0 {
                let pool_size = vk::DescriptorPoolSize::builder()
                    .descriptor_count(count as u32)
                    .ty(vk::DescriptorType::from_raw(descriptor_type as i32))
                    .build();
                pool_sizes.push(pool_size);
            }
        }

        // The allocator will produce descriptor sets as needed and destroy them after waiting a few
        // frames for them to finish any submits that reference them
        let descriptor_pool_allocator = VkDescriptorPoolAllocator::new(
            MAX_FRAMES_IN_FLIGHT as u32,
            std::u32::MAX, // No upper bound on pool count
            move |device| {
                let pool_builder = vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(MAX_DESCRIPTORS_PER_POOL)
                    .pool_sizes(&pool_sizes);

                unsafe { device.create_descriptor_pool(&*pool_builder, None) }
            },
        );

        let mut buffer_infos = Vec::new();
        for binding in &descriptor_set_layout
            .get_raw()
            .descriptor_set_layout_def
            .descriptor_set_layout_bindings
        {
            if let Some(per_descriptor_size) = binding.internal_buffer_per_descriptor_size {
                // 256 is the max allowed by the vulkan spec but we can improve this by using the
                // actual hardware value given by device limits
                let required_alignment =
                    device_context.limits().min_uniform_buffer_offset_alignment as u32;
                let per_descriptor_stride =
                    renderer_shell_vulkan::util::round_size_up_to_alignment_u32(
                        per_descriptor_size,
                        required_alignment,
                    );

                buffer_infos.push(DescriptorSetPoolRequiredBufferInfo {
                    per_descriptor_size,
                    per_descriptor_stride,
                    descriptor_type: binding.descriptor_type,
                    dst_element: DescriptorSetElementKey {
                        dst_binding: binding.binding,
                    },
                })
            }
        }

        ManagedDescriptorSetPool {
            slab: RawSlab::with_capacity(MAX_DESCRIPTORS_PER_POOL),
            drop_tx,
            drop_rx,
            descriptor_pool_allocator,
            descriptor_set_layout,
            chunks: Default::default(),
            buffer_infos,
            buffer_drop_sink: VkResourceDropSink::new(MAX_FRAMES_IN_FLIGHT as u32),
            pending_drops: Default::default(),
        }
    }

    pub fn insert(
        &mut self,
        device_context: &VkDeviceContext,
        write_set: DescriptorSetWriteSet,
    ) -> VkResult<DescriptorSetArc> {
        let registered_set = ManagedDescriptorSet {
            // Don't have anything to store yet
            //write_set: write_set.clone()
        };

        // Use the slab allocator to find an unused index, determine the chunk index from that
        let slab_key = self.slab.allocate(registered_set);
        let chunk_index = (slab_key.index() / MAX_DESCRIPTORS_PER_POOL) as usize;

        // Add more chunks if necessary
        while chunk_index as usize >= self.chunks.len() {
            self.chunks.push(ManagedDescriptorSetPoolChunk::new(
                device_context,
                &self.buffer_infos,
                self.descriptor_set_layout.get_raw().descriptor_set_layout,
                &mut self.descriptor_pool_allocator,
            )?);
        }

        // Insert the write into the chunk, it will be applied when update() is next called on it
        let descriptor_sets_per_frame =
            self.chunks[chunk_index].schedule_write_set(slab_key, write_set);

        // Return the ref-counted descriptor set
        let descriptor_set =
            DescriptorSetArc::new(slab_key, descriptor_sets_per_frame, self.drop_tx.clone());
        Ok(descriptor_set)
    }

    #[profiling::function]
    pub fn flush_changes(
        &mut self,
        device_context: &VkDeviceContext,
        frame_in_flight_index: FrameInFlightIndex,
    ) -> VkResult<()> {
        // Route messages that indicate a dropped descriptor set to the chunk that owns it
        for dropped in self.drop_rx.try_iter() {
            self.pending_drops.push_back(PendingDescriptorSetDrop {
                slab_key: dropped,
                live_until_frame: super::add_to_frame_in_flight_index(
                    frame_in_flight_index,
                    MAX_FRAMES_IN_FLIGHT as u32,
                ),
            });
        }

        // Determine how many drops we can drain (we keep them around for MAX_FRAMES_IN_FLIGHT frames
        let mut pending_drops_to_drain = 0;
        for pending_drop in &self.pending_drops {
            // If frame_in_flight_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if pending_drop.live_until_frame == frame_in_flight_index {
                self.slab.free(pending_drop.slab_key);
                pending_drops_to_drain += 1;
            } else {
                break;
            }
        }

        if pending_drops_to_drain > 0 {
            log::trace!(
                "Free {} descriptors on frame in flight index {} layout {:?}",
                pending_drops_to_drain,
                frame_in_flight_index,
                self.descriptor_set_layout
            );
        }

        // Drain any drops that have expired
        self.pending_drops.drain(0..pending_drops_to_drain);

        // Commit pending writes/removes, rotate to the descriptor set for the next frame
        for chunk in &mut self.chunks {
            chunk.update(device_context);
        }

        self.descriptor_pool_allocator
            .update(device_context.device())
    }

    pub fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        for chunk in &mut self.chunks {
            chunk.destroy(
                &mut self.descriptor_pool_allocator,
                &mut self.buffer_drop_sink,
            );
        }

        self.descriptor_pool_allocator
            .destroy(device_context.device())?;
        self.buffer_drop_sink.destroy(&device_context)?;
        self.chunks.clear();
        Ok(())
    }
}
