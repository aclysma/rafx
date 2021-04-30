use crate::gles2::{BufferId, DescriptorSetLayoutInfo, Gles2BufferContents, RafxDeviceContextGles2, DescriptorInfo, TextureId, RafxTextureGles2, RafxSamplerGles2};
use crate::{RafxDescriptorKey, RafxDescriptorSetArrayDef, RafxDescriptorUpdate, RafxResourceType, RafxResult, RafxRootSignature, RafxTexture, RafxTextureBindType};

use rafx_base::trust_cell::TrustCell;
use std::sync::Arc;

#[derive(Clone)]
pub struct RafxDescriptorSetHandleGles2 {
    descriptor_set_array_data: Arc<TrustCell<DescriptorSetArrayData>>,
    array_index: u32,
}

impl std::fmt::Debug for RafxDescriptorSetHandleGles2 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDescriptorSetHandleGl")
            .field("array_index", &self.array_index)
            .finish()
    }
}

impl RafxDescriptorSetHandleGles2 {
    pub fn descriptor_set_array_data(&self) -> &Arc<TrustCell<DescriptorSetArrayData>> {
        &self.descriptor_set_array_data
    }

    pub fn array_index(&self) -> u32 {
        self.array_index
    }
}

#[derive(Clone)]
pub struct BufferDescriptorState {
    pub(crate) buffer_id: Option<BufferId>,
    pub(crate) buffer_contents: Option<Gles2BufferContents>,
    pub(crate) offset: u64,
    //range: u32,
}

#[derive(Clone)]
pub struct TextureDescriptorState {
    //TODO: does this really need to be a RafxTexture?
    pub(crate) texture: Option<RafxTextureGles2>,
}

#[derive(Clone)]
pub struct SamplerDescriptorState {
    pub(crate) sampler: Option<RafxSamplerGles2>,
}

#[derive(Clone)]
pub struct DescriptorSetArrayData {
    pub(crate) buffer_states_per_set: u32,
    pub(crate) texture_states_per_set: u32,
    pub(crate) sampler_states_per_set: u32,
    pub(crate) buffer_states: Vec<Option<BufferDescriptorState>>,
    pub(crate) texture_states: Vec<Option<TextureDescriptorState>>,
    pub(crate) sampler_states: Vec<Option<SamplerDescriptorState>>,
}

pub struct RafxDescriptorSetArrayGles2 {
    root_signature: RafxRootSignature,
    set_index: u32,
    data: Arc<TrustCell<DescriptorSetArrayData>>,
    array_length: u32,
}

impl RafxDescriptorSetArrayGles2 {
    pub fn root_signature(&self) -> &RafxRootSignature {
        &self.root_signature
    }

    pub fn set_index(&self) -> u32 {
        self.set_index
    }

    pub fn descriptor_set_array_data(&self) -> &Arc<TrustCell<DescriptorSetArrayData>> {
        &self.data
    }

    pub fn handle(
        &self,
        array_index: u32,
    ) -> Option<RafxDescriptorSetHandleGles2> {
        if array_index >= self.array_length {
            return None;
        }

        Some(RafxDescriptorSetHandleGles2 {
            descriptor_set_array_data: self.data.clone(),
            array_index,
        })
    }

