use crate::DecodedImage;
use crate::DecodedImageColorSpace;
use crate::DecodedImageMips;
use rafx_api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx_api::{
    RafxBarrierQueueTransition, RafxCmdCopyBufferToTextureParams, RafxDeviceContext, RafxExtents3D,
    RafxFormat, RafxQueue, RafxResourceState, RafxResourceType, RafxResult, RafxSampleCount,
    RafxTexture, RafxTextureBarrier, RafxTextureDef, RafxTextureDimensions,
};

// This custom path for metal can be removed after I implement cmd_blit
#[cfg(feature = "rafx-metal")]
fn generate_mips_for_image(
    upload: &mut RafxTransferUpload,
    texture: &RafxTexture,
    layer: u32,
    _mip_level_count: u32,
) -> RafxResult<()> {
    upload
        .transfer_command_buffer()
        .metal_command_buffer()
        .unwrap()
        .end_current_encoders(false)?;
    let blit_encoder = upload
        .transfer_command_buffer()
        .metal_command_buffer()
        .unwrap()
        .metal_command_buffer()
        .unwrap()
        .new_blit_command_encoder();
    blit_encoder.generate_mipmaps(texture.metal_texture().unwrap().metal_texture());
    blit_encoder.end_encoding();

    upload.transfer_command_buffer().cmd_resource_barrier(
        &[],
        &[RafxTextureBarrier {
            texture,
            src_state: RafxResourceState::COPY_DST,
            dst_state: RafxResourceState::SHADER_RESOURCE,
            queue_transition: RafxBarrierQueueTransition::ReleaseTo(
                upload.dst_queue().queue_type(),
            ),
            array_slice: Some(layer as u16),
            mip_slice: Some(0),
        }],
        &[],
    )?;

    upload.dst_command_buffer().cmd_resource_barrier(
        &[],
        &[RafxTextureBarrier {
            texture,
            src_state: RafxResourceState::COPY_DST,
            dst_state: RafxResourceState::SHADER_RESOURCE,
            queue_transition: RafxBarrierQueueTransition::AcquireFrom(
                upload.transfer_queue().queue_type(),
            ),
            array_slice: Some(layer as u16),
            mip_slice: Some(0),
        }],
        &[],
    )?;

    return Ok(());
}

#[cfg(not(feature = "rafx-metal"))]
fn generate_mips_for_image(
    upload: &mut RafxTransferUpload,
    texture: &RafxTexture,
    layer: u32,
    mip_level_count: u32,
) -> RafxResult<()> {
    //
    // Transition the first mip range to COPY_SRC, on the graphics queue
    //
    upload.transfer_command_buffer().cmd_resource_barrier(
        &[],
        &[RafxTextureBarrier {
            texture,
            src_state: RafxResourceState::COPY_DST,
            dst_state: RafxResourceState::COPY_SRC,
            queue_transition: RafxBarrierQueueTransition::ReleaseTo(
                upload.dst_queue().queue_type(),
            ),
            array_slice: Some(layer as u16),
            mip_slice: Some(0),
        }],
        &[],
    )?;

    upload.dst_command_buffer().cmd_resource_barrier(
        &[],
        &[RafxTextureBarrier {
            texture,
            src_state: RafxResourceState::COPY_DST,
            dst_state: RafxResourceState::COPY_SRC,
            queue_transition: RafxBarrierQueueTransition::AcquireFrom(
                upload.transfer_queue().queue_type(),
            ),
            array_slice: Some(layer as u16),
            mip_slice: Some(0),
        }],
        &[],
    )?;

    do_generate_mips_for_image(upload.dst_command_buffer(), texture, layer, mip_level_count)?;

    //
    // Transition all mips to final layout
    //
    upload.dst_command_buffer().cmd_resource_barrier(
        &[],
        &[RafxTextureBarrier {
            texture,
            src_state: RafxResourceState::COPY_SRC,
            dst_state: RafxResourceState::SHADER_RESOURCE,
            queue_transition: RafxBarrierQueueTransition::None,
            array_slice: Some(layer as u16),
            mip_slice: None,
        }],
        &[],
    )
}

#[cfg(not(feature = "rafx-metal"))]
fn do_generate_mips_for_image(
    command_buffer: &rafx_api::RafxCommandBuffer,
    texture: &RafxTexture,
    layer: u32,
    mip_level_count: u32,
) -> RafxResult<()> {
    log::debug!("Generating mipmaps");

    let texture_def = texture.texture_def();

    // Walk through each mip level n:
    // - put level n+1 into write mode
    // - blit from n to n+1
    // - put level n+1 into read mode
    for dst_level in 1..mip_level_count {
        log::trace!("Generating mipmap level {}", dst_level);
        let src_level = dst_level - 1;

        //
        // Move DST mip into COPY_DST state
        //
        command_buffer.cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture,
                src_state: RafxResourceState::UNDEFINED,
                dst_state: RafxResourceState::COPY_DST,
                queue_transition: RafxBarrierQueueTransition::None,
                mip_slice: Some(dst_level as u8),
                array_slice: Some(layer as u16),
            }],
            &[],
        )?;

        let src_extents = [
            RafxExtents3D::default(),
            RafxExtents3D {
                width: (texture_def.extents.width >> src_level).max(1),
                height: (texture_def.extents.height >> src_level).max(1),
                depth: 1,
            },
        ];

        let dst_extents = [
            RafxExtents3D::default(),
            RafxExtents3D {
                width: (texture_def.extents.width >> dst_level).max(1),
                height: (texture_def.extents.height >> dst_level).max(1),
                depth: 1,
            },
        ];

        log::trace!("src {:?}", src_extents[1]);
        log::trace!("dst {:?}", dst_extents[1]);

        command_buffer.cmd_blit(
            texture,
            texture,
            &rafx_api::RafxCmdBlitParams {
                src_mip_level: src_level as u8,
                dst_mip_level: dst_level as u8,
                src_extents,
                dst_extents,
                src_state: RafxResourceState::COPY_SRC,
                dst_state: RafxResourceState::COPY_DST,
                array_slices: Some([layer as u16, layer as u16]),
            },
        )?;

        //
        // Move the DST mip into COPY_SRC so that we can copy from it into the next mip
        //
        command_buffer.cmd_resource_barrier(
            &[],
            &[RafxTextureBarrier {
                texture,
                src_state: RafxResourceState::COPY_DST,
                dst_state: RafxResourceState::COPY_SRC,
                queue_transition: RafxBarrierQueueTransition::None,
                mip_slice: Some(dst_level as u8),
                array_slice: Some(layer as u16),
            }],
            &[],
        )?;
    }

    Ok(())
}

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
            &[],
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

        if generate_mips {
            generate_mips_for_image(upload, &texture, layer_index as u32, mip_level_count)?;
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
                    array_slice: Some(layer_index as u16),
                    mip_slice: None,
                }],
                &[],
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
                    array_slice: Some(layer_index as u16),
                    mip_slice: None,
                }],
                &[],
            )?;
        }
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
