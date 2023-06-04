use crate::dx12::descriptor_heap::Dx12DescriptorHeap;
use crate::dx12::{DescriptorSetLayoutInfo, Dx12DescriptorId, RafxDeviceContextDx12};
use crate::*;
use windows::Win32::Graphics::Direct3D12::ID3D12Device;

fn copy_descriptor_handle(
    device: &ID3D12Device,
    src_heap: &Dx12DescriptorHeap,
    src_id: Dx12DescriptorId,
    dst_heap: &Dx12DescriptorHeap,
    dst_id: Dx12DescriptorId,
) {
    debug_assert_eq!(src_heap.heap_type(), dst_heap.heap_type());
    unsafe {
        device.CopyDescriptorsSimple(
            1,
            dst_heap.id_to_cpu_handle(dst_id),
            src_heap.id_to_cpu_handle(src_id),
            src_heap.heap_type(),
        )
    }
}

// struct DescriptorUpdateData {
//     // one per set * elements in each descriptor
//     //image_infos: Vec<vk::DescriptorImageInfo>,
//     //buffer_infos: Vec<vk::DescriptorBufferInfo>,
//     //buffer_views: Vec<vk::BufferView>,
//     //update_data_count: usize,
//     cbv_srv_uav_stride: u32,
//     cbv_srv_uav_first_id: Option<Dx12DescriptorId>,
//     sampler_stride: u32,
//     sampler_first_id: Option<Dx12DescriptorId>,
//
// }

// impl DescriptorUpdateData {
//     // fn new(update_data_count: usize) -> Self {
//     //     DescriptorUpdateData {
//     //         //image_infos: vec![vk::DescriptorImageInfo::default(); update_data_count],
//     //         //buffer_infos: vec![vk::DescriptorBufferInfo::default(); update_data_count],
//     //         //buffer_views: vec![vk::BufferView::default(); update_data_count],
//     //         //update_data_count,
//     //     }
//     // }
// }

#[derive(Copy, Clone, Debug)]
pub struct RafxDescriptorSetTableInfo {
    pub first_id: Dx12DescriptorId,
    pub stride: u32,
    pub root_index: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct RafxDescriptorSetHandleDx12 {
    cbv_srv_uav_descriptor_id: Option<Dx12DescriptorId>,
    sampler_descriptor_id: Option<Dx12DescriptorId>,
    cbv_srv_uav_root_index: u8,
    sampler_root_index: u8,
}

impl RafxDescriptorSetHandleDx12 {
    pub fn cbv_srv_uav_descriptor_id(&self) -> Option<Dx12DescriptorId> {
        self.cbv_srv_uav_descriptor_id
    }

    pub fn sampler_descriptor_id(&self) -> Option<Dx12DescriptorId> {
        self.sampler_descriptor_id
    }

    pub fn cbv_srv_uav_root_index(&self) -> u8 {
        self.cbv_srv_uav_root_index
    }

    pub fn sampler_root_index(&self) -> u8 {
        self.sampler_root_index
    }
}

pub struct RafxDescriptorSetArrayDx12 {
    root_signature: RafxRootSignature,
    set_index: u32,
    // one per set
    //descriptor_sets: Vec<vk::DescriptorSet>,
    //update_data: Vec<UpdateData>,
    //dynamic_size_offset: Option<SizeOffset>,
    //update_data: DescriptorUpdateData,
    //WARNING: This contains pointers into data stored in DescriptorUpdateData, however those
    // vectors are not added/removed from so their addresses will remain stable, even if this
    // struct is moved
    //pending_writes: Vec<vk::WriteDescriptorSet>,

    // cbv_srv_uav_stride: u32,
    // cbv_srv_uav_first_id: Option<Dx12DescriptorId>,
    // sampler_stride: u32,
    // sampler_first_id: Option<Dx12DescriptorId>,

