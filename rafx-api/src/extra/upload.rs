use crate::{
    RafxBuffer, RafxBufferDef, RafxCommandBuffer, RafxCommandBufferDef, RafxCommandPool,
    RafxCommandPoolDef, RafxDeviceContext, RafxError, RafxFence, RafxFenceStatus, RafxMemoryUsage,
    RafxQueue, RafxQueueType, RafxResourceType, RafxResult,
};
use crossbeam_channel::{Receiver, Sender};
use std::ops::{Deref, DerefMut};

// Based on UploadHeap in cauldron
// (https://github.com/GPUOpen-LibrariesAndSDKs/Cauldron/blob/5acc12602c55e469cc1f9181967dbcb122f8e6c7/src/VK/base/UploadHeap.h)

pub struct RafxUploadBuffer {
    buffer: Option<RafxBuffer>,
    upload_buffer_released_tx: Sender<RafxBuffer>,
}

impl Drop for RafxUploadBuffer {
    fn drop(&mut self) {
        // self.buffer is only allowed to be None after dropping
        let buffer = self.buffer.take().unwrap();
        // we assume RafxUploadBufferPool lifetime is greater than RafxUploadBuffer
        self.upload_buffer_released_tx.send(buffer).unwrap();
    }
}

impl RafxUploadBuffer {
    fn buffer(&self) -> &RafxBuffer {
        self.buffer.as_ref().unwrap()
    }

    fn buffer_mut(&mut self) -> &mut RafxBuffer {
        self.buffer.as_mut().unwrap()
    }
}

pub struct RafxUploadBufferPool {
    _device_context: RafxDeviceContext,
    buffer_count: u32,
    buffer_size: u64,
    unused_buffers: Vec<RafxBuffer>,
    upload_buffer_released_tx: Sender<RafxBuffer>,
    upload_buffer_released_rx: Receiver<RafxBuffer>,
}

impl Drop for RafxUploadBufferPool {
    fn drop(&mut self) {
        self.handle_dropped_buffers();
        // If this trips a buffer was in use when this pool was dropped
        assert_eq!(self.unused_buffers.len(), self.buffer_count as usize);
    }
}

impl RafxUploadBufferPool {
    pub fn new(
        device_context: &RafxDeviceContext,
        buffer_count: u32,
        buffer_size: u64,
    ) -> RafxResult<Self> {
        let (upload_buffer_released_tx, upload_buffer_released_rx) = crossbeam_channel::unbounded();
        let mut unused_buffers = Vec::with_capacity(buffer_count as usize);

        for _ in 0..buffer_count {
            let buffer = device_context.create_buffer(&RafxBufferDef {
                size: buffer_size,
                memory_usage: RafxMemoryUsage::CpuToGpu,
                queue_type: RafxQueueType::Transfer,
                resource_type: RafxResourceType::BUFFER,
                ..Default::default()
            })?;
            unused_buffers.push(buffer);
        }

        Ok(RafxUploadBufferPool {
            _device_context: device_context.clone(),
            buffer_count,
            buffer_size,
            unused_buffers,
            upload_buffer_released_tx,
            upload_buffer_released_rx,
        })
    }

    fn take(
        &mut self,
        required_size_bytes: u64,
    ) -> RafxResult<RafxUploadBuffer> {
        if self.buffer_size < required_size_bytes {
            return Err(format!(
                "Buffer of size {} requested but the pool's buffers are only {} in size",
                required_size_bytes, self.buffer_size
            ))?;
        }

        // Move any release buffers back into the unused_buffers list
        self.handle_dropped_buffers();

        // Take a buffer, if one is available, return it. Otherwise return an error.
        if let Some(buffer) = self.unused_buffers.pop() {
            Ok(RafxUploadBuffer {
                buffer: Some(buffer),
                upload_buffer_released_tx: self.upload_buffer_released_tx.clone(),
            })
        } else {
            Err("RafxUploadBufferPool has no more available buffers")?
        }
    }

    // Move any release buffers back into the unused_buffers list
    fn handle_dropped_buffers(&mut self) {
        for buffer in self.upload_buffer_released_rx.try_iter() {
            self.unused_buffers.push(buffer);
        }
    }
}

#[derive(Debug)]
pub enum RafxUploadError {
    BufferFull,
    Other(RafxError),
}

impl RafxUploadError {
    // Helpful for when types are not being inferred as expected
    pub fn into_rafx_error(self) -> RafxError {
        self.into()
    }
}

