use super::DescriptorSetWriteElementBufferData;
use super::{
    DescriptorLayoutBufferSet, DescriptorSetElementKey, DescriptorSetPoolRequiredBufferInfo,
    DescriptorSetWriteSet, ManagedDescriptorSet, MAX_DESCRIPTOR_SETS_PER_POOL,
};
use crate::descriptor_sets::{
    DescriptorSetElementWrite, DescriptorSetWriteElementBufferDataBufferRef,
    DescriptorSetWriteElementImageValue,
};
use crate::resources::descriptor_sets::DescriptorSetBindingKey;
use crate::{
    BufferResource, DescriptorSetArrayPoolAllocator, DescriptorSetBindings,
    DescriptorSetLayoutResource, ImageViewResource, ResourceArc, ResourceDropSink,
};
use fnv::FnvHashMap;
use rafx_api::{
    RafxBuffer, RafxDescriptorElements, RafxDescriptorKey, RafxDescriptorSetArray,
    RafxDescriptorSetHandle, RafxDescriptorUpdate, RafxDeviceContext, RafxOffsetSize,
    RafxResourceType, RafxResult,
};
use rafx_base::slab::RawSlabKey;
use std::collections::VecDeque;

fn try_queue_descriptor_set_image_update(
    descriptor_set_array: &mut RafxDescriptorSetArray,
    descriptor_set_index: u32,
    element_key: &DescriptorSetElementKey,
    element: &DescriptorSetElementWrite,
) -> RafxResult<()> {
    if element.has_immutable_sampler
        && element
            .descriptor_type
            .intersects(RafxResourceType::SAMPLER | RafxResourceType::COMBINED_IMAGE_SAMPLER)
    {
        // Skip any sampler bindings if the binding is populated with an immutable sampler
        return Ok(());
    }

    let image_info = &element.image_info;
    let image_info_index = element_key.array_index;

    if image_info.sampler.is_none() && image_info.image_view.is_none() {
        // Don't bind anything that has both a null sampler and image_view
        //TODO: Could set back to default state
        return Ok(());
    }

    if let Some(image_view) = &image_info.image_view {
        descriptor_set_array.queue_descriptor_set_update(&RafxDescriptorUpdate {
            array_index: descriptor_set_index,
            descriptor_key: RafxDescriptorKey::Binding(element_key.dst_binding),
            elements: RafxDescriptorElements {
                textures: Some(&[&image_view.get_image()]),
                ..Default::default()
            },
            dst_element_offset: image_info_index as u32,
            texture_bind_type: Default::default(),
        })?;
    }

    // Skip adding samplers if the binding is populated with an immutable sampler
    // (this case is hit when using CombinedImageSampler)
    if !element.has_immutable_sampler {
        if let Some(sampler) = &image_info.sampler {
            descriptor_set_array.queue_descriptor_set_update(&RafxDescriptorUpdate {
                array_index: descriptor_set_index,
                descriptor_key: RafxDescriptorKey::Binding(element_key.dst_binding),
                elements: RafxDescriptorElements {
                    samplers: Some(&[&sampler.get_raw().sampler]),
                    ..Default::default()
                },
                dst_element_offset: image_info_index as u32,
                texture_bind_type: Default::default(),
            })?;
        }
    }

    Ok(())
}

fn try_queue_descriptor_set_buffer_update(
    buffers: &mut DescriptorLayoutBufferSet,
    descriptor_set_array: &mut RafxDescriptorSetArray,
    descriptor_set_index: u32,
    element_key: &DescriptorSetElementKey,
    element: &DescriptorSetElementWrite,
) -> RafxResult<()> {
    let buffer_info = &element.buffer_info;
    let buffer_info_index = element_key.array_index;

    if let Some(buffer_info) = &buffer_info.buffer {
        match buffer_info {
            DescriptorSetWriteElementBufferData::BufferRef(buffer) => {
                let mut offset_sizes = None;
                if buffer.byte_offset.is_some() || buffer.size.is_some() {
                    offset_sizes = Some([RafxOffsetSize {
                        byte_offset: buffer.byte_offset.unwrap_or(0),
                        size: buffer.size.unwrap_or(0),
                    }])
                }

                descriptor_set_array.queue_descriptor_set_update(&RafxDescriptorUpdate {
                    array_index: descriptor_set_index,
                    descriptor_key: RafxDescriptorKey::Binding(element_key.dst_binding),
                    elements: RafxDescriptorElements {
                        buffers: Some(&[&*buffer.buffer.get_raw().buffer]),
                        buffer_offset_sizes: offset_sizes.as_ref().map(|x| &x[..]),
                        ..Default::default()
                    },
                    dst_element_offset: buffer_info_index as u32,
                    texture_bind_type: Default::default(),
                })?;
            }
            DescriptorSetWriteElementBufferData::Data(data) => {
                copy_data_to_buffer(buffers, descriptor_set_index, element_key, data)?;
            }
        }
    }

    Ok(())
}

