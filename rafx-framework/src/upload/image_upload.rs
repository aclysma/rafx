use crate::upload::gpu_image_data::GpuImageData;
use rafx_api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx_api::{
    RafxBarrierQueueTransition, RafxCmdCopyBufferToTextureParams, RafxDeviceContext, RafxExtents3D,
    RafxQueue, RafxResourceState, RafxResourceType, RafxSampleCount, RafxTexture,
    RafxTextureBarrier, RafxTextureDef, RafxTextureDimensions,
};

pub struct ImageUploadParams<'a> {
    pub resource_type: RafxResourceType,
    pub generate_mips: bool,
    pub layer_swizzle: Option<&'a [u32]>,
}

impl<'a> Default for ImageUploadParams<'a> {
    fn default() -> Self {
        ImageUploadParams {
            resource_type: RafxResourceType::TEXTURE,
            generate_mips: false,
            layer_swizzle: None,
        }
    }
}

// This function is a little more complex to use than enqueue_load_images but can support cubemaps
// We create a layer for each layer_image_assignment, and copy from the decoded_image
// at the index matching the assignment
pub fn enqueue_load_image(
    device_context: &RafxDeviceContext,
    upload: &mut RafxTransferUpload,
    image_data: &GpuImageData,
    params: ImageUploadParams,
) -> Result<RafxTexture, RafxUploadError> {
    // All images must have identical mip level count, sizes, etc.
    #[cfg(debug_assertions)]
    image_data.verify_state();

    //
    // Determine the total amount of data we need to upload and verify there is enough space
    //
    let bytes_required = image_data.total_size(
        device_context.device_info().upload_texture_alignment,
        device_context.device_info().upload_texture_row_alignment,
    );

    let has_space_available = upload.has_space_available(
        bytes_required as usize,
        device_context.device_info().upload_texture_alignment as usize,
        1,
    );

    if !has_space_available {
        Err(RafxUploadError::BufferFull)?;
    }

    //
    // Determine mip count
    //
    let mip_count = if params.generate_mips {
        rafx_api::extra::mipmaps::mip_level_max_count_for_image_size(
            image_data.width,
            image_data.height,
        )
    } else {
        image_data.layers[0].mip_levels.len() as u32
    };

    //
    // Push all image layers/levels into the staging buffer, keeping note of offsets within the
    // buffer where each resource is stored
    //
    let mut layer_offsets = Vec::default();
    for layer in &image_data.layers {
        let mut level_offsets = Vec::default();
        for level in &layer.mip_levels {
            let level_block_width = rafx_base::memory::round_size_up_to_alignment_u32(
                level.width,
                image_data.format.block_width_in_pixels(),
            ) / image_data.format.block_width_in_pixels();
            let level_block_height = rafx_base::memory::round_size_up_to_alignment_u32(
                level.height,
                image_data.format.block_height_in_pixels(),
            ) / image_data.format.block_height_in_pixels();

            // A block format's row may be multiple pixels high
            let row_size_in_bytes =
                level_block_width * image_data.format.block_or_pixel_size_in_bytes();
            for row_index in 0..level_block_height {
                let alignment = if row_index == 0 {
                    device_context.device_info().upload_texture_alignment
                } else {
                    device_context.device_info().upload_texture_row_alignment
                };

                let offset = upload.push(
                    &level.data[((row_size_in_bytes * row_index) as usize)
                        ..((row_size_in_bytes * (row_index + 1)) as usize)],
                    alignment as usize,
                )?;

                if row_index == 0 {
                    level_offsets.push(offset);
                }
            }
        }
        layer_offsets.push(level_offsets);
    }

    // If we are swizzling layers, we create a layer per layer_swizzle entry. Otherwise, we use
    // the number of layers in the image data
    let layer_count = params
        .layer_swizzle
        .map(|x| x.len())
        .unwrap_or_else(|| image_data.layers.len()) as u32;

    //
    // Create the texture
    //
    assert!(mip_count > 0);
    let texture = device_context.create_texture(&RafxTextureDef {
        extents: RafxExtents3D {
            width: image_data.width,
            height: image_data.height,
            depth: 1,
        },
        array_length: layer_count,
        mip_count,
        sample_count: RafxSampleCount::SampleCount1,
        format: image_data.format,
        resource_type: params.resource_type,
        dimensions: RafxTextureDimensions::Dim2D,
    })?;

    //
    // Write into the transfer command buffer
    // - transition destination memory to receive the data
    // - copy the data
    // - transition the destination to the graphics queue
    //

    upload
        .transfer_command_buffer()
        .cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture: &texture,
                src_state: RafxResourceState::UNDEFINED,
                dst_state: RafxResourceState::COPY_DST,
                queue_transition: RafxBarrierQueueTransition::None,
                array_slice: None,
                mip_slice: None,
            }],
        )
        .unwrap();

    for dst_layer_index in 0..layer_count {
        let src_layer_index = if let Some(layer_swizzle) = params.layer_swizzle {
            layer_swizzle[dst_layer_index as usize] as usize
        } else {
            dst_layer_index as usize
        };

        for level_index in 0..image_data.layers[src_layer_index].mip_levels.len() {
            upload
                .transfer_command_buffer()
                .cmd_copy_buffer_to_texture(
                    upload.staging_buffer(),
                    &texture,
                    &RafxCmdCopyBufferToTextureParams {
                        buffer_offset: layer_offsets[src_layer_index][level_index],
                        array_layer: dst_layer_index as u16,
                        mip_level: level_index as u8,
                    },
                )
                .unwrap();
        }
    }

    log::debug!(
        "upload image {}x{} format {:?} layers: {} levels: {} generate mips: {} resource type: {:?}",
        image_data.width,
        image_data.height,
        image_data.format,
        layer_count,
        mip_count,
        params.generate_mips,
        params.resource_type
    );

    if params.generate_mips && mip_count > 1 {
        //
        // Transition the first mip range to COPY_SRC on graphics queue (release)
        //
        upload.transfer_command_buffer().cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture: &texture,
                src_state: RafxResourceState::COPY_DST,
                dst_state: RafxResourceState::COPY_SRC,
                queue_transition: RafxBarrierQueueTransition::ReleaseTo(
                    upload.dst_queue().queue_type(),
                ),
                array_slice: None,
                mip_slice: Some(0),
            }],
        )?;

        //
        // Transition the first mip range to COPY_SRC on graphics queue (acquire)
        //
        upload.dst_command_buffer().cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture: &texture,
                src_state: RafxResourceState::COPY_DST,
                dst_state: RafxResourceState::COPY_SRC,
                queue_transition: RafxBarrierQueueTransition::AcquireFrom(
                    upload.transfer_queue().queue_type(),
                ),
                array_slice: None,
                mip_slice: Some(0),
            }],
        )?;

        rafx_api::extra::mipmaps::generate_mipmaps(upload.dst_command_buffer(), &texture)?;

        //
        // Transition everything to the final layout
        //
        upload.dst_command_buffer().cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture: &texture,
                src_state: RafxResourceState::COPY_SRC,
                dst_state: RafxResourceState::SHADER_RESOURCE,
                queue_transition: RafxBarrierQueueTransition::None,
                array_slice: None,
                mip_slice: None,
            }],
        )?;
    } else {
        upload.transfer_command_buffer().cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture: &texture,
                src_state: RafxResourceState::COPY_DST,
                dst_state: RafxResourceState::SHADER_RESOURCE,
                queue_transition: RafxBarrierQueueTransition::ReleaseTo(
                    upload.dst_queue().queue_type(),
                ),
                array_slice: None,
                mip_slice: None,
            }],
        )?;

        upload.dst_command_buffer().cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture: &texture,
                src_state: RafxResourceState::COPY_DST,
                dst_state: RafxResourceState::SHADER_RESOURCE,
                queue_transition: RafxBarrierQueueTransition::AcquireFrom(
                    upload.transfer_queue().queue_type(),
                ),
                array_slice: None,
                mip_slice: None,
            }],
        )?;
    }

    Ok(texture)
}

pub fn load_image_blocking(
    device_context: &RafxDeviceContext,
    transfer_queue: &RafxQueue,
    dst_queue: &RafxQueue,
    upload_buffer_max_size: u64,
    image_data: &GpuImageData,
    params: ImageUploadParams,
) -> Result<RafxTexture, RafxUploadError> {
    let total_size = image_data.total_size(
        device_context.device_info().upload_texture_alignment,
        device_context.device_info().upload_texture_row_alignment,
    );
    if upload_buffer_max_size < total_size {
        Err(RafxUploadError::BufferFull)?;
    }

    let mut upload = RafxTransferUpload::new(
        device_context,
        transfer_queue,
        dst_queue,
        upload_buffer_max_size,
        None,
    )?;

    let texture = enqueue_load_image(device_context, &mut upload, image_data, params)?;

    upload.block_until_upload_complete()?;

    Ok(texture)
}