impl core::fmt::Display for RafxUploadError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            RafxUploadError::BufferFull => write!(fmt, "UploadBufferFull"),
            RafxUploadError::Other(ref e) => e.fmt(fmt),
        }
    }
}

impl From<RafxError> for RafxUploadError {
    fn from(error: RafxError) -> Self {
        RafxUploadError::Other(error)
    }
}

impl Into<RafxError> for RafxUploadError {
    fn into(self) -> RafxError {
        match self {
            RafxUploadError::BufferFull => {
                RafxError::StringError("Upload buffer is full".to_string())
            }
            RafxUploadError::Other(e) => e,
        }
    }
}

impl From<&str> for RafxUploadError {
    fn from(str: &str) -> Self {
        RafxError::StringError(str.to_string()).into()
    }
}

impl From<String> for RafxUploadError {
    fn from(string: String) -> Self {
        RafxError::StringError(string).into()
    }
}

#[derive(PartialEq)]
pub enum RafxUploadState {
    /// The upload is not submitted yet and data may be appended to it
    Writable,

    /// The buffer has been sent to the GPU and is no longer writable
    SentToGpu,

    /// The upload is finished and the resources may be used
    Complete,
}

enum UploadBuffer {
    Pooled(RafxUploadBuffer),
    NonPooled(RafxBuffer),
}

impl Deref for UploadBuffer {
    type Target = RafxBuffer;

    fn deref(&self) -> &Self::Target {
        self.buffer()
    }
}

impl DerefMut for UploadBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer_mut()
    }
}

impl UploadBuffer {
    fn buffer(&self) -> &RafxBuffer {
        match self {
            UploadBuffer::Pooled(buffer) => buffer.buffer(),
            UploadBuffer::NonPooled(buffer) => buffer,
        }
    }

    fn buffer_mut(&mut self) -> &mut RafxBuffer {
        match self {
            UploadBuffer::Pooled(buffer) => buffer.buffer_mut(),
            UploadBuffer::NonPooled(buffer) => buffer,
        }
    }
}

/// Convenience struct that allows accumulating writes into a staging buffer and commands
/// to execute on the staging buffer. This allows for batching uploading resources.
pub struct RafxUpload {
    queue: RafxQueue,
    command_pool: RafxCommandPool,
    command_buffer: RafxCommandBuffer,

    buffer: UploadBuffer,

    writable: bool,
    fence: RafxFence,

    buffer_begin: *mut u8,
    buffer_end: *mut u8,
    buffer_write_pointer: *mut u8,
}

unsafe impl Send for RafxUpload {}
unsafe impl Sync for RafxUpload {}

impl RafxUpload {
    pub fn new(
        device_context: &RafxDeviceContext,
        queue: &RafxQueue,
        buffer_size: u64,
        buffer_pool: Option<&mut RafxUploadBufferPool>,
    ) -> RafxResult<Self> {
        //
        // Command Buffers
        //
        let mut command_pool =
            queue.create_command_pool(&RafxCommandPoolDef { transient: true })?;
        let command_buffer = command_pool.create_command_buffer(&RafxCommandBufferDef {
            is_secondary: false,
        })?;
        command_buffer.begin()?;

        let buffer = if let Some(buffer_pool) = buffer_pool {
            UploadBuffer::Pooled(buffer_pool.take(buffer_size)?)
        } else {
            UploadBuffer::NonPooled(device_context.create_buffer(&RafxBufferDef {
                size: buffer_size,
                memory_usage: RafxMemoryUsage::CpuToGpu,
                queue_type: RafxQueueType::Transfer,
                resource_type: RafxResourceType::BUFFER,
                ..Default::default()
            })?)
        };

        let (buffer_begin, buffer_end, buffer_write_pointer) = unsafe {
            let buffer_begin = buffer.map_buffer()?;

            let buffer_end = buffer_begin.add(buffer_size as usize);
            let buffer_write_pointer = buffer_begin;

            (buffer_begin, buffer_end, buffer_write_pointer)
        };

        let fence = device_context.create_fence()?;

        let upload = RafxUpload {
            queue: queue.clone(),
            command_pool,
            command_buffer,
            buffer,
            fence,
            writable: true,
            buffer_begin,
            buffer_end,
            buffer_write_pointer,
        };

        Ok(upload)
    }

    pub fn has_space_available(
        &self,
        bytes_to_write: usize,
        required_alignment: usize,
        number_of_writes: usize,
    ) -> bool {
        let mut write_end_ptr = self.buffer_write_pointer as usize;

        for _ in 0..number_of_writes {
            // Align the current write pointer
            let write_begin_ptr = ((write_end_ptr + required_alignment - 1) / required_alignment)
                * required_alignment;
            write_end_ptr = write_begin_ptr + bytes_to_write;
        }

        // See if we would walk past the end of the buffer
        write_end_ptr <= self.buffer_end as usize
    }

