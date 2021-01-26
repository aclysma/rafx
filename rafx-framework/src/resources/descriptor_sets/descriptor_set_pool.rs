use super::ManagedDescriptorSet;
use super::ManagedDescriptorSetPoolChunk;
use super::{
    DescriptorSetArc, DescriptorSetElementKey, DescriptorSetPoolRequiredBufferInfo,
    DescriptorSetWriteSet, FrameInFlightIndex, MAX_DESCRIPTOR_SETS_PER_POOL, MAX_FRAMES_IN_FLIGHT,
};
use crate::resources::resource_lookup::DescriptorSetLayoutResource;
use crate::resources::ResourceArc;
use crate::{DescriptorSetArrayPoolAllocator, ResourceDropSink};
use crossbeam_channel::{Receiver, Sender};
use rafx_api::{RafxBuffer, RafxDescriptorSetArrayDef, RafxDeviceContext, RafxResult};
use rafx_base::slab::{RawSlab, RawSlabKey};
use std::collections::VecDeque;

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
    descriptor_pool_allocator: DescriptorSetArrayPoolAllocator,

    // The layout of descriptor sets that this pool contains
    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,

    // Defers dropping buffers
    //TODO: We defer dropping descriptor sets so we may not need to defer dropping buffers
    buffer_drop_sink: ResourceDropSink<RafxBuffer>,

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
        device_context: &RafxDeviceContext,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        // The allocator will produce descriptor sets as needed and destroy them after waiting a few
        // frames for them to finish any submits that reference them
        let descriptor_set_layout_clone = descriptor_set_layout.clone();
        let descriptor_pool_allocator = DescriptorSetArrayPoolAllocator::new(
            device_context,
            MAX_FRAMES_IN_FLIGHT as u32,
            std::u32::MAX, // No upper bound on pool count
            move |device_context| {
                device_context.create_descriptor_set_array(&RafxDescriptorSetArrayDef {
                    root_signature: &descriptor_set_layout_clone.get_raw().root_signature,
                    set_index: descriptor_set_layout_clone.get_raw().set_index,
                    array_length: MAX_DESCRIPTOR_SETS_PER_POOL as usize,
                })
            },
        );

        let mut buffer_infos = Vec::new();
        for binding in &descriptor_set_layout
            .get_raw()
            .descriptor_set_layout_def
            .bindings
        {
            if let Some(per_descriptor_size) = binding.internal_buffer_per_descriptor_size {
                // Our stride is at least as large as the descriptor's buffer
                let mut per_descriptor_stride = per_descriptor_size;

                let device_info = device_context.device_info();

                // Round up uniform buffer stride to space them out as needed by the GPU
                if binding.resource.resource_type.is_uniform_buffer() {
                    per_descriptor_stride = rafx_base::memory::round_size_up_to_alignment_u32(
                        per_descriptor_stride,
                        device_info.min_uniform_buffer_offset_alignment,
                    );
                }

                // Round up storage buffer stride to space them out as needed by the GPU
                if binding.resource.resource_type.is_storage_buffer() {
                    per_descriptor_stride = rafx_base::memory::round_size_up_to_alignment_u32(
                        per_descriptor_stride,
                        device_info.min_storage_buffer_offset_alignment,
                    );
                }

                buffer_infos.push(DescriptorSetPoolRequiredBufferInfo {
                    per_descriptor_size,
                    per_descriptor_stride,
                    descriptor_type: binding.resource.resource_type,
                    dst_element: DescriptorSetElementKey {
                        dst_binding: binding.resource.binding,
                    },
                })
            }
        }

        ManagedDescriptorSetPool {
            slab: RawSlab::with_capacity(MAX_DESCRIPTOR_SETS_PER_POOL),
            drop_tx,
            drop_rx,
            descriptor_pool_allocator,
            descriptor_set_layout,
            chunks: Default::default(),
            buffer_infos,
            buffer_drop_sink: ResourceDropSink::new(MAX_FRAMES_IN_FLIGHT as u32),
            pending_drops: Default::default(),
        }
    }

    pub fn insert(
        &mut self,
        device_context: &RafxDeviceContext,
        write_set: DescriptorSetWriteSet,
    ) -> RafxResult<DescriptorSetArc> {
        let registered_set = ManagedDescriptorSet {
            // Don't have anything to store yet
            //write_set: write_set.clone()
        };

        // Use the slab allocator to find an unused index, determine the chunk index from that
        let slab_key = self.slab.allocate(registered_set);
        let chunk_index = (slab_key.index() / MAX_DESCRIPTOR_SETS_PER_POOL) as usize;

        // Add more chunks if necessary
        while chunk_index as usize >= self.chunks.len() {
            self.chunks.push(ManagedDescriptorSetPoolChunk::new(
                device_context,
                &self.buffer_infos,
                &self.descriptor_set_layout,
                &mut self.descriptor_pool_allocator,
            )?);
        }

        // Insert the write into the chunk, it will be applied when update() is next called on it
        let descriptor_set_handle =
            self.chunks[chunk_index].schedule_write_set(slab_key, write_set);

        // Return the ref-counted descriptor set
        let descriptor_set_arc = DescriptorSetArc::new(
            slab_key,
            self.drop_tx.clone(),
            &self.descriptor_set_layout,
            descriptor_set_handle,
        );

        Ok(descriptor_set_arc)
    }

    #[profiling::function]
    pub fn flush_changes(
        &mut self,
        frame_in_flight_index: FrameInFlightIndex,
    ) -> RafxResult<()> {
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
            chunk.update()?;
        }

        self.descriptor_pool_allocator.update()
    }

    pub fn destroy(&mut self) -> RafxResult<()> {
        for chunk in &mut self.chunks {
            chunk.destroy(
                &mut self.descriptor_pool_allocator,
                &mut self.buffer_drop_sink,
            );
        }

        self.descriptor_pool_allocator.destroy()?;
        self.buffer_drop_sink.destroy()?;
        self.chunks.clear();
        Ok(())
    }
}
