use crate::DecodedImage;
use crate::DecodedImageColorSpace;
use crate::DecodedImageMips;
use rafx_api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx_api::{
    RafxBarrierQueueTransition, RafxCmdCopyBufferToTextureParams, RafxDeviceContext, RafxExtents3D,
    RafxFormat, RafxQueue, RafxResourceState, RafxResourceType, RafxSampleCount, RafxTexture,
    RafxTextureBarrier, RafxTextureDef, RafxTextureDimensions,
};

// This function is a little more complex to use than enqueue_load_images but can support cubemaps
// We create a layer for each layer_image_assignment, and copy from the decoded_image
// at the index matching the assignment
pub fn enqueue_load_layered_image_2d(
    device_context: &RafxDeviceContext,
    upload: &mut RafxTransferUpload,
    // transfer_queue_family_index: u32,
    // dst_queue_family_index: u32,
    decoded_images: &[DecodedImage],
    layer_image_assignments: &[usize],
    resource_type: RafxResourceType,
) -> Result<RafxTexture, RafxUploadError> {
    // All images must have identical mip level count
    #[cfg(debug_assertions)]
    {
        let first = &decoded_images[0];
        for decoded_image in decoded_images {
            assert_eq!(first.mips, decoded_image.mips);
            assert_eq!(first.width, decoded_image.width);
            assert_eq!(first.height, decoded_image.height);
            assert_eq!(first.color_space, decoded_image.color_space);
            assert_eq!(first.data.len(), decoded_image.data.len());
        }
    }

    // Arbitrary, not sure if there is any requirement
    const REQUIRED_ALIGNMENT: usize = 16;

    // Check ahead of time if there is space since we are uploading multiple images
    let has_space_available = upload.has_space_available(
        decoded_images[0].data.len(),
        REQUIRED_ALIGNMENT,
        decoded_images.len(),
    );
    if !has_space_available {
        Err(RafxUploadError::BufferFull)?;
    }

    let (mip_level_count, generate_mips) = match decoded_images[0].mips {
        DecodedImageMips::None => (1, false),
        DecodedImageMips::Precomputed(_mip_count) => unimplemented!(), //(info.mip_level_count, false),
        DecodedImageMips::Runtime(mip_count) => (mip_count, mip_count > 1),
    };

    // Push all images into the staging buffer
    let mut layer_offsets = Vec::default();
    for decoded_image in decoded_images {
        layer_offsets.push(upload.push(&decoded_image.data, REQUIRED_ALIGNMENT)?);
    }

    let format = match decoded_images[0].color_space {
        DecodedImageColorSpace::Linear => RafxFormat::R8G8B8A8_UNORM,
        DecodedImageColorSpace::Srgb => RafxFormat::R8G8B8A8_SRGB,
    };

    let texture = device_context.create_texture(&RafxTextureDef {
        extents: RafxExtents3D {
            width: decoded_images[0].width,
            height: decoded_images[0].height,
            depth: 1,
        },
        array_length: layer_image_assignments.len() as u32,
        mip_count: mip_level_count,
        sample_count: RafxSampleCount::SampleCount1,
        format,
        resource_type,
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

    for (layer_index, image_index) in layer_image_assignments.iter().enumerate() {
        upload
            .transfer_command_buffer()
            .cmd_copy_buffer_to_texture(
                upload.staging_buffer(),
                &texture,
                &RafxCmdCopyBufferToTextureParams {
                    buffer_offset: layer_offsets[*image_index],
                    array_layer: layer_index as u16,
                    mip_level: 0,
                },
            )
            .unwrap();
    }

    if generate_mips {
        //
        // Copy mip level 0 into the image
        //
        for (layer_index, image_index) in layer_image_assignments.iter().enumerate() {
            upload
                .transfer_command_buffer()
                .cmd_copy_buffer_to_texture(
                    upload.staging_buffer(),
                    &texture,
                    &RafxCmdCopyBufferToTextureParams {
                        buffer_offset: layer_offsets[*image_index],
                        array_layer: layer_index as u16,
                        mip_level: 0,
                    },
                )
                .unwrap();
        }

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

pub fn enqueue_load_image(
    device_context: &RafxDeviceContext,
    upload: &mut RafxTransferUpload,
    decoded_image: &DecodedImage,
    resource_type: RafxResourceType,
) -> Result<RafxTexture, RafxUploadError> {
    enqueue_load_layered_image_2d(
        device_context,
        upload,
        std::slice::from_ref(decoded_image),
        &[0],
        resource_type,
    )
}

pub fn load_layered_image_2d_blocking(
    device_context: &RafxDeviceContext,
    transfer_queue: &RafxQueue,
    dst_queue: &RafxQueue,
    decoded_images: &[DecodedImage],
    layer_image_assignments: &[usize],
    upload_buffer_max_size: u64,
    resource_type: RafxResourceType,
) -> Result<RafxTexture, RafxUploadError> {
    let mut upload = RafxTransferUpload::new(
        device_context,
        transfer_queue,
        dst_queue,
        upload_buffer_max_size,
    )?;

    let texture = enqueue_load_layered_image_2d(
        device_context,
        &mut upload,
        decoded_images,
        layer_image_assignments,
        resource_type,
    )?;

    upload.block_until_upload_complete()?;

    Ok(texture)
}

pub fn load_image_blocking(
    device_context: &RafxDeviceContext,
    transfer_queue: &RafxQueue,
    dst_queue: &RafxQueue,
    decoded_image: &DecodedImage,
    upload_buffer_max_size: u64,
    resource_type: RafxResourceType,
) -> Result<RafxTexture, RafxUploadError> {
    let mut upload = RafxTransferUpload::new(
        device_context,
        transfer_queue,
        dst_queue,
        upload_buffer_max_size,
    )?;

    let texture = enqueue_load_image(device_context, &mut upload, decoded_image, resource_type)?;

    upload.block_until_upload_complete()?;

    Ok(texture)
}