    pub fn push(
        &mut self,
        data: &[u8],
        required_alignment: usize,
    ) -> Result<u64, RafxUploadError> {
        log::trace!("Pushing {} bytes into upload", data.len());

        if self.writable {
            unsafe {
                // Figure out the span of memory we will write over
                let write_begin_ptr = (((self.buffer_write_pointer as usize + required_alignment
                    - 1)
                    / required_alignment)
                    * required_alignment) as *mut u8; // as const *u8;
                let write_end_ptr = write_begin_ptr.add(data.len());

                // If the span walks past the end of the buffer, fail
                if write_end_ptr > self.buffer_end {
                    Err(RafxUploadError::BufferFull)?;
                }

                std::ptr::copy_nonoverlapping(data.as_ptr(), write_begin_ptr, data.len());
                self.buffer_write_pointer = write_end_ptr;

                Ok(write_begin_ptr as u64 - self.buffer_begin as u64)
            }
        } else {
            Err("Upload buffer is not writable")?
        }
    }

    pub fn buffer_size(&self) -> u64 {
        self.buffer_end as u64 - self.buffer_begin as u64
    }

    pub fn bytes_written(&self) -> u64 {
        self.buffer_write_pointer as u64 - self.buffer_begin as u64
    }

    pub fn bytes_free(&self) -> u64 {
        self.buffer_end as u64 - self.buffer_write_pointer as u64
    }

    pub fn command_pool(&self) -> &RafxCommandPool {
        &self.command_pool
    }

    pub fn command_buffer(&self) -> &RafxCommandBuffer {
        &self.command_buffer
    }

    pub fn staging_buffer(&self) -> &RafxBuffer {
        &self.buffer
    }

    pub fn queue(&self) -> &RafxQueue {
        &self.queue
    }

    pub fn submit(&mut self) -> RafxResult<()> {
        if self.writable {
            self.command_buffer.end()?;
            self.queue
                .submit(&[&self.command_buffer], &[], &[], Some(&self.fence))?;
            self.writable = false;
        }

        Ok(())
    }

    pub fn state(&self) -> RafxResult<RafxUploadState> {
        let state = if self.writable {
            RafxUploadState::Writable
        } else {
            if self.fence.get_fence_status()? != RafxFenceStatus::Incomplete {
                RafxUploadState::Complete
            } else {
                RafxUploadState::SentToGpu
            }
        };

        Ok(state)
    }

    fn wait_for_idle(&self) -> RafxResult<()> {
        self.fence.wait()
    }
}

impl Drop for RafxUpload {
    fn drop(&mut self) {
        log::trace!("destroying RafxUpload");

        // If the transfer is in flight, wait for it to complete
        self.wait_for_idle().unwrap();

        self.buffer.unmap_buffer().unwrap();

        // buffer, command pool, and fence are destroyed by dropping them

        log::trace!("destroyed RafxUpload");
    }
}

#[derive(PartialEq)]
pub enum RafxTransferUploadState {
    /// The upload is not submitted yet and data may be appended to it
    Writable,

    /// The buffer has been sent to the GPU's transfer queue and is no longer writable
    SentToTransferQueue,

    /// The submit to the transfer queue finished. We are ready to submit to the graphics queue
    /// but we wait here until called explicitly because submitting to a queue is not thread-safe.
    /// Additionally, it's likely we will want to batch this submit with other command buffers going
    /// to the same queue
    PendingSubmitDstQueue,

    /// The buffer has been sent to the GPU's graphics queue but has not finished
    SentToDstQueue,

    /// The submit has finished on both queues and the uploaded resources are ready for use
    Complete,
}

/// A state machine and associated buffers/synchronization primitives to simplify uploading resources
/// to the GPU via a transfer queue, and then submitting a memory barrier to the graphics queue
pub struct RafxTransferUpload {
    upload: RafxUpload,

    dst_queue: RafxQueue,
    dst_command_pool: RafxCommandPool,
    dst_command_buffer: RafxCommandBuffer,

    dst_fence: RafxFence,
    sent_to_dst_queue: bool,
}

