use crate::dx12::{
    RafxCommandBufferDx12, RafxCommandPoolDx12, RafxDeviceContextDx12, RafxFenceDx12,
    RafxSemaphoreDx12, RafxSwapchainDx12,
};
use crate::{RafxCommandPoolDef, RafxError, RafxPresentSuccessResult, RafxQueueType, RafxResult};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use super::d3d12;

static NEXT_QUEUE_ID: AtomicU32 = AtomicU32::new(0);

pub struct RafxQueueDx12Inner {
    device_context: RafxDeviceContextDx12,
    queue_type: RafxQueueType,
    queue: d3d12::ID3D12CommandQueue,
    queue_id: u32,
    queue_fence: RafxFenceDx12,
}

impl std::fmt::Debug for RafxQueueDx12Inner {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("RafxQueueDx12Inner")
            .field("device_context", &self.device_context)
            .field("queue_type", &self.queue_type)
            .field("queue_id", &self.queue_id)
            .field("queue_fence", &self.queue_fence)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct RafxQueueDx12 {
    inner: Arc<RafxQueueDx12Inner>,
}

impl RafxQueueDx12 {
    pub fn queue_id(&self) -> u32 {
        self.inner.queue_id
    }

    pub fn dx12_queue(&self) -> &d3d12::ID3D12CommandQueue {
        &self.inner.queue
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.inner.queue_type
    }

    pub fn device_context(&self) -> &RafxDeviceContextDx12 {
        &self.inner.device_context
    }

    pub fn create_command_pool(
        &self,
        command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolDx12> {
        RafxCommandPoolDx12::new(&self, command_pool_def)
    }

    pub fn new(
        device_context: &RafxDeviceContextDx12,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueDx12> {
        //TODO: Allow setting priority

        let d3d12_queue_type = match queue_type {
            RafxQueueType::Graphics => d3d12::D3D12_COMMAND_LIST_TYPE_DIRECT,
            RafxQueueType::Compute => d3d12::D3D12_COMMAND_LIST_TYPE_COMPUTE,
            RafxQueueType::Transfer => d3d12::D3D12_COMMAND_LIST_TYPE_COPY,
        };

        let queue_desc = super::d3d12::D3D12_COMMAND_QUEUE_DESC {
            Type: d3d12_queue_type,
            Priority: d3d12::D3D12_COMMAND_QUEUE_PRIORITY_NORMAL.0,
            Flags: d3d12::D3D12_COMMAND_QUEUE_FLAG_NONE, //TODO: d3d12::D3D12_COMMAND_QUEUE_FLAG_DISABLE_GPU_TIMEOUT,
            NodeMask: 0,
        };

        let queue = unsafe {
            let queue: d3d12::ID3D12CommandQueue = device_context
                .d3d12_device()
                .CreateCommandQueue(&queue_desc)?;

            queue.SetName(match queue_type {
                RafxQueueType::Graphics => windows::core::w!("Graphics"),
                RafxQueueType::Compute => windows::core::w!("Compute"),
                RafxQueueType::Transfer => windows::core::w!("Copy"),
            })?;

            queue
        };

        let queue_fence = RafxFenceDx12::new(device_context)?;

        let queue_id = NEXT_QUEUE_ID.fetch_add(1, Ordering::Relaxed);
        let inner = RafxQueueDx12Inner {
            device_context: device_context.clone(),
            queue_type,
            queue,
            queue_id,
            //barrier_flags: AtomicU8::default(),
            queue_fence,
        };

        Ok(RafxQueueDx12 {
            inner: Arc::new(inner),
        })
    }

    pub fn wait_for_queue_idle(&self) -> RafxResult<()> {
        self.inner.queue_fence.queue_signal(self)?;
        self.inner.queue_fence.wait()
    }

    pub fn submit(
        &self,
        command_buffers: &[&RafxCommandBufferDx12],
        wait_semaphores: &[&RafxSemaphoreDx12],
        signal_semaphores: &[&RafxSemaphoreDx12],
        signal_fence: Option<&RafxFenceDx12>,
    ) -> RafxResult<()> {
        //println!("SUBMIT");
        //assert!(!command_buffers.is_empty());

        for wait_semaphore in wait_semaphores {
            wait_semaphore.fence().queue_wait(self)?;
        }

        //TODO: Get command lists out of command buffers
        //TODO: Don't allocate a vec, ideally use stack memory
        unsafe {
            //let command_lists: Vec<Option<d3d12::ID3D12CommandList>> = command_buffers.iter().map(|x| Some(x.dx12_command_list())).collect();
            let command_lists: Vec<d3d12::ID3D12CommandList> = command_buffers
                .iter()
                .map(|x| x.dx12_command_list())
                .collect();
            self.inner.queue.ExecuteCommandLists(&command_lists);
        }

        for signal_semaphore in signal_semaphores {
            signal_semaphore.fence().queue_signal(self)?;
        }

        if let Some(signal_fence) = signal_fence {
            signal_fence.queue_signal(self)?;
        }

        Ok(())
    }

    pub fn present(
        &self,
        swapchain: &RafxSwapchainDx12,
        wait_semaphores: &[&RafxSemaphoreDx12],
        _image_index: u32,
    ) -> RafxResult<RafxPresentSuccessResult> {
        for wait_semaphore in wait_semaphores {
            wait_semaphore.fence().queue_wait(self)?;
        }

        // 1 = vsync, 0 + DXGI_PRESENT_ALLOW_TEARING = not
        unsafe {
            let result = swapchain
                .dx12_swapchain()
                .Present(1, swapchain.swapchain_flags().0 as u32);
            if result.is_err() {
                Err(RafxError::HResult(result))
            } else {
                Ok(RafxPresentSuccessResult::Success)
            }
        }

        //TODO: Handle HRESULT better, such as nvidia aftermath/DRED
    }
}