    // cbv_srv_uav_table_root_index: u8,
    // sampler_table_root_index: u8,
    cbv_srv_uav_table_info: Option<RafxDescriptorSetTableInfo>,
    sampler_table_info: Option<RafxDescriptorSetTableInfo>,
    descriptor_set_array_length: usize,
}

impl std::fmt::Debug for RafxDescriptorSetArrayDx12 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDescriptorSetArrayDx12")
            //.field("first_descriptor_set", &self.descriptor_sets[0])
            .field("root_signature", &self.root_signature)
            .field("set_index", &self.set_index)
            //.field("pending_write_count", &self.pending_writes.len())
            .finish()
    }
}

// For *const c_void in vk::WriteDescriptorSet, which always point at contents of vectors in
// update_data that never get resized
unsafe impl Send for RafxDescriptorSetArrayDx12 {}
unsafe impl Sync for RafxDescriptorSetArrayDx12 {}

impl RafxDescriptorSetArrayDx12 {
    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn set_index(&self) -> u32 {
        self.set_index
    }

    pub fn cbv_srv_uav_table_info(&self) -> Option<RafxDescriptorSetTableInfo> {
        // self.cbv_srv_uav_first_id.map(|table_first_id| RafxDescriptorSetTableInfo {
        //     table_first_id,
        //     table_root_index: self.cbv_srv_uav_table_root_index
        // })
        self.cbv_srv_uav_table_info
    }

    pub fn sampler_table_info(&self) -> Option<RafxDescriptorSetTableInfo> {
        self.sampler_table_info
        // self.sampler_first_id.map(|table_first_id| RafxDescriptorSetTableInfo {
        //     table_first_id,
        //     table_root_index: self.sampler_table_root_index
        // })
    }

    // pub fn cbv_srv_uav_first_id_for_set(&self, index: u32) -> Option<Dx12DescriptorId> {
    //     self.cbv_srv_uav_first_id.map(|x| Dx12DescriptorId(x.0 + index * self.cbv_srv_uav_stride))
    // }
    //
    // pub fn sampler_first_id_for_set(&self, index: u32) -> Option<Dx12DescriptorId> {
    //     self.sampler_first_id.map(|x| Dx12DescriptorId(x.0 + index * self.sampler_stride))
    // }

    // pub fn vk_descriptor_set(
    //     &self,
    //     index: u32,
    // ) -> Option<vk::DescriptorSet> {
    //     self.descriptor_sets.get(index as usize).copied()
    // }

    pub fn handle(
        &self,
        index: u32,
    ) -> Option<RafxDescriptorSetHandleDx12> {
        let mut handle = RafxDescriptorSetHandleDx12 {
            cbv_srv_uav_descriptor_id: None,
            sampler_descriptor_id: None,
            cbv_srv_uav_root_index: 0,
            sampler_root_index: 0,
        };

        if let Some(cbv_srv_uav_table_info) = &self.cbv_srv_uav_table_info {
            handle.cbv_srv_uav_descriptor_id = Some(Dx12DescriptorId(
                cbv_srv_uav_table_info.first_id.0 + index * cbv_srv_uav_table_info.stride,
            ));
            handle.cbv_srv_uav_root_index = cbv_srv_uav_table_info.root_index;
        }

        if let Some(sampler_table_info) = &self.sampler_table_info {
            handle.cbv_srv_uav_descriptor_id = Some(Dx12DescriptorId(
                sampler_table_info.first_id.0 + index * sampler_table_info.stride,
            ));
            handle.sampler_root_index = sampler_table_info.root_index;
        }

        Some(handle)
    }

