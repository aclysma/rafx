use super::DescriptorSetWriteElementBufferData;
use super::{
    DescriptorLayoutBufferSet, DescriptorSetElementKey, DescriptorSetPoolRequiredBufferInfo,
    DescriptorSetWriteSet, ManagedDescriptorSet, MAX_DESCRIPTOR_SETS_PER_POOL,
};
use crate::{
    DescriptorSetLayoutResource, ResourceArc, ResourceDropSink, VkDescriptorPoolAllocator,
};
use fnv::FnvHashMap;
use rafx_api::{
    RafxBuffer, RafxDescriptorElements, RafxDescriptorKey, RafxDescriptorSetArray,
    RafxDescriptorSetHandle, RafxDescriptorUpdate, RafxDeviceContext, RafxOffsetSize,
    RafxResourceType, RafxResult,
};
use rafx_base::slab::RawSlabKey;
use std::collections::VecDeque;

// A write to the descriptors within a single descriptor set that has been scheduled (i.e. will occur
// over the next MAX_FRAMES_IN_FLIGHT_PLUS_1 frames
#[derive(Debug)]
struct PendingDescriptorSetWriteSet {
    slab_key: RawSlabKey<ManagedDescriptorSet>,
    write_set: DescriptorSetWriteSet,
}

//
// A single chunk within a pool. This allows us to create MAX_DESCRIPTOR_SETS_PER_POOL * MAX_FRAMES_IN_FLIGHT_PLUS_1
// descriptors for a single descriptor set layout
//
pub(super) struct ManagedDescriptorSetPoolChunk {
    // for logging
    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,

    // The pool holding all descriptors in this chunk
    // This is Some until destroy() is called, at which point the descriptor set array is returned
    // to a pool for future reuse
    descriptor_set_array: Option<RafxDescriptorSetArray>,

    // The buffers that back the descriptor sets
    buffers: DescriptorLayoutBufferSet,

    // The writes that have been scheduled to occur over the next MAX_FRAMES_IN_FLIGHT_PLUS_1 frames. This
    // ensures that each frame's descriptor sets/buffers are appropriately updated
    pending_set_writes: VecDeque<PendingDescriptorSetWriteSet>,
}

impl ManagedDescriptorSetPoolChunk {
    #[profiling::function]
    pub(super) fn new(
        device_context: &RafxDeviceContext,
        buffer_info: &[DescriptorSetPoolRequiredBufferInfo],
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        allocator: &mut VkDescriptorPoolAllocator,
    ) -> RafxResult<Self> {
        let mut descriptor_set_array = allocator.allocate_pool()?;

        // Now allocate all the buffers that act as backing-stores for descriptor sets
        let buffers = DescriptorLayoutBufferSet::new(device_context, buffer_info)?;

        // For every binding/buffer set
        for (binding_key, binding_buffers) in &buffers.buffer_sets {
            // For every descriptor
            let mut offset = 0;
            for i in 0..MAX_DESCRIPTOR_SETS_PER_POOL {
                descriptor_set_array.queue_descriptor_set_update(&RafxDescriptorUpdate {
                    descriptor_key: RafxDescriptorKey::Binding(binding_key.dst_binding),
                    array_index: i,
                    elements: RafxDescriptorElements {
                        buffers: Some(&[&binding_buffers.buffer]),
                        buffer_offset_sizes: Some(&[RafxOffsetSize {
                            offset: offset,
                            size: binding_buffers.buffer_info.per_descriptor_size as u64,
                        }]),
                        ..Default::default()
                    },
                    dst_element_offset: 0,
                    texture_bind_type: None,
                })?;

                offset += binding_buffers.buffer_info.per_descriptor_stride as u64;
            }
        }

        descriptor_set_array.flush_descriptor_set_updates()?;

        Ok(ManagedDescriptorSetPoolChunk {
            descriptor_set_layout: descriptor_set_layout.clone(),
            descriptor_set_array: Some(descriptor_set_array),
            //descriptor_sets,
            pending_set_writes: Default::default(),
            buffers,
        })
    }

    pub(super) fn destroy(
        &mut self,
        pool_allocator: &mut VkDescriptorPoolAllocator,
        buffer_drop_sink: &mut ResourceDropSink<RafxBuffer>,
    ) {
        pool_allocator.retire_pool(self.descriptor_set_array.take().unwrap());
        for (_, buffer_set) in self.buffers.buffer_sets.drain() {
            buffer_drop_sink.retire(buffer_set.buffer);
        }
    }