fn copy_data_to_buffer<T: Copy>(
    buffers: &mut DescriptorLayoutBufferSet,
    descriptor_set_index: u32,
    element_key: &DescriptorSetElementKey,
    data: &[T],
) -> RafxResult<()> {
    //TODO: Rebind the buffer if we are no longer bound to the internal buffer, or at
    // least fail
    // Failing here means that we're trying to write to a descriptor's internal buffer
    // but the binding was not configured to enabled internal buffering

    let buffer = buffers
        .buffer_sets
        .get_mut(&DescriptorSetBindingKey {
            dst_binding: element_key.dst_binding,
        })
        .expect("Tried to copy data into descriptor set internal buffer but could not find buffer for this binding. Is @[internal_buffer] missing in the shader?");

    if data.len() as u32 > buffer.buffer_info.per_descriptor_size {
        panic!(
            "Wrote {} bytes to a descriptor set buffer that holds {} bytes",
            data.len(),
            buffer.buffer_info.per_descriptor_size
        );
    }

    if data.len() as u32 != buffer.buffer_info.per_descriptor_size {
        log::warn!(
            "Wrote {} bytes to a descriptor set buffer that holds {} bytes",
            data.len(),
            buffer.buffer_info.per_descriptor_size
        );
    }

    let offset = buffer.buffer_info.per_descriptor_stride * descriptor_set_index;

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

    Ok(())
}

pub trait DescriptorSetWriter<'a> {
    fn write_to(
        descriptor_set: &mut DescriptorSetWriterContext,
        args: Self,
    );
}

pub struct DescriptorSetWriterContext<'a> {
    descriptor_set_handle: RafxDescriptorSetHandle,
    descriptor_set_index: usize,
    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    descriptor_set_array: &'a mut RafxDescriptorSetArray,
    buffers: &'a mut DescriptorLayoutBufferSet,
}

impl DescriptorSetWriterContext<'_> {
    pub(crate) fn handle(&self) -> RafxDescriptorSetHandle {
        self.descriptor_set_handle.clone()
    }

    fn write(
        &mut self,
        element_key: &DescriptorSetElementKey,
        element: &DescriptorSetElementWrite,
    ) -> RafxResult<()> {
        let descriptor_set_index = self.descriptor_set_index;

        log::trace!(
            "Process descriptor set write for {:?} {:?}. layout {:?}",
            descriptor_set_index,
            element_key,
            self.descriptor_set_layout,
        );

        try_queue_descriptor_set_image_update(
            self.descriptor_set_array,
            descriptor_set_index as u32,
            element_key,
            element,
        )?;

        try_queue_descriptor_set_buffer_update(
            self.buffers,
            self.descriptor_set_array,
            descriptor_set_index as u32,
            element_key,
            element,
        )?;

        Ok(())
    }

    fn write_buffer_data<T: Copy>(
        &mut self,
        element_key: &DescriptorSetElementKey,
        data: &[T],
    ) -> RafxResult<()> {
        copy_data_to_buffer(
            self.buffers,
            self.descriptor_set_index as u32,
            element_key,
            data,
        )
    }

    fn set_image_array_element(
        &mut self,
        binding_index: u32,
        array_index: usize,
        image_view: DescriptorSetWriteElementImageValue,
    ) -> RafxResult<()> {
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            array_index,
        };

        if let Some(mut element) =
            super::get_descriptor_set_element_write(&self.descriptor_set_layout, &key)
        {
            let what_to_bind = super::what_to_bind(&element);
            if what_to_bind.bind_images {
                element.image_info.image_view = Some(image_view);
                self.write(&key, &element)?;
            } else {
                // This is not necessarily an error if the user is binding with a slot name (although not sure
                // if that's the right approach long term)
                //log::warn!("Tried to bind an image to a descriptor set where the type does not accept an image", array_index)
            }
        } else {
            log::warn!("Tried to set image on a binding index that does not exist");
        }

        Ok(())
    }

    fn set_buffer_array_element(
        &mut self,
        binding_index: u32,
        array_index: usize,
        buffer: &ResourceArc<BufferResource>,
    ) -> RafxResult<()> {
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            array_index,
        };

        if let Some(mut element) =
            super::get_descriptor_set_element_write(&self.descriptor_set_layout, &key)
        {
            let what_to_bind = super::what_to_bind(&element);
            if what_to_bind.bind_buffers {
                element.buffer_info.buffer = Some(DescriptorSetWriteElementBufferData::BufferRef(
                    DescriptorSetWriteElementBufferDataBufferRef {
                        buffer: buffer.clone(),
                        byte_offset: None,
                        size: None,
                    },
                ));
                self.write(&key, &element)?;
            } else {
                // This is not necessarily an error if the user is binding with a slot name (although not sure
                // if that's the right approach long term)
                //log::warn!("Tried to bind an image to a descriptor set where the type does not accept an image", array_index)
            }
        } else {
            log::warn!("Tried to set image on a binding index that does not exist");
        }

        Ok(())
    }

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    fn set_buffer_data_array_element<T: Copy + 'static>(
        &mut self,
        binding_index: u32,
        array_index: usize,
        data: &T,
    ) -> RafxResult<()> {
        //TODO: Verify that T's size matches the buffer
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            array_index,
        };

        if let Some(element) =
            super::get_descriptor_set_element_write(&self.descriptor_set_layout, &key)
        {
            let what_to_bind = super::what_to_bind(&element);
            if what_to_bind.bind_buffers {
                let data = rafx_base::memory::any_as_bytes(data);
                self.write_buffer_data(&key, data)?;
            } else {
                // This is not necessarily an error if the user is binding with a slot name (although not sure
                // if that's the right approach long term)
                //log::warn!("Tried to bind an image to a descriptor set where the type does not accept an image", array_index)
            }
        } else {
            log::warn!("Tried to set buffer data on a binding index that does not exist");
        }

        Ok(())
    }
}