    pub(crate) fn new(
        device_context: &RafxDeviceContextDx12,
        //heap: &RafxDescriptorHeapDx12,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<Self> {
        let root_signature = descriptor_set_array_def
            .root_signature
            .dx12_root_signature()
            .unwrap()
            .clone();

        let layout_index = descriptor_set_array_def.set_index as usize;
        let layout = &root_signature.inner.layouts[layout_index];
        //let cbv_srv_uav_stride = layout.cbv_srv_uav_table_descriptor_count.unwrap_or(0);
        //let sampler_stride = layout.sampler_table_descriptor_count.unwrap_or(0);

        // let mut update_data = DescriptorUpdateData {
        //     sampler_stride,
        //     sampler_first_id: None,
        //     cbv_srv_uav_stride,
        //     cbv_srv_uav_first_id: None
        // };

        let cbv_srv_uav_table_info = if let Some(root_index) = layout.cbv_srv_uav_table_root_index {
            let stride = layout.cbv_srv_uav_table_descriptor_count.unwrap();
            let first_id = device_context.inner.heaps.gpu_cbv_srv_uav_heap.allocate(
                device_context.d3d12_device(),
                stride * descriptor_set_array_def.array_length as u32,
            )?;
            Some(RafxDescriptorSetTableInfo {
                first_id,
                stride,
                root_index,
            })
            //TODO: Init to safe defaults
        } else {
            None
        };

        let sampler_table_info = if let Some(root_index) = layout.sampler_table_root_index {
            let stride = layout.sampler_table_descriptor_count.unwrap();
            let first_id = device_context.inner.heaps.gpu_sampler_heap.allocate(
                device_context.d3d12_device(),
                stride * descriptor_set_array_def.array_length as u32,
            )?;
            Some(RafxDescriptorSetTableInfo {
                first_id,
                stride,
                root_index,
            })
            //TODO: Init to safe defaults
        } else {
            None
        };

        Ok(RafxDescriptorSetArrayDx12 {
            root_signature: RafxRootSignature::Dx12(root_signature),
            set_index: descriptor_set_array_def.set_index,
            cbv_srv_uav_table_info,
            sampler_table_info,
            // cbv_srv_uav_first_id,
            // cbv_srv_uav_stride,
            // cbv_srv_uav_root_index: layout.cbv_srv_uav_table_root_index,
            // sampler_first_id,
            // sampler_stride,
            // sampler_root_index: layout.sampler_table_root_index
            //descriptor_sets,
            //update_data,
            //pending_writes: Vec::default(),
            descriptor_set_array_length: descriptor_set_array_def.array_length,
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
        // Don't need to do anything on flush
        Ok(())
    }

    pub fn queue_descriptor_set_update(
        &mut self,
        update: &RafxDescriptorUpdate,
    ) -> RafxResult<()> {
        let root_signature = self.root_signature.dx12_root_signature().unwrap();
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
        let descriptor = root_signature.descriptor(descriptor_index).unwrap();

        let use_sampler_heap = descriptor.resource_type == RafxResourceType::SAMPLER;
        let table_info = if use_sampler_heap {
            self.sampler_table_info.as_ref().unwrap()
        } else {
            self.cbv_srv_uav_table_info.as_ref().unwrap()
        };
        let update_data_count = table_info.stride as usize * self.descriptor_set_array_length;
        let descriptor_first_update_data =
            descriptor.update_data_offset_in_set.unwrap() + table_info.stride * update.array_index;

        let device_context = self
            .root_signature
            .dx12_root_signature()
            .unwrap()
            .device_context();

        // let descriptor_set_first_update_data = layout.update_data_count * update.array_index;
        // let descriptor_offset = 0; // TODO: if sampler vs. if cbv_srv_uav
        // let stride = layout.update_data_count;
        // let sampler_first_update_data = descriptor.update_data_offset_in_set.unwrap()
        //     + (layout.sampler_table_descriptor_count * update.array_index);
        // let cbv_srv_uav_first_update_data = descriptor.update_data_offset_in_set.unwrap()
        //     + (layout.cbv_srv_uav_table_descriptor_count * update.array_index);

        // log::trace!(
        //     "update descriptor set {:?} (set_index: {:?} binding: {} name: {:?} type: {:?} array_index: {} first update data index: {} set: {:?})",
        //     update.descriptor_key,
        //     descriptor.set_index,
        //     descriptor.binding,
        //     descriptor.name,
        //     descriptor.resource_type,
        //     update.array_index,
        //     descriptor_first_update_data,
        //     vk_set
        // );

        match descriptor.resource_type {
            RafxResourceType::SAMPLER => {
                unimplemented!()
                // if descriptor.has_immutable_sampler {
                //     Err(format!(
                //         "Tried to update sampler {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but it is a static/immutable sampler",
                //         update.descriptor_key,
                //         descriptor.set_index,
                //         descriptor.binding,
                //         descriptor.name,
                //         descriptor.resource_type,
                //     ))?;
                // }
                //
                // let samplers = update.elements.samplers.ok_or_else(||
                //     format!(
                //         "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the samplers element list was None",
                //         update.descriptor_key,
                //         descriptor.set_index,
                //         descriptor.binding,
                //         descriptor.name,
                //         descriptor.resource_type,
                //     )
                // )?;
                // let begin_index =
                //     (descriptor_first_update_data + update.dst_element_offset) as usize;
                // assert!(begin_index + samplers.len() <= update_data_count);
                //
                // // Modify the update data
                // let mut next_index = begin_index;
                // for sampler in samplers {
                //     let image_info = &mut self.update_data.image_infos[next_index];
                //     next_index += 1;
                //
                //     image_info.sampler = sampler.vk_sampler().unwrap().vk_sampler();
                // }
                //
                // // Queue a descriptor write
                // self.pending_writes.push(
                //     write_descriptor_builder
                //         .image_info(&self.update_data.image_infos[begin_index..next_index])
                //         .build(),
                // );
            }
            RafxResourceType::COMBINED_IMAGE_SAMPLER => {
                unimplemented!()
                // if !descriptor.has_immutable_sampler {
                //     Err(format!(
                //         "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the sampler is NOT immutable. This is not currently supported.",
                //         update.descriptor_key,
                //         descriptor.set_index,
                //         descriptor.binding,
                //         descriptor.name,
                //         descriptor.resource_type
                //     ))?;
                // }
                //
                // let textures = update.elements.textures.ok_or_else(||
                //     format!(
                //         "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the texture element list was None",
                //         update.descriptor_key,
                //         descriptor.set_index,
                //         descriptor.binding,
                //         descriptor.name,
                //         descriptor.resource_type,
                //     )
                // )?;
                // let begin_index =
                //     (descriptor_first_update_data + update.dst_element_offset) as usize;
                // assert!(begin_index + textures.len() <= update_data_count);
                //
                // let texture_bind_type =
                //     update.texture_bind_type.unwrap_or(RafxTextureBindType::Srv);
                //
                // // Modify the update data
                // let mut next_index = begin_index;
                // for texture in textures {
                //     let image_info = &mut self.update_data.image_infos[next_index];
                //     next_index += 1;
                //
                //     if texture_bind_type == RafxTextureBindType::SrvStencil {
                //         image_info.image_view = texture.vk_texture().unwrap().vk_srv_view_stencil().ok_or_else(|| {
                //             format!(
                //                 "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::SrvStencil but there is no srv_stencil view",
                //                 update.descriptor_key,
                //                 descriptor.set_index,
                //                 descriptor.binding,
                //                 descriptor.name,
                //                 descriptor.resource_type,
                //             )
                //         })?;
                //     } else if texture_bind_type == RafxTextureBindType::Srv {
                //         image_info.image_view = texture.vk_texture().unwrap().vk_srv_view().ok_or_else(|| {
                //             format!(
                //                 "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::Srv but there is no srv_stencil view",
                //                 update.descriptor_key,
                //                 descriptor.set_index,
                //                 descriptor.binding,
                //                 descriptor.name,
                //                 descriptor.resource_type,
                //             )
                //         })?;
                //     } else {
                //         Err(format!(
                //             "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but texture_bind_type {:?} was unexpected for this kind of resource",
                //             update.descriptor_key,
                //             descriptor.set_index,
                //             descriptor.binding,
                //             descriptor.name,
                //             descriptor.resource_type,
                //             update.texture_bind_type
                //         ))?;
                //     }
                //
                //     image_info.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                // }
                //
                // // Queue a descriptor write
                // self.pending_writes.push(
                //     write_descriptor_builder
                //         .image_info(&self.update_data.image_infos[begin_index..next_index])
                //         .build(),
                // );
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
                assert!(begin_index + textures.len() <= update_data_count);

                let texture_bind_type =
                    update.texture_bind_type.unwrap_or(RafxTextureBindType::Srv);

                // Modify the update data
                let mut next_index = table_info.first_id.0 + begin_index as u32;
                match texture_bind_type {
                    RafxTextureBindType::Srv | RafxTextureBindType::SrvStencil => {
                        for texture in textures {
                            let descriptor_id = Dx12DescriptorId(next_index);
                            next_index += 1;

                            let src_id = texture.dx12_texture().unwrap().srv().unwrap();

                            //println!("Copy descriptor handle {:?} to {:?}", src_id, descriptor_id);
                            copy_descriptor_handle(
                                device_context.d3d12_device(),
                                &device_context.inner.heaps.cbv_srv_uav_heap,
                                src_id,
                                &device_context.inner.heaps.gpu_cbv_srv_uav_heap,
                                descriptor_id,
                            );
                        }
                    }
                    RafxTextureBindType::UavMipChain => {
                        unimplemented!()
                    }
                    RafxTextureBindType::UavMipSlice(_) => {
                        unimplemented!()
                    }
                }

                //     if texture_bind_type == RafxTextureBindType::SrvStencil {
                //         image_info.image_view = texture.vk_texture().unwrap().vk_srv_view_stencil().ok_or_else(|| {
                //             format!(
                //                 "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::SrvStencil but there is no srv_stencil view",
                //                 update.descriptor_key,
                //                 descriptor.set_index,
                //                 descriptor.binding,
                //                 descriptor.name,
                //                 descriptor.resource_type,
                //             )
                //         })?;
                //     } else if texture_bind_type == RafxTextureBindType::Srv {
                //         image_info.image_view = texture.vk_texture().unwrap().vk_srv_view().ok_or_else(|| {
                //             format!(
                //                 "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) as RafxTextureBindType::Srv but there is no srv view",
                //                 update.descriptor_key,
                //                 descriptor.set_index,
                //                 descriptor.binding,
                //                 descriptor.name,
                //                 descriptor.resource_type,
                //             )
                //         })?;
                //     } else {
                //         Err(format!(
                //             "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but texture_bind_type {:?} was unexpected for this kind of resource",
                //             update.descriptor_key,
                //             descriptor.set_index,
                //             descriptor.binding,
                //             descriptor.name,
                //             descriptor.resource_type,
                //             update.texture_bind_type
                //         ))?;
                //     }
                //
                //     image_info.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
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
                assert!(begin_index + textures.len() <= update_data_count);

                // Modify the update data
                let mut next_index = table_info.first_id.0 + begin_index as u32;

                let texture_bind_type = update
                    .texture_bind_type
                    .unwrap_or(RafxTextureBindType::UavMipSlice(0));

                if let RafxTextureBindType::UavMipSlice(slice) = texture_bind_type {
                    for texture in textures {
                        let descriptor_id = Dx12DescriptorId(next_index);
                        next_index += 1;

                        let src_id = texture.dx12_texture().unwrap().uav(slice).ok_or_else(|| format!(
                            "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the chosen mip slice {} exceeds the mip count of {} in the image",
                            update.descriptor_key,
                            descriptor.set_index,
                            descriptor.binding,
                            descriptor.name,
                            descriptor.resource_type,
                            slice,
                            texture.texture_def().mip_count
                        ))?;

                        copy_descriptor_handle(
                            device_context.d3d12_device(),
                            &device_context.inner.heaps.cbv_srv_uav_heap,
                            src_id,
                            &device_context.inner.heaps.gpu_cbv_srv_uav_heap,
                            descriptor_id,
                        );
                    }
                } else if texture_bind_type == RafxTextureBindType::UavMipChain {
                    unimplemented!(); // this might still be broken
                    let texture = textures.first().unwrap();

                    if texture.texture_def().mip_count > descriptor.element_count {
                        Err(format!(
                            "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) using UavMipChain but the mip chain has {} images and the descriptor has {} elements",
                            update.descriptor_key,
                            descriptor.set_index,
                            descriptor.binding,
                            descriptor.name,
                            descriptor.resource_type,
                            texture.texture_def().mip_count,
                            descriptor.element_count
                        ))?;
                    }

                    for slice in 0..texture.texture_def().mip_count {
                        let descriptor_id = Dx12DescriptorId(next_index);
                        next_index += 1;

                        copy_descriptor_handle(
                            device_context.d3d12_device(),
                            &device_context.inner.heaps.cbv_srv_uav_heap,
                            texture.dx12_texture().unwrap().uav(slice).unwrap(),
                            &device_context.inner.heaps.gpu_cbv_srv_uav_heap,
                            descriptor_id,
                        );
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
            }
            RafxResourceType::UNIFORM_BUFFER => {
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
                assert!(begin_index + buffers.len() <= update_data_count);

                // Modify the update data
                let mut next_index = table_info.first_id.0 + begin_index as u32;
                for (buffer_index, buffer) in buffers.iter().enumerate() {
                    let descriptor_id = Dx12DescriptorId(next_index);
                    next_index += 1;

                    if let Some(buffer_offset_sizes) = &update.elements.buffer_offset_sizes {
                        //
                        // For custom offset/range, we emit a CBV directly to the heap
                        //
                        let offset_size = &buffer_offset_sizes[0];
                        assert!(
                            offset_size.size
                                <= super::d3d12::D3D12_REQ_CONSTANT_BUFFER_ELEMENT_COUNT as u64
                                    * 16
                        );
                        assert!(
                            offset_size.size + offset_size.byte_offset <= buffer.buffer_def().size
                        );
                        // Device requires SizeInBytes be a multiple of 256 (STATE_CREATION #650: CREATE_CONSTANT_BUFFER_VIEW_INVALID_DESC)
                        let bound_size = rafx_base::memory::round_size_up_to_alignment_u64(
                            offset_size.size,
                            256,
                        );
                        let buffer_view_desc = super::d3d12::D3D12_CONSTANT_BUFFER_VIEW_DESC {
                            SizeInBytes: bound_size as u32,
                            BufferLocation: buffer.dx12_buffer().unwrap().gpu_address()
                                + offset_size.byte_offset,
                        };
                        let handle = device_context
                            .inner
                            .heaps
                            .gpu_cbv_srv_uav_heap
                            .id_to_cpu_handle(descriptor_id);
                        unsafe {
                            device_context
                                .d3d12_device()
                                .CreateConstantBufferView(Some(&buffer_view_desc), handle);
                        }
                    } else {
                        let src_id = buffer.dx12_buffer().unwrap().cbv().unwrap();
                        //println!("Copy descriptor handle {:?} to {:?}", src_id, descriptor_id);
                        copy_descriptor_handle(
                            device_context.d3d12_device(),
                            &device_context.inner.heaps.cbv_srv_uav_heap,
                            src_id,
                            &device_context.inner.heaps.gpu_cbv_srv_uav_heap,
                            descriptor_id,
                        );
                    }
                }
            }
            RafxResourceType::BUFFER => {
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
                assert!(begin_index + buffers.len() <= update_data_count);

                // Modify the update data
                let mut next_index = table_info.first_id.0 + begin_index as u32;
                for (buffer_index, buffer) in buffers.iter().enumerate() {
                    let descriptor_id = Dx12DescriptorId(next_index);
                    next_index += 1;

                    if let Some(buffer_offset_sizes) = &update.elements.buffer_offset_sizes {
                        //TODO: Support this?
                        unimplemented!();
                    } else {
                        let src_id = buffer.dx12_buffer().unwrap().srv().unwrap();
                        //println!("Copy descriptor handle {:?} to {:?}", src_id, descriptor_id);
                        copy_descriptor_handle(
                            device_context.d3d12_device(),
                            &device_context.inner.heaps.cbv_srv_uav_heap,
                            src_id,
                            &device_context.inner.heaps.gpu_cbv_srv_uav_heap,
                            descriptor_id,
                        );
                    }
                }
            }
            RafxResourceType::BUFFER_READ_WRITE => {
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
                assert!(begin_index + buffers.len() <= update_data_count);

                // Modify the update data
                let mut next_index = table_info.first_id.0 + begin_index as u32;
                for (buffer_index, buffer) in buffers.iter().enumerate() {
                    let descriptor_id = Dx12DescriptorId(next_index);
                    next_index += 1;

                    if let Some(buffer_offset_sizes) = &update.elements.buffer_offset_sizes {
                        //TODO: Support this?
                        unimplemented!();
                    } else {
                        //println!("BIND UAV BUFFER DEF {:?}", buffer.buffer_def());
                        let src_id = buffer.dx12_buffer().unwrap().uav().unwrap();
                        //println!("Copy descriptor handle {:?} to {:?}", src_id, descriptor_id);
                        copy_descriptor_handle(
                            device_context.d3d12_device(),
                            &device_context.inner.heaps.cbv_srv_uav_heap,
                            src_id,
                            &device_context.inner.heaps.gpu_cbv_srv_uav_heap,
                            descriptor_id,
                        );
                    }
                }
            }
            RafxResourceType::TEXEL_BUFFER | RafxResourceType::TEXEL_BUFFER_READ_WRITE => {
                unimplemented!()
                // let buffers = update.elements.buffers.ok_or_else(||
                //     format!(
                //         "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the buffers element list was None",
                //         update.descriptor_key,
                //         descriptor.set_index,
                //         descriptor.binding,
                //         descriptor.name,
                //         descriptor.resource_type,
                //     )
                // )?;
                // let begin_index =
                //     (descriptor_first_update_data + update.dst_element_offset) as usize;
                // assert!(begin_index + buffers.len() <= update_data_count);
                //
                // // Modify the update data
                // let mut next_index = begin_index;
                // for buffer in buffers {
                //     let buffer_view = &mut self.update_data.buffer_views[next_index];
                //     next_index += 1;
                //
                //     if descriptor.resource_type == RafxResourceType::TEXEL_BUFFER {
                //         *buffer_view = buffer.vk_buffer().unwrap().vk_uniform_texel_view().ok_or_else(|| {
                //             format!(
                //                 "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but there was no uniform texel view",
                //                 update.descriptor_key,
                //                 descriptor.set_index,
                //                 descriptor.binding,
                //                 descriptor.name,
                //                 descriptor.resource_type,
                //             )
                //         })?;
                //     } else {
                //         *buffer_view = buffer.vk_buffer().unwrap().vk_storage_texel_view().ok_or_else(|| {
                //             format!(
                //                 "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but there was no storage texel view",
                //                 update.descriptor_key,
                //                 descriptor.set_index,
                //                 descriptor.binding,
                //                 descriptor.name,
                //                 descriptor.resource_type,
                //             )
                //         })?;
                //     };
                // }
                //
                // // Queue a descriptor write
                // self.pending_writes.push(
                //     write_descriptor_builder
                //         .texel_buffer_view(&self.update_data.buffer_views[begin_index..next_index])
                //         .build(),
                // );
            }
            _ => unimplemented!(),
        }

        Ok(())

        //let csu_heap = &root_signature.device_context().inner.heaps.cbv_srv_uav_heap;
        // csu_heap.cpu_visible_heap();
        // csu_heap.gpu_visible_heap();
        // csu_heap.id_to_shader_visible_cpu_handle();
        // csu_heap.id_to_shader_visible_gpu_handle();
        // csu_heap.id_to_cpu_handle();

        //unimplemented!();
        /*

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
            .descriptor_type(descriptor.vk_type.unwrap());

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
                assert!(begin_index + samplers.len() <= update_data_count);

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
                assert!(begin_index + textures.len() <= update_data_count);

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
                assert!(begin_index + textures.len() <= update_data_count);

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
                assert!(begin_index + textures.len() <= update_data_count);

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
                if descriptor.vk_type.unwrap() == vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC {
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
                assert!(begin_index + buffers.len() <= update_data_count);

                // Modify the update data
                let mut next_index = begin_index;
                for (buffer_index, buffer) in buffers.iter().enumerate() {
                    let buffer_info = &mut self.update_data.buffer_infos[next_index];
                    next_index += 1;

                    buffer_info.buffer = buffer.vk_buffer().unwrap().vk_buffer();
                    buffer_info.offset = 0;
                    buffer_info.range = vk::WHOLE_SIZE;

                    if let Some(offset_size) = update.elements.buffer_offset_sizes {
                        if offset_size[buffer_index].byte_offset != 0 {
                            buffer_info.offset = offset_size[buffer_index].byte_offset;
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
                assert!(begin_index + buffers.len() <= update_data_count);

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

         */
    }
}