    pub(super) fn schedule_write_set(
        &mut self,
        slab_key: RawSlabKey<ManagedDescriptorSet>,
        write_set: DescriptorSetWriteSet,
    ) -> RafxDescriptorSetHandle {
        log::trace!(
            "Schedule a write for descriptor set {:?} on layout {:?}",
            slab_key,
            self.descriptor_set_layout
        );
        //log::trace!("{:#?}", write_set);

        // Use frame_in_flight_index for the live_until_frame because every update, we immediately
        // increment the frame and *then* do updates. So by setting it to the pre-next-update
        // frame_in_flight_index, this will make the write stick around for this and the next
        // MAX_FRAMES_IN_FLIGHT frames
        let pending_write = PendingDescriptorSetWriteSet {
            slab_key,
            write_set,
        };

        //TODO: Consider pushing these into a hashmap for the frame and let the pending write array
        // be a list of hashmaps
        self.pending_set_writes.push_back(pending_write);

        let descriptor_index = slab_key.index() % MAX_DESCRIPTOR_SETS_PER_POOL;
        self.descriptor_set_array
            .as_ref()
            .unwrap()
            .handle(descriptor_index)
            .unwrap()
    }

    #[profiling::function]
    pub(super) fn update(&mut self) -> RafxResult<()> {
        #[derive(PartialEq, Eq, Hash, Debug)]
        struct SlabElementKey(RawSlabKey<ManagedDescriptorSet>, DescriptorSetElementKey);

        // Flatten the vec of hash maps into a single hashmap. This eliminates any duplicate
        // sets with the most recent set taking precedence
        let mut all_set_writes = FnvHashMap::default();
        for pending_write in &self.pending_set_writes {
            for (key, value) in &pending_write.write_set.elements {
                all_set_writes.insert(SlabElementKey(pending_write.slab_key, *key), value);
            }
        }

        let descriptor_set_array = self.descriptor_set_array.as_mut().unwrap();

        for (key, element) in all_set_writes {
            let slab_key = key.0;
            let element_key = key.1;

            //log::trace!("{:#?}", element);

            let descriptor_set_index = slab_key.index() % MAX_DESCRIPTOR_SETS_PER_POOL;

            log::trace!(
                "Process descriptor set pending_write for {:?} {:?}. layout {:?}",
                slab_key,
                element_key,
                self.descriptor_set_layout,
            );

            if !element.image_info.is_empty() {
                for (image_info_index, image_info) in element.image_info.iter().enumerate() {
                    if element.has_immutable_sampler
                        && element.descriptor_type.intersects(
                            RafxResourceType::SAMPLER | RafxResourceType::COMBINED_IMAGE_SAMPLER,
                        )
                    {
                        // Skip any sampler bindings if the binding is populated with an immutable sampler
                        continue;
                    }

                    if image_info.sampler.is_none() && image_info.image_view.is_none() {
                        // Don't bind anything that has both a null sampler and image_view
                        //TODO: Could set back to default state
                        continue;
                    }

                    if let Some(image_view) = &image_info.image_view {
                        descriptor_set_array.queue_descriptor_set_update(
                            &RafxDescriptorUpdate {
                                array_index: descriptor_set_index,
                                descriptor_key: RafxDescriptorKey::Binding(element_key.dst_binding),
                                elements: RafxDescriptorElements {
                                    textures: Some(&[image_view.get_image().texture()]),
                                    ..Default::default()
                                },
                                dst_element_offset: image_info_index as u32,
                                texture_bind_type: Default::default(),
                            },
                        )?;
                    }

                    // Skip adding samplers if the binding is populated with an immutable sampler
                    // (this case is hit when using CombinedImageSampler)
                    if !element.has_immutable_sampler {
                        if let Some(sampler) = &image_info.sampler {
                            descriptor_set_array.queue_descriptor_set_update(
                                &RafxDescriptorUpdate {
                                    array_index: descriptor_set_index,
                                    descriptor_key: RafxDescriptorKey::Binding(
                                        element_key.dst_binding,
                                    ),
                                    elements: RafxDescriptorElements {
                                        samplers: Some(&[&sampler.get_raw().sampler]),
                                        ..Default::default()
                                    },
                                    dst_element_offset: image_info_index as u32,
                                    texture_bind_type: Default::default(),
                                },
                            )?;
                        }
                    }
                }
            }

            if !element.buffer_info.is_empty() {
                for (buffer_info_index, buffer_info) in element.buffer_info.iter().enumerate() {
                    if let Some(buffer_info) = &buffer_info.buffer {
                        match buffer_info {
                            DescriptorSetWriteElementBufferData::BufferRef(buffer) => {
                                let mut offset_sizes = None;
                                if buffer.offset.is_some() || buffer.size.is_some() {
                                    offset_sizes = Some([RafxOffsetSize {
                                        offset: buffer.offset.unwrap_or(0),
                                        size: buffer.size.unwrap_or(0),
                                    }])
                                }

                                descriptor_set_array.queue_descriptor_set_update(
                                    &RafxDescriptorUpdate {
                                        array_index: descriptor_set_index,
                                        descriptor_key: RafxDescriptorKey::Binding(
                                            element_key.dst_binding,
                                        ),
                                        elements: RafxDescriptorElements {
                                            buffers: Some(&[&*buffer.buffer.get_raw().buffer]),
                                            buffer_offset_sizes: offset_sizes
                                                .as_ref()
                                                .map(|x| &x[..]),
                                            ..Default::default()
                                        },
                                        dst_element_offset: buffer_info_index as u32,
                                        texture_bind_type: Default::default(),
                                    },
                                )?;
                            }
                            DescriptorSetWriteElementBufferData::Data(data) => {
                                //TODO: Rebind the buffer if we are no longer bound to the internal buffer, or at
                                // least fail
                                // Failing here means that we're trying to write to a descriptor's internal buffer
                                // but the binding was not configured to enabled internal buffering
                                let buffer =
                                    self.buffers.buffer_sets.get_mut(&element_key).unwrap();
                                //assert!(data.len() as u32 <= buffer.buffer_info.per_descriptor_size);
                                if data.len() as u32 > buffer.buffer_info.per_descriptor_size {
                                    panic!(
                                        "Wrote {} bytes to a descriptor set buffer that holds {} bytes layout: {:?}",
                                        data.len(),
                                        buffer.buffer_info.per_descriptor_size,
                                        self.descriptor_set_layout
                                    );
                                }

                                if data.len() as u32 != buffer.buffer_info.per_descriptor_size {
                                    log::warn!(
                                        "Wrote {} bytes to a descriptor set buffer that holds {} bytes layout: {:?}",
                                        data.len(),
                                        buffer.buffer_info.per_descriptor_size,
                                        self.descriptor_set_layout
                                    );
                                }

                                let descriptor_set_index =
                                    slab_key.index() % MAX_DESCRIPTOR_SETS_PER_POOL;
                                let offset =
                                    buffer.buffer_info.per_descriptor_stride * descriptor_set_index;

                                log::trace!(
                                    "Writing {} bytes to internal buffer to set {} at offset {}",
                                    data.len(),
                                    descriptor_set_index,
                                    offset
                                );
                                buffer
                                    .buffer
                                    .copy_to_host_visible_buffer_with_offset(&data, offset as u64)
                                    .unwrap();

                                //TODO: If we bound this as BufferRef, we would need to reset it back to Data

                                // descriptor_set_array.queue_descriptor_set_update(&RafxDescriptorUpdate {
                                //     array_index: descriptor_set_index,
                                //     descriptor_key: RafxDescriptorKey::Binding(element_key.dst_binding),
                                //     elements: RafxDescriptorElements {
                                //         buffers: Some(&[&buffer.buffer]),
                                //         buffer_offset_sizes: Some(&[
                                //             RafxOffsetSize {
                                //                 offset: offset as u64,
                                //                 size: buffer.buffer_info.per_descriptor_size as u64
                                //             }
                                //         ]),
                                //         ..Default::default()
                                //     },
                                //     dst_element_offset: buffer_info_index as u32,
                                //     texture_bind_type: Default::default(),
                                // });
                            }
                        }
                    }
                }
            }
        }

        descriptor_set_array.flush_descriptor_set_updates()?;

        self.pending_set_writes.clear();
        Ok(())
    }
}
