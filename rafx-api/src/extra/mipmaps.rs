use crate::{RafxCommandBuffer, RafxResult, RafxTexture};

#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxCommandBufferDx12;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxCommandBufferGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxCommandBufferGles3;
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
        #[cfg(feature = "rafx-dx12")]
        RafxCommandBuffer::Dx12(inner) => generate_mipmaps_dx12(inner, _texture),
        #[cfg(feature = "rafx-vulkan")]
        RafxCommandBuffer::Vk(inner) => generate_mipmaps_vk(inner, _texture),
        #[cfg(feature = "rafx-metal")]
        RafxCommandBuffer::Metal(inner) => generate_mipmaps_metal(inner, _texture),
        #[cfg(feature = "rafx-gles2")]
        RafxCommandBuffer::Gles2(inner) => generate_mipmaps_gles2(inner, _texture),
        #[cfg(feature = "rafx-gles3")]
        RafxCommandBuffer::Gles3(inner) => generate_mipmaps_gles3(inner, _texture),
        #[cfg(any(
            feature = "rafx-empty",
            not(any(
                feature = "rafx-dx12",
                feature = "rafx-metal",
                feature = "rafx-vulkan",
                feature = "rafx-gles2",
                feature = "rafx-gles3"
            ))
        ))]
        RafxCommandBuffer::Empty(_) => unimplemented!(),
    }
}

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

#[cfg(feature = "rafx-gles2")]
fn generate_mipmaps_gles2(
    command_buffer: &RafxCommandBufferGles2,
    texture: &RafxTexture,
) -> RafxResult<()> {
    use crate::gles2::NONE_TEXTURE;
    //TODO: Implement mipmaps for GL

    let texture = texture.gles2_texture().unwrap();
    let target = texture.gl_target();
    let texture_id = texture.gl_raw_image().gl_texture_id().unwrap();
    let gl_context = command_buffer.queue().device_context().gl_context();
    gl_context.gl_bind_texture(target, texture_id)?;
    gl_context.gl_generate_mipmap(target)?;
    gl_context.gl_bind_texture(target, NONE_TEXTURE)?;
    Ok(())
}

#[cfg(feature = "rafx-gles3")]
fn generate_mipmaps_gles3(
    command_buffer: &RafxCommandBufferGles3,
    texture: &RafxTexture,
) -> RafxResult<()> {
    use crate::gles3::NONE_TEXTURE;
    //TODO: Implement mipmaps for GL

    let texture = texture.gles3_texture().unwrap();
    let target = texture.gl_target();
    let texture_id = texture.gl_raw_image().gl_texture_id().unwrap();
    let gl_context = command_buffer.queue().device_context().gl_context();
    gl_context.gl_bind_texture(target, texture_id)?;
    gl_context.gl_generate_mipmap(target)?;
    gl_context.gl_bind_texture(target, NONE_TEXTURE)?;
    Ok(())
}

