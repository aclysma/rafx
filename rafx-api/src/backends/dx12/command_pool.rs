use crate::dx12::{RafxCommandBufferDx12, RafxDeviceContextDx12, RafxQueueDx12};
use crate::{RafxCommandBufferDef, RafxCommandPoolDef, RafxQueueType, RafxResult};

pub struct RafxCommandPoolDx12 {
    command_allocator: super::d3d12::ID3D12CommandAllocator,
    command_list_type: super::d3d12::D3D12_COMMAND_LIST_TYPE,
    queue: RafxQueueDx12,
}

impl RafxCommandPoolDx12 {
    pub fn device_context(&self) -> &RafxDeviceContextDx12 {
        self.queue.device_context()
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.queue.queue_type()
    }

    pub fn queue(&self) -> &RafxQueueDx12 {
        &self.queue
    }

    pub fn command_list_type(&self) -> super::d3d12::D3D12_COMMAND_LIST_TYPE {
        self.command_list_type
    }

    pub fn command_allocator(&self) -> &super::d3d12::ID3D12CommandAllocator {
        &self.command_allocator
    }

    pub fn create_command_buffer(
        &self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferDx12> {
        RafxCommandBufferDx12::new(self, command_buffer_def)
    }

    pub fn reset_command_pool(&self) -> RafxResult<()> {
        unsafe {
            //WARNING: Crashing here may indicate a command buffer was left unclosed
            self.command_allocator.Reset()?
        }
        Ok(())
    }

    pub fn new(
        queue: &RafxQueueDx12,
        _command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolDx12> {
        let command_list_type =
            super::internal::queue_type_to_command_list_type(queue.queue_type());
        let command_allocator = unsafe {
            queue
                .device_context()
                .d3d12_device()
                .CreateCommandAllocator(command_list_type)
        }?;

        Ok(RafxCommandPoolDx12 {
            command_allocator,
            command_list_type,
            queue: queue.clone(),
        })
    }
}
