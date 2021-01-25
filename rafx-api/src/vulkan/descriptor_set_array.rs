use crate::vulkan::{DescriptorSetLayoutInfo, RafxDescriptorHeapVulkan, RafxDeviceContextVulkan};
use crate::{
    RafxDescriptorKey, RafxDescriptorSetArrayDef, RafxDescriptorUpdate, RafxResourceType,
    RafxResult, RafxRootSignature, RafxTextureBindType,
};
use ash::version::DeviceV1_0;
use ash::vk;

struct DescriptorUpdateData {
    // one per set * elements in each descriptor
    image_infos: Vec<vk::DescriptorImageInfo>,
    buffer_infos: Vec<vk::DescriptorBufferInfo>,
    buffer_views: Vec<vk::BufferView>,
    update_data_count: usize,
}

impl DescriptorUpdateData {
    fn new(update_data_count: usize) -> Self {
        DescriptorUpdateData {
            image_infos: vec![vk::DescriptorImageInfo::default(); update_data_count],
            buffer_infos: vec![vk::DescriptorBufferInfo::default(); update_data_count],
            buffer_views: vec![vk::BufferView::default(); update_data_count],
            update_data_count,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RafxDescriptorSetHandleVulkan(pub vk::DescriptorSet);

pub struct RafxDescriptorSetArrayVulkan {
    root_signature: RafxRootSignature,
    set_index: u32,
    // one per set
    descriptor_sets: Vec<vk::DescriptorSet>,
    //update_data: Vec<UpdateData>,
    //dynamic_size_offset: Option<SizeOffset>,
    update_data: DescriptorUpdateData,
    //WARNING: This contains pointers into data stored in DescriptorUpdateData, however those
    // vectors are not added/removed from so their addresses will remain stable, even if this
    // struct is moved
    pending_writes: Vec<vk::WriteDescriptorSet>,
}

impl std::fmt::Debug for RafxDescriptorSetArrayVulkan {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDescriptorSetArrayVulkan")
            .field("first_descriptor_set", &self.descriptor_sets[0])
            .field("root_signature", &self.root_signature)
            .field("set_index", &self.set_index)
            .field("pending_write_count", &self.pending_writes.len())
            .finish()
    }
}

// For *const c_void in vk::WriteDescriptorSet, which always point at contents of vectors in
// update_data that never get resized
unsafe impl Send for RafxDescriptorSetArrayVulkan {}

impl RafxDescriptorSetArrayVulkan {
    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn set_index(&self) -> u32 {
        self.set_index
    }

    pub fn vk_descriptor_set(
        &self,
        index: u32,
    ) -> Option<vk::DescriptorSet> {
        self.descriptor_sets.get(index as usize).copied()
    }

    pub fn handle(
        &self,
        index: u32,
    ) -> Option<RafxDescriptorSetHandleVulkan> {
        self.descriptor_sets
            .get(index as usize)
            .map(|x| RafxDescriptorSetHandleVulkan(*x))
    }

    pub(crate) fn new(
        device_context: &RafxDeviceContextVulkan,
        heap: &RafxDescriptorHeapVulkan,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<Self> {
        let root_signature = descriptor_set_array_def
            .root_signature
            .vk_root_signature()
            .unwrap()
            .clone();
        let layout_index = descriptor_set_array_def.set_index as usize;
        let update_data_count = descriptor_set_array_def.array_length
            * root_signature.inner.layouts[layout_index].update_data_count_per_set as usize;
        // let dynamic_offset_count = root_signature.layouts[layout_index]
        //     .dynamic_descriptor_indexes
        //     .len();

        let descriptor_set_layout = root_signature.inner.descriptor_set_layouts[layout_index];

        // these persist
        let mut descriptors_set_layouts = Vec::with_capacity(descriptor_set_array_def.array_length);
        //let mut update_data = Vec::with_capacity(descriptor_set_array_def.array_length * update_data_count);
        let update_data = DescriptorUpdateData::new(update_data_count);

        if root_signature.inner.descriptor_set_layouts[layout_index]
            == vk::DescriptorSetLayout::null()
        {
            Err("Descriptor set layout does not exist in this root signature")?;
        }

        for _ in 0..descriptor_set_array_def.array_length {
            descriptors_set_layouts.push(descriptor_set_layout);

            // for _ in 0..update_data_count {
            //     //TODO: copy it from root signature update template
            //     update_data.push(UpdateData::default());
            // }
        }

        let descriptor_sets =
            heap.allocate_descriptor_sets(device_context.device(), &descriptors_set_layouts)?;

        // let dynamic_size_offset = if dynamic_offset_count > 0 {
        //     assert_eq!(1, dynamic_offset_count);
        //     Some(SizeOffset)
        // } else {
        //     None
        // };

        Ok(RafxDescriptorSetArrayVulkan {
            root_signature: RafxRootSignature::Vk(root_signature),
            set_index: descriptor_set_array_def.set_index,
            descriptor_sets,
            update_data,
            pending_writes: Vec::default(),
        })
    }

    pub fn update_descriptor_set(
        &mut self,
        descriptor_updates: &[RafxDescriptorUpdate],
    ) -> RafxResult<()> {
        for update in descriptor_updates {
            self.queue_descriptor_set_update(update)?;
        }
        self.flush_descriptor_set_updates()
    }

    pub fn flush_descriptor_set_updates(&mut self) -> RafxResult<()> {
        if !self.pending_writes.is_empty() {
            let device = self
                .root_signature
                .vk_root_signature()
                .unwrap()
                .device_context()
                .device();
            unsafe {
                device.update_descriptor_sets(&self.pending_writes, &[]);
            }

            self.pending_writes.clear();
        }

        Ok(())
    }

    pub fn queue_descriptor_set_update(
        &mut self,
        update: &RafxDescriptorUpdate,
    ) -> RafxResult<()> {
        let root_signature = self.root_signature.vk_root_signature().unwrap();
        let layout: &DescriptorSetLayoutInfo =
            &root_signature.inner.layouts[self.set_index as usize];
        let descriptor_index = match &update.descriptor_key {
            RafxDescriptorKey::Name(name) => {
                let descriptor_index = root_signature.find_descriptor_by_name(name);
                if let Some(descriptor_index) = descriptor_index {
                    let set_index = root_signature
                        .descriptor(descriptor_index)
                        .unwrap()
                        .set_index;
                    if set_index == self.set_index {
                        descriptor_index
                    } else {
                        return Err(format!(
                            "Found descriptor {:?} but it's set_index ({:?}) does not match the set ({:?})",
                            &update.descriptor_key,
                            set_index,
                            self.set_index
                        ))?;
                    }
                } else {
                    return Err(format!(
                        "Could not find descriptor {:?}",
                        &update.descriptor_key
                    ))?;
                }
            }
            RafxDescriptorKey::Binding(binding) => layout
                .binding_to_descriptor_index
                .get(binding)
                .copied()
                .ok_or_else(|| format!("Could not find descriptor {:?}", update.descriptor_key,))?,
            RafxDescriptorKey::DescriptorIndex(descriptor_index) => *descriptor_index,
            RafxDescriptorKey::Undefined => {
                return Err("Passed RafxDescriptorKey::Undefined to update_descriptor_set()")?
            }
        };

        //let descriptor_index = descriptor_index.ok_or_else(|| format!("Could not find descriptor {:?}", &update.descriptor_key))?;
        let descriptor = root_signature.descriptor(descriptor_index).unwrap();

        let descriptor_first_update_data = descriptor.update_data_offset_in_set.unwrap()
            + (layout.update_data_count_per_set * update.array_index);

        //let mut descriptor_set_writes = Vec::default();

        let vk_set = self.descriptor_sets[update.array_index as usize];
        let write_descriptor_builder = vk::WriteDescriptorSet::builder()
            .dst_set(vk_set)
            .dst_binding(descriptor.binding)
            .dst_array_element(update.dst_element_offset)
            .descriptor_type(descriptor.vk_type);

        log::trace!(
            "update descriptor set {:?} (set_index: {:?} binding: {} name: {:?} type: {:?} array_index: {} first update data index: {} set: {:?})",
            update.descriptor_key,
            descriptor.set_index,
            descriptor.binding,
            descriptor.name,
            descriptor.resource_type,
            update.array_index,
            descriptor_first_update_data,
            vk_set
        );

        match descriptor.resource_type {
            RafxResourceType::SAMPLER => {
                if descriptor.has_immutable_sampler {
                    Err(format!(
                        "Tried to update sampler {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but it is a static/immutable sampler",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                    ))?;
                }

                let samplers = update.elements.samplers.ok_or_else(||
                    format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the samplers element list was None",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                    )
                )?;
                let begin_index =
                    (descriptor_first_update_data + update.dst_element_offset) as usize;
                assert!(begin_index + samplers.len() <= self.update_data.update_data_count);

                // Modify the update data
                let mut next_index = begin_index;
                for sampler in samplers {
                    let image_info = &mut self.update_data.image_infos[next_index];
                    next_index += 1;

                    image_info.sampler = sampler.vk_sampler().unwrap().vk_sampler();
                }

                // Queue a descriptor write
                self.pending_writes.push(
                    write_descriptor_builder
                        .image_info(&self.update_data.image_infos[begin_index..next_index])
                        .build(),
                );
            }
            RafxResourceType::COMBINED_IMAGE_SAMPLER => {
                if !descriptor.has_immutable_sampler {
                    Err(format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the sampler is NOT immutable. This is not currently supported.",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type
                    ))?;
                }

                let textures = update.elements.textures.ok_or_else(||
                    format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the texture element list was None",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                    )
                )?;
                let begin_index =
                    (descriptor_first_update_data + update.dst_element_offset) as usize;
                assert!(begin_index + textures.len() <= self.update_data.update_data_count);

                let texture_bind_type =
                    update.texture_bind_type.unwrap_or(RafxTextureBindType::Srv);

                // Modify the update data
                let mut next_index = begin_index;
                for texture in textures {
                    let image_info = &mut self.update_data.image_infos[next_index];
                    next_index += 1;

                    if texture_bind_type == RafxTextureBindType::SrvStencil {
                        image_info.image_view = texture.vk_texture().unwrap().vk_srv_view_stencil().ok_or_else(|| {
                            format!(
                                "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::SrvStencil but there is no srv_stencil view",
                                update.descriptor_key,
                                descriptor.set_index,
                                descriptor.binding,
                                descriptor.name,
                                descriptor.resource_type,
                            )
                        })?;
                    } else if texture_bind_type == RafxTextureBindType::Srv {
                        image_info.image_view = texture.vk_texture().unwrap().vk_srv_view().ok_or_else(|| {
                            format!(
                                "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::Srv but there is no srv_stencil view",
                                update.descriptor_key,
                                descriptor.set_index,
                                descriptor.binding,
                                descriptor.name,
                                descriptor.resource_type,
                            )
                        })?;
                    } else {
                        Err(format!(
                            "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but texture_bind_type {:?} was unexpected for this kind of resource",
                            update.descriptor_key,
                            descriptor.set_index,
                            descriptor.binding,
                            descriptor.name,
                            descriptor.resource_type,
                            update.texture_bind_type
                        ))?;
                    }

                    image_info.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                }

                // Queue a descriptor write
                self.pending_writes.push(
                    write_descriptor_builder
                        .image_info(&self.update_data.image_infos[begin_index..next_index])
                        .build(),
                );
            }
            RafxResourceType::TEXTURE => {
                let textures = update.elements.textures.ok_or_else(||
                    format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the texture element list was None",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                    )
                )?;
                let begin_index =
                    (descriptor_first_update_data + update.dst_element_offset) as usize;
                assert!(begin_index + textures.len() <= self.update_data.update_data_count);

                let texture_bind_type =
                    update.texture_bind_type.unwrap_or(RafxTextureBindType::Srv);

                // Modify the update data
                let mut next_index = begin_index;
                for texture in textures {
                    let image_info = &mut self.update_data.image_infos[next_index];
                    next_index += 1;

                    if texture_bind_type == RafxTextureBindType::SrvStencil {
                        image_info.image_view = texture.vk_texture().unwrap().vk_srv_view_stencil().ok_or_else(|| {
                            format!(
                                "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::SrvStencil but there is no srv_stencil view",
                                update.descriptor_key,
                                descriptor.set_index,
                                descriptor.binding,
                                descriptor.name,
                                descriptor.resource_type,
                            )
                        })?;
                    } else if texture_bind_type == RafxTextureBindType::Srv {
                        image_info.image_view = texture.vk_texture().unwrap().vk_srv_view().ok_or_else(|| {
                            format!(
                                "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::Srv but there is no srv view",
                                update.descriptor_key,
                                descriptor.set_index,
                                descriptor.binding,
                                descriptor.name,
                                descriptor.resource_type,
                            )
                        })?;
                    } else {
                        Err(format!(
                            "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but texture_bind_type {:?} was unexpected for this kind of resource",
                            update.descriptor_key,
                            descriptor.set_index,
                            descriptor.binding,
                            descriptor.name,
                            descriptor.resource_type,
                            update.texture_bind_type
                        ))?;
                    }

                    image_info.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                }

                // Queue a descriptor write
                self.pending_writes.push(
                    write_descriptor_builder
                        .image_info(&self.update_data.image_infos[begin_index..next_index])
                        .build(),
                );
            }
            RafxResourceType::TEXTURE_READ_WRITE => {
                let textures = update.elements.textures.ok_or_else(||
                    format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the texture element list was None",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                    )
                )?;
                let begin_index =
                    (descriptor_first_update_data + update.dst_element_offset) as usize;
                assert!(begin_index + textures.len() <= self.update_data.update_data_count);

                // Modify the update data
                let mut next_index = begin_index;

                let texture_bind_type = update
                    .texture_bind_type
                    .unwrap_or(RafxTextureBindType::UavMipSlice(0));

                if let RafxTextureBindType::UavMipSlice(slice) = texture_bind_type {
                    for texture in textures {
                        let image_info = &mut self.update_data.image_infos[next_index];
                        next_index += 1;

                        let image_views = texture.vk_texture().unwrap().vk_uav_views();
                        let image_view = *image_views.get(slice as usize).ok_or_else(|| format!(
                            "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the chosen mip slice {} exceeds the mip count of {} in the image",
                            update.descriptor_key,
                            descriptor.set_index,
                            descriptor.binding,
                            descriptor.name,
                            descriptor.resource_type,
                            slice,
                            image_views.len()
                        ))?;
                        image_info.image_view = image_view;

                        image_info.image_layout = vk::ImageLayout::GENERAL;
                    }
                } else if texture_bind_type == RafxTextureBindType::UavMipChain {
                    let texture = textures.first().unwrap();

                    let image_views = texture.vk_texture().unwrap().vk_uav_views();
                    if image_views.len() > descriptor.element_count as usize {
                        Err(format!(
                            "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) using UavMipChain but the mip chain has {} images and the descriptor has {} elements",
                            update.descriptor_key,
                            descriptor.set_index,
                            descriptor.binding,
                            descriptor.name,
                            descriptor.resource_type,
                            image_views.len(),
                            descriptor.element_count
                        ))?;
                    }

                    for image_view in image_views {
                        let image_info = &mut self.update_data.image_infos[next_index];
                        next_index += 1;

                        image_info.image_view = *image_view;
                        image_info.image_layout = vk::ImageLayout::GENERAL;
                    }
                } else {
                    Err(format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but texture_bind_type {:?} was unexpected for this kind of resource",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                        update.texture_bind_type
                    ))?;
                }

                // Queue a descriptor write
                self.pending_writes.push(
                    write_descriptor_builder
                        .image_info(&self.update_data.image_infos[begin_index..next_index])
                        .build(),
                );
            }
            RafxResourceType::UNIFORM_BUFFER
            | RafxResourceType::BUFFER
            | RafxResourceType::BUFFER_READ_WRITE => {
                if descriptor.vk_type == vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC {
                    //TODO: Add support for dynamic uniforms
                    unimplemented!();
                }

                let buffers = update.elements.buffers.ok_or_else(||
                    format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the buffers element list was None",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                    )
                )?;
                let begin_index =
                    (descriptor_first_update_data + update.dst_element_offset) as usize;
                assert!(begin_index + buffers.len() <= self.update_data.update_data_count);

                // Modify the update data
                let mut next_index = begin_index;
                for (buffer_index, buffer) in buffers.iter().enumerate() {
                    let buffer_info = &mut self.update_data.buffer_infos[next_index];
                    next_index += 1;

                    buffer_info.buffer = buffer.vk_buffer().unwrap().vk_buffer();
                    buffer_info.offset = 0;
                    buffer_info.range = vk::WHOLE_SIZE;

                    if let Some(offset_size) = update.elements.buffer_offset_sizes {
                        if offset_size[buffer_index].offset != 0 {
                            buffer_info.offset = offset_size[buffer_index].offset;
                        }

                        if offset_size[buffer_index].size != 0 {
                            buffer_info.range = offset_size[buffer_index].size;
                        }
                    }
                }

                // Queue a descriptor write
                self.pending_writes.push(
                    write_descriptor_builder
                        .buffer_info(&self.update_data.buffer_infos[begin_index..next_index])
                        .build(),
                );
            }
            RafxResourceType::TEXEL_BUFFER | RafxResourceType::TEXEL_BUFFER_READ_WRITE => {
                let buffers = update.elements.buffers.ok_or_else(||
                    format!(
                        "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the buffers element list was None",
                        update.descriptor_key,
                        descriptor.set_index,
                        descriptor.binding,
                        descriptor.name,
                        descriptor.resource_type,
                    )
                )?;
                let begin_index =
                    (descriptor_first_update_data + update.dst_element_offset) as usize;
                assert!(begin_index + buffers.len() <= self.update_data.update_data_count);

                // Modify the update data
                let mut next_index = begin_index;
                for buffer in buffers {
                    let buffer_view = &mut self.update_data.buffer_views[next_index];
                    next_index += 1;

                    if descriptor.resource_type == RafxResourceType::TEXEL_BUFFER {
                        *buffer_view = buffer.vk_buffer().unwrap().vk_uniform_texel_view().ok_or_else(|| {
                            format!(
                                "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but there was no uniform texel view",
                                update.descriptor_key,
                                descriptor.set_index,
                                descriptor.binding,
                                descriptor.name,
                                descriptor.resource_type,
                            )
                        })?;
                    } else {
                        *buffer_view = buffer.vk_buffer().unwrap().vk_storage_texel_view().ok_or_else(|| {
                            format!(
                                "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but there was no storage texel view",
                                update.descriptor_key,
                                descriptor.set_index,
                                descriptor.binding,
                                descriptor.name,
                                descriptor.resource_type,
                            )
                        })?;
                    };
                }

                // Queue a descriptor write
                self.pending_writes.push(
                    write_descriptor_builder
                        .texel_buffer_view(&self.update_data.buffer_views[begin_index..next_index])
                        .build(),
                );
            }
            _ => unimplemented!(),
        }

        Ok(())
    }
}
