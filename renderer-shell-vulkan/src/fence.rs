use ash::vk;
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;

use std::task::{Context, Poll, Waker};
use std::pin::Pin;
use std::future::Future;
use std::time::{Instant, Duration};
use std::sync::{
    atomic::{Ordering, AtomicUsize},
    Arc, Weak,
};
use crossbeam_channel::{Sender, Receiver, TryRecvError};
use super::VkDeviceContext;

struct VkFenceWaitRequest {
    state: Arc<VkFenceClientState>,
    waker: Waker,
    prev_signal_version: usize,
}
struct VkFenceClientState {
    signal_version: AtomicUsize,
    fence: vk::Fence,
}
pub struct VkFenceContext {
    device_context: VkDeviceContext,
    pending_fences: Vec<vk::Fence>,
    pending_requests: Vec<VkFenceWaitRequest>,
    fence_states: Vec<Arc<VkFenceClientState>>,
    receiver: Receiver<VkFenceWaitRequest>,
    sender: Sender<VkFenceWaitRequest>,
}
impl Drop for VkFenceContext {
    fn drop(&mut self) {
        for request in self.pending_requests.drain(0..self.pending_requests.len()) {
            request.waker.wake()
        }
    }
}
impl VkFenceContext {
    fn cloned_sender(&self) -> Sender<VkFenceWaitRequest> {
        self.sender.clone()
    }

    pub fn create_fence(&mut self) -> VkResult<VkFenceHandle> {
        let fence = unsafe {
            self.device_context
                .device()
                .create_fence(&vk::FenceCreateInfo::default(), None)?
        };
        let client_state = Arc::new(VkFenceClientState {
            signal_version: AtomicUsize::new(0),
            fence,
        });
        let state_handle = Arc::downgrade(&client_state);
        self.fence_states.push(client_state);
        Ok(VkFenceHandle {
            state: state_handle,
            prev_version: 0,
            signalling_sender: self.cloned_sender(),
        })
    }

    pub fn poll_fences(
        &mut self,
        timeout: Duration,
        channel_check_period: Duration,
    ) -> VkResult<()> {
        let mut time_remaining = timeout;
        loop {
            'channel_check: loop {
                match self.receiver.try_recv() {
                    Ok(request) => {
                        if request.state.signal_version.load(Ordering::Relaxed)
                            > request.prev_signal_version
                        {
                            request.waker.wake();
                        } else {
                            self.pending_fences.push(request.state.fence);
                            self.pending_requests.push(request);
                        }
                    }
                    Err(TryRecvError::Empty) => {
                        break 'channel_check;
                    }
                    Err(TryRecvError::Disconnected) => return Ok(()), // should we return some other error here?
                }
            }

            let device = self.device_context.device();
            unsafe {
                match device.wait_for_fences(
                    &self.pending_fences,
                    false,
                    channel_check_period.as_nanos() as u64,
                ) {
                    Ok(()) => {
                        // something is signalled
                        for i in (0..self.pending_fences.len()).rev() {
                            if unsafe { device.get_fence_status(self.pending_fences[i]) }? {
                                // the vulkan fence is immediately reset, but the user signal must be reset manually
                                device.reset_fences(&self.pending_fences[i..=i]);
                                self.pending_fences.swap_remove(i);
                                let request = self.pending_requests.swap_remove(i);
                                request.state.signal_version.fetch_add(1, Ordering::Relaxed);
                                request.waker.wake();
                            // check the signalled state - maybe the fence is already user-signalled
                            } else if self.pending_requests[i]
                                .state
                                .signal_version
                                .load(Ordering::Relaxed)
                                > self.pending_requests[i].prev_signal_version
                            {
                                self.pending_fences.swap_remove(i);
                                let request = self.pending_requests.swap_remove(i);
                                request.waker.wake();
                            }
                        }
                    }
                    Err(vk::Result::TIMEOUT) => {}
                    err => err?,
                }
            }
            if let Some(new_duration) = time_remaining.checked_sub(channel_check_period) {
                time_remaining = new_duration;
            } else {
                return Ok(());
            }
        }
    }
}

pub struct VkFenceHandle {
    prev_version: usize,
    state: Weak<VkFenceClientState>,
    signalling_sender: Sender<VkFenceWaitRequest>,
}
impl VkFenceHandle {
    pub fn reset(&mut self) -> VkResult<()> {
        if let Some(state) = self.state.upgrade() {
            self.prev_version = state.signal_version.load(Ordering::Relaxed);
            Ok(())
        } else {
            Err(vk::Result::ERROR_DEVICE_LOST)
        }
    }
}

impl Future for VkFenceHandle {
    type Output = VkResult<()>;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(state) = self.state.upgrade() {
            if state.signal_version.load(Ordering::Relaxed) > self.prev_version {
                Poll::Ready(Ok(()))
            } else {
                match self.signalling_sender.send(VkFenceWaitRequest {
                    waker: cx.waker().clone(),
                    state,
                    prev_signal_version: self.prev_version,
                }) {
                    Err(_) => Poll::Ready(Err(vk::Result::ERROR_DEVICE_LOST)),
                    Ok(()) => Poll::Pending,
                }
            }
        } else {
            Poll::Ready(Err(vk::Result::ERROR_DEVICE_LOST))
        }
    }
}

async fn test_await(fence: &mut VkFenceHandle) {
    let result = fence.await;
}
