use rafx_api::extra::upload::{RafxTransferUpload, RafxUploadError};
use rafx_api::{
    RafxBarrierQueueTransition, RafxBuffer, RafxBufferBarrier, RafxBufferDef,
    RafxCmdCopyBufferToBufferParams, RafxDeviceContext, RafxMemoryUsage, RafxResourceState,
    RafxResourceType,
};

pub fn enqueue_load_buffer(
    device_context: &RafxDeviceContext,
    upload: &mut RafxTransferUpload,
    // transfer_queue_family_index: u32,
    // dst_queue_family_index: u32,
    resource_type: RafxResourceType,
    data: &[u8],
) -> Result<RafxBuffer, RafxUploadError> {
    // Arbitrary, not sure if there is any requirement
    const REQUIRED_ALIGNMENT: usize = 16;

    // Push data into the staging buffer
    let offset = upload.push(data, REQUIRED_ALIGNMENT)?;
    let size = data.len() as u64;

    // Allocate a GPU buffer
    let dst_buffer = device_context.create_buffer(&RafxBufferDef {
        size,
        memory_usage: RafxMemoryUsage::GpuOnly,
        queue_type: upload.dst_queue().queue_type(),
        resource_type,
        ..Default::default()
    })?;

    upload.transfer_command_buffer().cmd_copy_buffer_to_buffer(
        &upload.staging_buffer(),
        &dst_buffer,
        &RafxCmdCopyBufferToBufferParams {
            src_byte_offset: offset,
            dst_byte_offset: 0,
            size,
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
        }],
        &[],
    )?;

    log::debug!("upload buffer bytes: {}", size);

    Ok(dst_buffer)
}