impl DescriptorSetBindings for DescriptorSetWriterContext<'_> {
    fn set_image(
        &mut self,
        binding_index: u32,
        image_view: &ResourceArc<ImageViewResource>,
    ) {
        self.set_image_array_element(
            binding_index,
            0,
            DescriptorSetWriteElementImageValue::Resource(image_view.clone()),
        )
        .unwrap();
    }

    fn set_images(
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
                .unwrap();
            }
        }
    }

    fn set_image_at_index(
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
        .unwrap();
    }

    fn set_buffer(
        &mut self,
        binding_index: u32,
        data: &ResourceArc<BufferResource>,
    ) {
        self.set_buffer_array_element(binding_index, 0, data)
            .unwrap();
    }

    fn set_buffer_at_index(
        &mut self,
        binding_index: u32,
        array_index: usize,
        data: &ResourceArc<BufferResource>,
    ) {
        self.set_buffer_array_element(binding_index, array_index, data)
            .unwrap();
    }

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    fn set_buffer_data<T: Copy + 'static>(
        &mut self,
        binding_index: u32,
        data: &T,
    ) {
        self.set_buffer_data_array_element(binding_index, 0, data)
            .unwrap();
    }

    // Requiring 'static helps us catch accidentally trying to store a reference in the buffer
    fn set_buffer_data_at_index<T: Copy + 'static>(
        &mut self,
        binding_index: u32,
        array_index: usize,
        data: &T,
    ) {
        self.set_buffer_data_array_element(binding_index, array_index, data)
            .unwrap();
    }
}

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
        allocator: &mut DescriptorSetArrayPoolAllocator,
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
                            byte_offset: offset,
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
        pool_allocator: &mut DescriptorSetArrayPoolAllocator,
        buffer_drop_sink: &mut ResourceDropSink<RafxBuffer>,
    ) {
        pool_allocator.retire_pool(self.descriptor_set_array.take().unwrap());
        for (_, buffer_set) in self.buffers.buffer_sets.drain() {
            buffer_drop_sink.retire(buffer_set.buffer);
        }
    }

    pub(super) fn get_writer(
        &mut self,
        slab_key: RawSlabKey<ManagedDescriptorSet>,
    ) -> RafxResult<DescriptorSetWriterContext> {
        let descriptor_set_array = { self.descriptor_set_array.as_mut().unwrap() };
        let buffers = &mut self.buffers;

        log::trace!(
            "Write directly to descriptor set {:?} on layout {:?}",
            slab_key,
            self.descriptor_set_layout
        );

        let descriptor_index = slab_key.index() % MAX_DESCRIPTOR_SETS_PER_POOL;

        let descriptor_set_handle = descriptor_set_array.handle(descriptor_index).unwrap();

        Ok(DescriptorSetWriterContext {
            descriptor_set_handle,
            descriptor_set_index: descriptor_index as usize,
            descriptor_set_layout: self.descriptor_set_layout.clone(),
            descriptor_set_array,
            buffers,
        })
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

        let descriptor_set_array = { self.descriptor_set_array.as_mut().unwrap() };

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

            try_queue_descriptor_set_image_update(
                descriptor_set_array,
                descriptor_set_index,
                &element_key,
                element,
            )?;

            try_queue_descriptor_set_buffer_update(
                &mut self.buffers,
                descriptor_set_array,
                descriptor_set_index,
                &element_key,
                element,
            )?;
        }

        descriptor_set_array.flush_descriptor_set_updates()?;

        self.pending_set_writes.clear();
        Ok(())
    }
}
