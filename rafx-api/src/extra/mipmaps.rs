use crate::{RafxCommandBuffer, RafxResult, RafxTexture};

#[cfg(feature = "rafx-metal")]
use crate::metal::RafxCommandBufferMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxCommandBufferVulkan;

#[cfg(feature = "rafx-vulkan")]
use crate::{
    RafxBarrierQueueTransition, RafxCmdBlitParams, RafxExtents3D, RafxResourceState,
    RafxTextureBarrier,
};

/// The max number of mip levels an image can have given its size
pub fn mip_level_max_count_for_image_size(
    width: u32,
    height: u32,
) -> u32 {
    let max_dimension = std::cmp::max(width, height);
    (max_dimension as f32).log2().floor() as u32 + 1
}

// Texture must be in COPY_SRC state
// After this call, it will be in COPY_DST state
// Vulkan requires this on a graphics queue. Metal allows this on any queue.
pub fn generate_mipmaps(
    command_buffer: &RafxCommandBuffer,
    _texture: &RafxTexture,
) -> RafxResult<()> {
    match command_buffer {
        #[cfg(feature = "rafx-vulkan")]
        RafxCommandBuffer::Vk(inner) => generate_mipmaps_vk(inner, _texture),
        #[cfg(feature = "rafx-metal")]
        RafxCommandBuffer::Metal(inner) => generate_mipmaps_metal(inner, _texture),
        #[cfg(any(
            feature = "rafx-empty",
            not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
        ))]
        RafxCommandBuffer::Empty(_) => unimplemented!(),
    }
}

// This custom path for metal can be removed after I implement cmd_blit
#[cfg(feature = "rafx-metal")]
fn generate_mipmaps_metal(
    command_buffer: &RafxCommandBufferMetal,
    texture: &RafxTexture,
) -> RafxResult<()> {
    log::trace!("Generating mipmaps");
    command_buffer.end_current_encoders(false)?;
    let blit_encoder = command_buffer
        .metal_command_buffer()
        .unwrap()
        .new_blit_command_encoder();
    blit_encoder.generate_mipmaps(texture.metal_texture().unwrap().metal_texture());
    blit_encoder.end_encoding();

    return Ok(());
}

#[cfg(feature = "rafx-vulkan")]
fn generate_mipmaps_vk(
    command_buffer: &RafxCommandBufferVulkan,
    texture: &RafxTexture,
) -> RafxResult<()> {
    let mip_level_count = texture.texture_def().mip_count;

    for layer in 0..texture.texture_def().array_length {
        do_generate_mipmaps_vk(command_buffer, texture, layer, mip_level_count)?;
    }

    Ok(())
}

#[cfg(feature = "rafx-vulkan")]
fn do_generate_mipmaps_vk(
    command_buffer: &RafxCommandBufferVulkan,
    texture: &RafxTexture,
    layer: u32,
    mip_level_count: u32,
) -> RafxResult<()> {
    log::debug!("Generating mipmaps");

    let texture_def = texture.texture_def();
    let vk_texture = texture.vk_texture().unwrap();

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

        command_buffer.cmd_blit_image(
            texture.vk_texture().unwrap(),
            vk_texture,
            &RafxCmdBlitParams {
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
        )?;
    }

    Ok(())
}