    pub(crate) fn new(
        _device_context: &RafxDeviceContextGles2,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<Self> {
        let root_signature = descriptor_set_array_def
            .root_signature
            .gles2_root_signature()
            .unwrap()
            .clone();

        let layout_index = descriptor_set_array_def.set_index as usize;
        let layout = &root_signature.inner.layouts[layout_index];

        let data = DescriptorSetArrayData {
            buffer_states_per_set: layout.buffer_descriptor_state_count,
            texture_states_per_set: layout.texture_descriptor_state_count,
            sampler_states_per_set: layout.sampler_descriptor_state_count,
            buffer_states: vec![None; descriptor_set_array_def.array_length * layout.buffer_descriptor_state_count as usize],
            texture_states: vec![None; descriptor_set_array_def.array_length * layout.texture_descriptor_state_count as usize],
            sampler_states: vec![None; descriptor_set_array_def.array_length * layout.sampler_descriptor_state_count as usize],
        };

        Ok(RafxDescriptorSetArrayGles2 {
            root_signature: RafxRootSignature::Gles2(root_signature),
            set_index: descriptor_set_array_def.set_index,
            data: Arc::new(TrustCell::new(data)),
            array_length: descriptor_set_array_def.array_length as u32,
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
        let root_signature = self.root_signature.gles2_root_signature().unwrap();
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

        log::trace!(
            "update descriptor set {:?} (set_index: {:?} binding: {} name: {:?} type: {:?} array_index: {})",
            update.descriptor_key,
            descriptor.set_index,
            descriptor.binding,
            descriptor.name,
            descriptor.resource_type,
            update.array_index,
        );

        let mut descriptor_set_data = self.data.borrow_mut();

        match descriptor.resource_type {
            RafxResourceType::SAMPLER => {
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

                let mut next_index = Self::first_sampler_index(layout, update, descriptor);
                for sampler in samplers {
                    descriptor_set_data.sampler_states[next_index as usize] =
                        Some(SamplerDescriptorState {
                            sampler: Some(sampler.gles2_sampler().unwrap().clone()),
                        });

                    next_index += 1;
                }
            }
            RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE => {
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


                // Modify the update data
                let mut next_index = Self::first_texture_index(layout, update, descriptor);
                for texture in textures {
                    descriptor_set_data.texture_states[next_index as usize] =
                        Some(TextureDescriptorState {
                            texture: Some(texture.gles2_texture().unwrap().clone()),
                        });

                    next_index += 1;
                }

                //TODO: Do we need to support these? Maybe not for GL ES 2.0

                // Defaults to UavMipSlice(0) for TEXTURE_READ_WRITE and Srv for TEXTURE
                // let texture_bind_type =
                //     if descriptor.resource_type == RafxResourceType::TEXTURE_READ_WRITE {
                //         update
                //             .texture_bind_type
                //             .unwrap_or(RafxTextureBindType::UavMipSlice(0))
                //     } else {
                //         update.texture_bind_type.unwrap_or(RafxTextureBindType::Srv)
                //     };

                //         let mut next_index = begin_index;
                //         if let RafxTextureBindType::UavMipSlice(slice) = texture_bind_type {
                //             for texture in textures {
                //                 let uav_views =
                //                     texture.gl_texture().unwrap().gl_mip_level_uav_views();
                //                 let uav_view = uav_views.get(slice as usize).ok_or_else(|| format!(
                //                     "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but the chosen mip slice {} exceeds the mip count of {} in the image",
                //                     update.descriptor_key,
                //                     descriptor.set_index,
                //                     descriptor.binding,
                //                     descriptor.name,
                //                     descriptor.resource_type,
                //                     slice,
                //                     uav_views.len()
                //                 ))?;
                //
                //                 argument_buffer
                //                     .encoder
                //                     .set_texture(next_index as _, uav_view);
                //                 descriptor_resource_pointers[next_index] =
                //                     (uav_view as &gl_rs::ResourceRef).as_ptr();
                //                 next_index += 1;
                //             }
                //         } else if texture_bind_type == RafxTextureBindType::UavMipChain {
                //             let texture = textures.first().unwrap();
                //
                //             let uav_views = texture.gl_texture().unwrap().gl_mip_level_uav_views();
                //             if uav_views.len() > descriptor.element_count as usize {
                //                 Err(format!(
                //                     "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) using UavMipChain but the mip chain has {} images and the descriptor has {} elements",
                //                     update.descriptor_key,
                //                     descriptor.set_index,
                //                     descriptor.binding,
                //                     descriptor.name,
                //                     descriptor.resource_type,
                //                     uav_views.len(),
                //                     descriptor.element_count
                //                 ))?;
                //             }
                //
                //             for uav_view in uav_views {
                //                 argument_buffer
                //                     .encoder
                //                     .set_texture(next_index as _, uav_view);
                //                 descriptor_resource_pointers[next_index] =
                //                     (uav_view as &gl_rs::ResourceRef).as_ptr();
                //                 next_index += 1;
                //             }
                //         } else if texture_bind_type == RafxTextureBindType::Srv
                //             || texture_bind_type == RafxTextureBindType::SrvStencil
                //         {
                //             for texture in textures {
                //                 let gl_texture = texture.gl_texture().unwrap().gl_texture();
                //                 argument_buffer
                //                     .encoder
                //                     .set_texture(next_index as _, gl_texture);
                //                 descriptor_resource_pointers[next_index] =
                //                     (gl_texture as &gl_rs::ResourceRef).as_ptr();
                //                 next_index += 1;
                //             }
                //         } else {
                //             Err(format!(
                //                 "Tried to update binding {:?} (set: {:?} binding: {} name: {:?} type: {:?}) but texture_bind_type {:?} was unexpected for this kind of resource",
                //                 update.descriptor_key,
                //                 descriptor.set_index,
                //                 descriptor.binding,
                //                 descriptor.name,
                //                 descriptor.resource_type,
                //                 update.texture_bind_type
                //             ))?;
                //         }
            }
            RafxResourceType::UNIFORM_BUFFER
            | RafxResourceType::BUFFER
            | RafxResourceType::BUFFER_READ_WRITE => {
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

                // Modify the update data
                let mut next_index = Self::first_buffer_index(layout, update, descriptor);
                for (buffer_index, buffer) in buffers.iter().enumerate() {
                    let offset = update
                        .elements
                        .buffer_offset_sizes
                        .map(|x| x[buffer_index].byte_offset)
                        .unwrap_or(0);

                    let gl_buffer = buffer.gles2_buffer().unwrap();
                    descriptor_set_data.buffer_states[next_index as usize] =
                        Some(BufferDescriptorState {
                            buffer_contents: gl_buffer.buffer_contents().clone(),
                            buffer_id: gl_buffer.gl_buffer_id(),
                            offset,
                        });

                    next_index += 1;
                }
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    fn first_buffer_index(layout: &DescriptorSetLayoutInfo, update: &RafxDescriptorUpdate, descriptor: &DescriptorInfo) -> u32 {
        assert!(update.dst_element_offset + update.elements.buffers.as_ref().unwrap().len() as u32 <= descriptor.element_count);
        layout.buffer_descriptor_state_count * update.array_index + descriptor.descriptor_data_offset_in_set.unwrap() + update.dst_element_offset
    }

    fn first_texture_index(layout: &DescriptorSetLayoutInfo, update: &RafxDescriptorUpdate, descriptor: &DescriptorInfo) -> u32 {
        assert!(update.dst_element_offset + update.elements.textures.as_ref().unwrap().len() as u32 <= descriptor.element_count);
        layout.texture_descriptor_state_count * update.array_index + descriptor.descriptor_data_offset_in_set.unwrap() + update.dst_element_offset
    }

    fn first_sampler_index(layout: &DescriptorSetLayoutInfo, update: &RafxDescriptorUpdate, descriptor: &DescriptorInfo) -> u32 {
        assert!(update.dst_element_offset + update.elements.samplers.as_ref().unwrap().len() as u32 <= descriptor.element_count);
        layout.sampler_descriptor_state_count * update.array_index + descriptor.descriptor_data_offset_in_set.unwrap() + update.dst_element_offset
    }
}

impl std::fmt::Debug for RafxDescriptorSetArrayGles2 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDescriptorSetArrayGl")
            .field("root_signature", &self.root_signature)
            .field("set_index", &self.set_index)
            .field("array_length", &self.array_length)
            .finish()
    }
}
