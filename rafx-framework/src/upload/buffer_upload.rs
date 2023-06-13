use rafx_api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx_api::{
    RafxBarrierQueueTransition, RafxBuffer, RafxBufferBarrier, RafxBufferDef,
    RafxCmdCopyBufferToBufferParams, RafxDeviceContext, RafxMemoryUsage, RafxOffsetSize,
    RafxResourceState, RafxResourceType,
};

pub fn enqueue_load_buffer(
    device_context: &RafxDeviceContext,
    upload: &mut RafxTransferUpload,
    // transfer_queue_family_index: u32,
    // dst_queue_family_index: u32,
    resource_type: RafxResourceType,
    data: &[u8],
    dst_buffer: Option<&RafxBuffer>,
    dst_byte_offset: u64,
    //TODO: params?
) -> Result<Option<RafxBuffer>, RafxUploadError> {
    // Arbitrary, not sure if there is any requirement
    const REQUIRED_ALIGNMENT: usize = 16;

    // Push data into the staging buffer
    let src_byte_offset = upload.push(data, REQUIRED_ALIGNMENT)?;
    let size = data.len() as u64;

    // Allocate a GPU buffer
    let mut new_buffer = None;
    let (dst_buffer, barrier_offset_size) = if let Some(dst_buffer) = dst_buffer {
        let barrier_offset_size = RafxOffsetSize {
            size: data.len() as u64,
            byte_offset: dst_byte_offset,
        };
        (dst_buffer, Some(barrier_offset_size))
    } else {
        new_buffer = Some(device_context.create_buffer(&RafxBufferDef {
            size,
            memory_usage: RafxMemoryUsage::GpuOnly,
            queue_type: upload.transfer_queue().queue_type(),
            resource_type,
            ..Default::default()
        })?);
        (new_buffer.as_ref().unwrap(), None)
    };

    //println!("upload to buffer {:?} offset: {}", dst_buffer, dst_byte_offset);
    upload.transfer_command_buffer().cmd_copy_buffer_to_buffer(
        &upload.staging_buffer(),
        &dst_buffer,
        &RafxCmdCopyBufferToBufferParams {
            src_byte_offset,
            dst_byte_offset,
            size: data.len() as u64,
        },
    )?;

    upload.transfer_command_buffer().cmd_resource_barrier(
        &[RafxBufferBarrier {
            buffer: &dst_buffer,
            src_state: RafxResourceState::COPY_DST,
            dst_state: RafxResourceState::VERTEX_AND_CONSTANT_BUFFER
                | RafxResourceState::INDEX_BUFFER,
            queue_transition: RafxBarrierQueueTransition::ReleaseTo(
                upload.dst_queue().queue_type(),
            ),
            offset_size: barrier_offset_size,
        }],
        &[],
    )?;

    upload.dst_command_buffer().cmd_resource_barrier(
        &[RafxBufferBarrier {
            buffer: &dst_buffer,
            src_state: RafxResourceState::COPY_DST,
            dst_state: RafxResourceState::VERTEX_AND_CONSTANT_BUFFER
                | RafxResourceState::INDEX_BUFFER,
            queue_transition: RafxBarrierQueueTransition::AcquireFrom(
                upload.transfer_queue().queue_type(),
            ),
            offset_size: barrier_offset_size,
        }],
        &[],
    )?;

    log::debug!("upload buffer bytes: {}", data.len());

    Ok(new_buffer)
}