impl RafxTransferUpload {
    pub fn new(
        device_context: &RafxDeviceContext,
        transfer_queue: &RafxQueue,
        dst_queue: &RafxQueue,
        size: u64,
        buffer_pool: Option<&mut RafxUploadBufferPool>,
    ) -> RafxResult<Self> {
        //
        // Command Buffers
        //
        let mut dst_command_pool =
            dst_queue.create_command_pool(&RafxCommandPoolDef { transient: true })?;
        let dst_command_buffer = dst_command_pool.create_command_buffer(&RafxCommandBufferDef {
            is_secondary: false,
        })?;

        dst_command_buffer.begin()?;

        let upload = RafxUpload::new(device_context, transfer_queue, size, buffer_pool)?;

        let dst_fence = device_context.create_fence()?;

        Ok(RafxTransferUpload {
            upload,
            dst_queue: dst_queue.clone(),
            dst_command_pool,
            dst_command_buffer,
            dst_fence,
            sent_to_dst_queue: false,
        })
    }

    pub fn has_space_available(
        &self,
        bytes_to_write: usize,
        required_alignment: usize,
        number_of_writes: usize,
    ) -> bool {
        self.upload
            .has_space_available(bytes_to_write, required_alignment, number_of_writes)
    }

    pub fn push(
        &mut self,
        data: &[u8],
        required_alignment: usize,
    ) -> Result<u64, RafxUploadError> {
        self.upload.push(data, required_alignment)
    }

    pub fn buffer_size(&self) -> u64 {
        self.upload.buffer_size()
    }

    pub fn bytes_written(&self) -> u64 {
        self.upload.bytes_written()
    }

    pub fn bytes_free(&self) -> u64 {
        self.upload.bytes_free()
    }

    pub fn staging_buffer(&self) -> &RafxBuffer {
        &self.upload.staging_buffer()
    }

    pub fn transfer_command_pool(&self) -> &RafxCommandPool {
        self.upload.command_pool()
    }

    pub fn transfer_command_buffer(&self) -> &RafxCommandBuffer {
        self.upload.command_buffer()
    }

    pub fn dst_command_pool(&self) -> &RafxCommandPool {
        &self.dst_command_pool
    }

    pub fn dst_command_buffer(&self) -> &RafxCommandBuffer {
        &self.dst_command_buffer
    }

    pub fn transfer_queue(&self) -> &RafxQueue {
        self.upload.queue()
    }

    pub fn dst_queue(&self) -> &RafxQueue {
        &self.dst_queue
    }

    pub fn submit_transfer(&mut self) -> RafxResult<()> {
        self.upload.submit()
    }

    pub fn submit_dst(&mut self) -> RafxResult<()> {
        if self.state()? == RafxTransferUploadState::PendingSubmitDstQueue {
            self.dst_command_buffer.end()?;
            self.dst_queue
                .submit(&[&self.dst_command_buffer], &[], &[], Some(&self.dst_fence))?;
            self.sent_to_dst_queue = true;
        }

        Ok(())
    }

    pub fn state(&self) -> RafxResult<RafxTransferUploadState> {
        let state = if self.sent_to_dst_queue {
            if self.dst_fence.get_fence_status()? != RafxFenceStatus::Incomplete {
                RafxTransferUploadState::Complete
            } else {
                RafxTransferUploadState::SentToDstQueue
            }
        } else {
            match self.upload.state()? {
                RafxUploadState::Writable => RafxTransferUploadState::Writable,
                RafxUploadState::SentToGpu => RafxTransferUploadState::SentToTransferQueue,
                RafxUploadState::Complete => RafxTransferUploadState::PendingSubmitDstQueue,
            }
        };

        Ok(state)
    }

    fn wait_for_idle(&self) -> RafxResult<()> {
        if self.sent_to_dst_queue {
            self.dst_fence.wait()
        } else {
            Ok(())
        }
    }

    pub fn block_until_upload_complete(&mut self) -> RafxResult<()> {
        log::trace!("wait on transfer queue {:?}", self.upload.queue);
        self.submit_transfer()?;
        loop {
            if self.state()? == RafxTransferUploadState::PendingSubmitDstQueue {
                break;
            }
        }

        log::trace!("blocking on dst queue {:?}", self.dst_queue);
        self.submit_dst()?;
        loop {
            if self.state()? == RafxTransferUploadState::Complete {
                break;
            }
        }

        Ok(())
    }
}

impl Drop for RafxTransferUpload {
    fn drop(&mut self) {
        log::trace!("destroying RafxUpload");

        // If the transfer is in flight, wait for it to complete
        self.upload.wait_for_idle().unwrap();
        self.wait_for_idle().unwrap();

        log::trace!("destroyed RafxUpload");
    }
}
