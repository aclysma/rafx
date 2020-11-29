use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk;
use crossbeam_channel::{Receiver, Sender};
use fnv::FnvHashMap;
use renderer_shell_vulkan::VkDeviceContext;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Info we hash across to identify equivalent command pools, allowing us to share them
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPoolMeta {
    queue_family_index: u32,
    command_pool_create_flags: vk::CommandPoolCreateFlags,
}

/// Represents a command pool with arbitrary lifetime. Destroys itself on cleanup. You must follow
/// standard vulkan rules:
///  - Don't write to command buffers from the same pool simultaneously on different threads
///  - Don't write to command buffers in this pool if any other command buffer that was allocated
///    from it is in-flight
///  - Don't allocate or reset the pool if other if any other command buffer that was allocated
///    from it is in-flight, or is being written
///
/// Normally, you would want one of these per thread, per frames in flight. For easier usage in
/// multi-threaded code for short-lived command buffers, use DynCommandWriterAllocator.
pub struct CommandPool {
    device_context: VkDeviceContext,
    command_pool: vk::CommandPool,
    command_pool_meta: CommandPoolMeta,
}

impl CommandPool {
    /// Creates the pool
    pub fn new(
        device_context: &VkDeviceContext,
        queue_family_index: u32,
        command_pool_create_flags: vk::CommandPoolCreateFlags,
    ) -> VkResult<CommandPool> {
        let command_pool_meta = CommandPoolMeta {
            command_pool_create_flags,
            queue_family_index,
        };

        log::trace!("Creating command pool {:?}", command_pool_meta);

        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(command_pool_create_flags)
            .queue_family_index(queue_family_index);

        let command_pool = unsafe {
            device_context
                .device()
                .create_command_pool(&pool_create_info, None)?
        };

        Ok(CommandPool {
            device_context: device_context.clone(),
            command_pool,
            command_pool_meta,
        })
    }

    /// Allocates command buffers from the pool
    pub fn create_command_buffers(
        &self,
        command_buffer_level: vk::CommandBufferLevel,
        count: u32,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        log::trace!(
            "Creating command buffers from pool {:?}",
            self.command_pool_meta
        );

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool)
            .level(command_buffer_level)
            .command_buffer_count(count);

        unsafe {
            self.device_context
                .device()
                .allocate_command_buffers(&command_buffer_allocate_info)
        }
    }

    /// Resets the command pool, invalidating all command buffers that have previously been created
    pub fn reset_command_pool(&self) -> VkResult<()> {
        log::trace!("Resetting command buffer pool {:?}", self.command_pool_meta);

        unsafe {
            self.device_context
                .device()
                .reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())
        }
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_command_pool(self.command_pool, None);
        }
    }
}

struct DynCommandWriterInner {
    pool: CommandPool,
    device_context: VkDeviceContext,
    submits_in_frame_index: u64,

    // Just a debugging aid
    writer_id: u64,
}

/// A helper that can be allocated as needed to create very short-lived command buffers. The object
/// may not be persisted across frames. Instead, allocated a new one every frame. They are pooled,
/// allocation is cheap and thread-safe.
pub struct DynCommandWriter {
    // This should never be None. We always allocate this to a non-none value and we don't clear
    // it until the drop handler
    inner: Option<DynCommandWriterInner>,
    drop_tx: Sender<DynCommandWriterInner>,
    currently_writing: Option<vk::CommandBuffer>,
}

impl DynCommandWriter {
    fn new(
        inner: DynCommandWriterInner,
        drop_tx: Sender<DynCommandWriterInner>,
    ) -> Self {
        log::trace!(
            "Creating DynCommandWriter({}) {:?}",
            inner.writer_id,
            inner.pool.command_pool_meta
        );

        DynCommandWriter {
            inner: Some(inner),
            drop_tx,
            currently_writing: None,
        }
    }

    /// Allocate a command buffer and call vkBeginCommandBuffer on it
    pub fn begin_command_buffer(
        &mut self,
        command_buffer_level: vk::CommandBufferLevel,
        command_buffer_usage_flags: vk::CommandBufferUsageFlags,
        inheritance_info: Option<&vk::CommandBufferInheritanceInfo>,
    ) -> VkResult<vk::CommandBuffer> {
        let inner = self.inner.as_ref().unwrap();
        log::trace!(
            "DynCommandWriter({}) begin_command_buffer Level: {:?} Usage Flags: {:?} Inheritance Info: {:?}",
            inner.writer_id,
            command_buffer_level,
            command_buffer_usage_flags,
            inheritance_info
        );

        assert!(self.currently_writing.is_none());
        let mut begin_info =
            vk::CommandBufferBeginInfo::builder().flags(command_buffer_usage_flags);

        if let Some(inheritance_info) = inheritance_info {
            begin_info = begin_info.inheritance_info(inheritance_info);
        }

        let command_buffer = inner.pool.create_command_buffers(command_buffer_level, 1)?[0];

        unsafe {
            inner
                .device_context
                .device()
                .begin_command_buffer(command_buffer, &*begin_info)?;
        }
        self.currently_writing = Some(command_buffer);
        Ok(command_buffer)
    }