#[cfg(feature = "rafx-dx12")]
fn generate_mipmaps_dx12(
    command_buffer: &RafxCommandBufferDx12,
    texture: &RafxTexture,
) -> RafxResult<()> {
    use windows::Win32::Graphics::Direct3D12 as d3d12;

    // Don't generate mipmaps if only one mip level exists
    if texture.texture_def().mip_count <= 1 {
        return Ok(());
    }

    let device_context = command_buffer.queue().device_context();
    let mipmap_resources_ref = device_context.inner.mipmap_resources.borrow();
    let mipmap_resources = mipmap_resources_ref.as_ref().unwrap();

    //TODO: Set the command buffer to not have a root signature set
    let command_list = command_buffer.dx12_graphics_command_list();
    unsafe {
        command_list.SetComputeRootSignature(&mipmap_resources.root_signature);
        command_list.SetPipelineState(&mipmap_resources.pipeline);
    }

    // Must match compute shader
    #[derive(Debug)]
    struct Constants {
        // Texture level of source mip
        src_mip_level: u32,

        // upper 16-bits: if non-zero, apply SRGB curve
        // lower 16-bits: number of OutMips to write: [1, 4]
        packed_is_srgb_num_mip_levels: u32,

        // 1.0 / OutMip1.Dimensions
        texel_size_x: f32,
        texel_size_y: f32,
    }

    let srv_src_descriptor_id = texture.dx12_texture().unwrap().srv().unwrap();
    let srv_src_cpu_handle = device_context
        .inner
        .heaps
        .cbv_srv_uav_heap
        .id_to_cpu_handle(srv_src_descriptor_id);

    //TODO: LEAKING THESE
    println!("TODO: Generating mipmaps is leaking descriptors");
    let srv_dst_descriptor_id = device_context
        .inner
        .heaps
        .gpu_cbv_srv_uav_heap
        .allocate(device_context.d3d12_device(), 1)
        .unwrap();
    let srv_dst_cpu_handle = device_context
        .inner
        .heaps
        .gpu_cbv_srv_uav_heap
        .id_to_cpu_handle(srv_dst_descriptor_id);
    unsafe {
        device_context.d3d12_device().CopyDescriptorsSimple(
            1,
            srv_dst_cpu_handle,
            srv_src_cpu_handle,
            d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
        );
    }

    unsafe {
        command_list.SetComputeRootDescriptorTable(
            1,
            device_context
                .inner
                .heaps
                .gpu_cbv_srv_uav_heap
                .id_to_gpu_handle(srv_dst_descriptor_id),
        );
    }

    let mut src_mip_level = 0;

    // Continue until there are no unfilled mips, up to 4 at a time
    while src_mip_level + 1 < texture.texture_def().mip_count {
        let first_dst_level = src_mip_level + 1;
        let num_levels = (texture.texture_def().mip_count - first_dst_level).min(4);
        assert!(num_levels > 1 && num_levels <= 4);

        println!(
            "MIPMAP from src {} -> {} mips of {}",
            src_mip_level,
            num_levels,
            texture.texture_def().mip_count
        );

        // set high bit on upper 16-bits to turn on srgb conversion
        let mut packed_is_srgb_num_mip_levels = num_levels;
        if texture.texture_def().format.is_srgb() {
            packed_is_srgb_num_mip_levels |= 0x00010000;
        }

        let constants = Constants {
            src_mip_level,
            packed_is_srgb_num_mip_levels,
            texel_size_x: 1.0 / (texture.texture_def().extents.width >> first_dst_level) as f32,
            texel_size_y: 1.0 / (texture.texture_def().extents.height >> first_dst_level) as f32,
        };
        println!("MIPMAP constants {:?}", constants);

        unsafe {
            command_list.SetComputeRoot32BitConstants(
                0,
                4,
                &constants as *const Constants as *const std::ffi::c_void,
                0,
            );
        }

        let uav_src_descriptor_id = texture
            .dx12_texture()
            .unwrap()
            .uav(first_dst_level)
            .unwrap();
        let uav_src_cpu_handle = device_context
            .inner
            .heaps
            .cbv_srv_uav_heap
            .id_to_cpu_handle(uav_src_descriptor_id);

        //TODO: LEAKING THESE
        println!("TODO: Generating mipmaps is leaking descriptors");
        let uav_dst_descriptor_id = device_context
            .inner
            .heaps
            .gpu_cbv_srv_uav_heap
            .allocate(device_context.d3d12_device(), 4)
            .unwrap();
        let uav_dst_cpu_handle = device_context
            .inner
            .heaps
            .gpu_cbv_srv_uav_heap
            .id_to_cpu_handle(uav_dst_descriptor_id);
        unsafe {
            //TODO: This assumes the UAVs are consecutive which is not how RTVs work with texture arrays

            println!("MIPMAP set UAVs {}", num_levels);
            device_context.d3d12_device().CopyDescriptorsSimple(
                num_levels,
                uav_dst_cpu_handle,
                uav_src_cpu_handle,
                d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            );

            // Assign the unset descriptor to the first dest mip
            for i in num_levels..4 {
                println!("MIPMAP set unassigned UAV index {}", i);
                let uav_dst_cpu_handle = device_context
                    .inner
                    .heaps
                    .gpu_cbv_srv_uav_heap
                    .id_to_cpu_handle(uav_dst_descriptor_id.add_offset(i));
                device_context.d3d12_device().CopyDescriptorsSimple(
                    1,
                    uav_dst_cpu_handle,
                    uav_src_cpu_handle,
                    d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                );
            }

            command_list.SetComputeRootDescriptorTable(
                2,
                device_context
                    .inner
                    .heaps
                    .gpu_cbv_srv_uav_heap
                    .id_to_gpu_handle(uav_dst_descriptor_id),
            );
        }

        // round up to 8 for group size
        let num_threads_x = rafx_base::memory::round_size_up_to_alignment_u32(
            texture.texture_def().extents.width >> src_mip_level,
            8,
        );
        let num_threads_y = rafx_base::memory::round_size_up_to_alignment_u32(
            texture.texture_def().extents.height >> src_mip_level,
            8,
        );
        unsafe {
            command_list.Dispatch(num_threads_x, num_threads_y, 1);

            //
            // Barrier UAV access so we can potentially do next loop
            // TODO: Determine if there is another loop to do
            //
            let mut resource_barrier = d3d12::D3D12_RESOURCE_BARRIER::default();
            resource_barrier.Type = d3d12::D3D12_RESOURCE_BARRIER_TYPE_UAV;
            command_list.ResourceBarrier(&[resource_barrier]);
        }

        // We can skip 4 at a time
        src_mip_level += 4;
    }

    Ok(())
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