    /// Finish writing the current command buffer by calling vkEndCommandBuffer on it.
    pub fn end_command_buffer(&mut self) -> VkResult<vk::CommandBuffer> {
        let inner = self.inner.as_ref().unwrap();
        log::trace!("DynCommandWriter({}) end_command_buffer", inner.writer_id,);

        // Only valid to call end_writing() if we are writing a command buffer
        let command_buffer = self.currently_writing.take().unwrap();

        unsafe {
            inner
                .device_context
                .device()
                .end_command_buffer(command_buffer)?;
        }
        Ok(command_buffer)
    }

    /// Get the underlying pool within the allocator. The pool will be destroyed after
    /// MAX_FRAMES_IN_FLIGHT pass, and all command buffers created with it must follow the same
    /// restrictions as a command buffer created via begin_command_buffer/end_command_buffer. It's
    /// recommended to use begin_writing/end_writing as it is less error prone.
    pub fn pool(&mut self) -> &mut CommandPool {
        &mut self.inner.as_mut().unwrap().pool
    }
}

impl Drop for DynCommandWriter {
    fn drop(&mut self) {
        assert!(self.currently_writing.is_none());
        let inner = self.inner.take().unwrap();
        self.drop_tx.send(inner).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingCommandPoolMeta {
    submits_in_frame_index: u64,
    command_pool_meta: CommandPoolMeta,
}

struct DynCommandWriterAllocatorInner {
    // Command pools that are ready to use but have no recorded commands
    unused_writers: FnvHashMap<CommandPoolMeta, Vec<DynCommandWriterInner>>,

    // Command pools that are in use and have a frame that we know they will be submitted in
    pending_writers: FnvHashMap<PendingCommandPoolMeta, Vec<DynCommandWriterInner>>,

    // submitted writers
    // TODO: Would be less allocations if this was a static array of vecs
    submitted_writers: BTreeMap<u64, Vec<DynCommandWriterInner>>,

    max_frames_in_flight: u64,
    current_frame_index: u64,

    device_context: VkDeviceContext,
    drop_tx: Sender<DynCommandWriterInner>,
    drop_rx: Receiver<DynCommandWriterInner>,

    // Just a debugging aid
    next_writer_id: u64,
}

/// An allocator for DynCommandWriters, objects that are short-lived and NOT persisted across
/// frames. Meant for allocating command buffers that are usually single use and only for the
/// current frame. The allocator is multi-thread friendly, but the writers themselves are not. So
/// if writing command buffers from multiple threads, allocate a writer per thread.
#[derive(Clone)]
pub struct DynCommandWriterAllocator {
    inner: Arc<Mutex<DynCommandWriterAllocatorInner>>,
}

impl DynCommandWriterAllocator {
    /// Create an allocator for DynCommandWriters.
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        let inner = DynCommandWriterAllocatorInner {
            max_frames_in_flight: max_frames_in_flight as u64,
            pending_writers: Default::default(),
            submitted_writers: Default::default(),
            unused_writers: Default::default(),
            current_frame_index: 0,
            device_context: device_context.clone(),
            drop_tx,
            drop_rx,
            next_writer_id: 0,
        };

        DynCommandWriterAllocator {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Allocates a writer. Writers wrap CommandPools. The parameters match inputs for
    /// CommandPool::new. `delay_submission_by_frame_count` indicates how many frames will pass
    /// before the commands will be submitted (which affects how long-lived they will be). DO NOT
    /// submit command buffers earlier than this as the writers are pooled.
    ///
    /// The common case for delay_submission_by_frame_count is to pass 0. You might pass 1 if for
    /// example, you are building a command buffer for frame N + 1 while frame N is not yet
    /// submitted.
    pub fn allocate_writer(
        &self,
        queue_family_index: u32,
        command_pool_create_flags: vk::CommandPoolCreateFlags,
        delay_submission_by_frame_count: u64,
    ) -> VkResult<DynCommandWriter> {
        let mut guard = self.inner.lock().unwrap();

        // Determine what frame this will be committed in
        let submits_in_frame_index = guard.current_frame_index + delay_submission_by_frame_count;

        // Build a key to search for an existing writer to reuse
        let meta = PendingCommandPoolMeta {
            submits_in_frame_index,
            command_pool_meta: CommandPoolMeta {
                queue_family_index,
                command_pool_create_flags,
            },
        };

        log::trace!("DynCommandWriterAllocator::allocate_writer {:?}", meta);

        Self::drain_drop_rx(&mut *guard);

        // Try to find something in the pending collection and reuse it
        if let Some(writers) = guard.pending_writers.get_mut(&meta) {
            if let Some(writer) = writers.pop() {
                log::trace!(
                    "DynCommandWriterAllocator::allocate_writer {:?} reusing pending writer DynCommandWriter({})",
                    meta,
                    writer.writer_id
                );
                assert_eq!(writer.submits_in_frame_index, submits_in_frame_index);
                return Ok(DynCommandWriter::new(writer, guard.drop_tx.clone()));
            }
        }

        // If we don't have a "dirty" writer for this frame yet, try to reuse an existing unused one
        if let Some(writers) = guard.unused_writers.get_mut(&meta.command_pool_meta) {
            if let Some(mut writer) = writers.pop() {
                log::trace!(
                    "DynCommandWriterAllocator::allocate_writer {:?} reusing unused writer DynCommandWriter({})",
                    meta,
                    writer.writer_id
                );
                writer.submits_in_frame_index = submits_in_frame_index;
                return Ok(DynCommandWriter::new(writer, guard.drop_tx.clone()));
            }
        }

        let writer_id = guard.next_writer_id;
        guard.next_writer_id += 1;

        log::trace!(
            "DynCommandWriterAllocator::allocate_writer {:?} creating new DynCommandWriter({})",
            meta,
            writer_id
        );

        // Did not find a suitable writer, create a new one
        let pool = CommandPool::new(
            &guard.device_context,
            queue_family_index,
            command_pool_create_flags,
        )?;

        let inner = DynCommandWriterInner {
            device_context: guard.device_context.clone(),
            pool,
            submits_in_frame_index,
            writer_id,
        };

        Ok(DynCommandWriter::new(inner, guard.drop_tx.clone()))
    }

    /// Call every frame to recycle command pools that are no-longer in flight
    #[profiling::function]
    pub fn on_frame_complete(&self) -> VkResult<()> {
        let mut guard = self.inner.lock().unwrap();
        log::trace!("DynCommandWriterAllocator::on_frame_complete: DynCommandWriterAllocator on_frame_complete finishing frame {}", guard.current_frame_index);

        {
            profiling::scope!("drain_drop_rx");
            Self::drain_drop_rx(&mut *guard);
        }

        // Find any pending writers that should submit during this frame
        let mut pending_writer_keys = Vec::default();
        for key in guard.pending_writers.keys() {
            if key.submits_in_frame_index == guard.current_frame_index {
                pending_writer_keys.push(key.clone());
            }
        }

        // Move them to the submitted writers collection
        for key in pending_writer_keys {
            let mut pending_writers = guard.pending_writers.remove(&key).unwrap();

            for pending_writer in &pending_writers {
                log::trace!(
                    "DynCommandWriterAllocator::on_frame_complete: DynCommandWriter({}) being moved to submitted writer list",
                    pending_writer.writer_id,
                );
            }

            guard
                .submitted_writers
                .entry(key.submits_in_frame_index)
                .or_default()
                .append(&mut pending_writers);
        }

        // Find all the submitted writers that are old enough to no longer be in flight
        let mut submitted_writer_keys = Vec::default();
        for &submits_in_frame_index in guard.submitted_writers.keys() {
            // We can use >= here because we're bumping current_frame_index at the end of this
            // function
            if guard.current_frame_index >= submits_in_frame_index + guard.max_frames_in_flight {
                submitted_writer_keys.push(submits_in_frame_index);
            } else {
                // The map is sorted by frame count
                break;
            }
        }

        // Move them to the unused collection
        for key in submitted_writer_keys {
            let submitted_writers = guard.submitted_writers.remove(&key).unwrap();
            for submitted_writer in submitted_writers {
                log::trace!(
                    "DynCommandWriterAllocator::on_frame_complete: DynCommandWriter({}) being moved to unused writer map",
                    submitted_writer.writer_id,
                );

                let meta = submitted_writer.pool.command_pool_meta.clone();
                {
                    profiling::scope!("reset_command_pool");
                    submitted_writer.pool.reset_command_pool()?;
                }

                guard
                    .unused_writers
                    .entry(meta)
                    .or_default()
                    .push(submitted_writer);
            }
        }

        log::trace!("DynCommandWriterAllocator::on_frame_complete: DynCommandWriterAllocator on_frame_complete completed finishing frame {}", guard.current_frame_index);

        // Bump current frame index
        guard.current_frame_index += 1;
        Ok(())
    }

    fn drain_drop_rx(inner: &mut DynCommandWriterAllocatorInner) {
        for writer in inner.drop_rx.try_iter() {
            if writer.submits_in_frame_index >= inner.current_frame_index {
                // insert in pending
                let meta = PendingCommandPoolMeta {
                    submits_in_frame_index: writer.submits_in_frame_index,
                    command_pool_meta: writer.pool.command_pool_meta.clone(),
                };

                log::trace!(
                    "DynCommandWriterAllocator::drain_drop_rx: dropped DynCommandWriter({}) moved in pending map {:?}",
                    writer.writer_id,
                    meta,
                );

                inner.pending_writers.entry(meta).or_default().push(writer);
            } else {
                log::trace!(
                    "DynCommandWriterAllocator::drain_drop_rx: dropped DynCommandWriter({}) moved to submitted map {}",
                    writer.writer_id,
                    writer.submits_in_frame_index
                );

                // insert in submitted
                inner
                    .submitted_writers
                    .entry(writer.submits_in_frame_index)
                    .or_default()
                    .push(writer);
            }
        }
    }
}
