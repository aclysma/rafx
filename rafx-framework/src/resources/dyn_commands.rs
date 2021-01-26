use crossbeam_channel::{Receiver, Sender};
use fnv::FnvHashMap;
use rafx_api::{
    RafxCommandBuffer, RafxCommandBufferDef, RafxCommandPool, RafxCommandPoolDef, RafxQueue,
    RafxResult,
};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub struct DynCommandBuffer(Arc<RafxCommandBuffer>);

impl Deref for DynCommandBuffer {
    type Target = RafxCommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for DynCommandBuffer {
    fn clone(&self) -> Self {
        DynCommandBuffer(self.0.clone())
    }
}

/// Info we hash across to identify equivalent command pools, allowing us to share them
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPoolMeta {
    queue_id: u32,
    command_pool_def: RafxCommandPoolDef,
}

// Internally represents a VkCommandPool with automatic lifetime/reuse management
struct DynCommandPoolInner {
    command_pool: RafxCommandPool,
    command_pool_meta: CommandPoolMeta,
    allocated_command_buffers: Vec<DynCommandBuffer>,
    submits_in_frame_index: u64,

    // Just a debugging aid
    pool_id: u64,
}

impl DynCommandPoolInner {
    fn reset_command_pool(&mut self) -> RafxResult<()> {
        for command_buffer in &self.allocated_command_buffers {
            command_buffer.return_to_pool()?;
        }

        self.allocated_command_buffers.clear();
        self.command_pool.reset_command_pool()
    }
}

/// A helper that can be allocated as needed to create very short-lived command buffers. The object
/// may not be persisted across frames. Instead, allocated a new one every frame. They are pooled,
/// allocation is cheap and thread-safe.
///
/// This is designed for fire-and-forget command buffers. A DynCommandPool borrows a command pool
/// that is not in use and not in flight, allocates out of it, resets itself after the appropriate
/// number of frames pass, and returns itself to the pool for future reuse. See allocate_dyn_pool
/// for more details
pub struct DynCommandPool {
    // This should never be None. We always allocate this to a non-none value and we don't clear
    // it until the drop handler
    inner: Option<DynCommandPoolInner>,
    drop_tx: Sender<DynCommandPoolInner>,
}

impl DynCommandPool {
    fn new(
        inner: DynCommandPoolInner,
        drop_tx: Sender<DynCommandPoolInner>,
    ) -> Self {
        log::trace!(
            "Creating DynCommandPool({}) {:?}",
            inner.pool_id,
            inner.command_pool_meta
        );

        DynCommandPool {
            inner: Some(inner),
            drop_tx,
        }
    }

    /// Allocate a command buffer and call begin() on it
    pub fn allocate_dyn_command_buffer(
        &mut self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<DynCommandBuffer> {
        let inner = self.inner.as_mut().unwrap();
        log::trace!(
            "DynCommandPool({}) allocate_command_buffer: {:?}",
            inner.pool_id,
            command_buffer_def
        );

        let command_buffer = inner
            .command_pool
            .create_command_buffer(command_buffer_def)?;
        //command_buffer.begin()?;

        let command_buffer_inner = Arc::new(command_buffer);
        let dyn_command_buffer = DynCommandBuffer(command_buffer_inner.clone());

        inner
            .allocated_command_buffers
            .push(dyn_command_buffer.clone());
        Ok(dyn_command_buffer)
    }

    /// Get the underlying pool within the allocator. The pool will be destroyed after
    /// MAX_FRAMES_IN_FLIGHT pass, and all command buffers created with it must follow the same
    /// restrictions as a command buffer created via begin_command_buffer/end_command_buffer. It's
    /// recommended to use begin_writing/end_writing as it is less error prone.
    pub fn pool(&mut self) -> &mut RafxCommandPool {
        &mut self.inner.as_mut().unwrap().command_pool
    }
}

impl Drop for DynCommandPool {
    fn drop(&mut self) {
        let inner = self.inner.take().unwrap();
        self.drop_tx.send(inner).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingCommandPoolMeta {
    submits_in_frame_index: u64,
    command_pool_meta: CommandPoolMeta,
}

struct DynCommandPoolAllocatorInner {
    // Command pools that are ready to use but have no recorded commands
    unused_pools: FnvHashMap<CommandPoolMeta, Vec<DynCommandPoolInner>>,

    // Command pools that are in use and have a frame that we know they will be submitted in
    pending_pools: FnvHashMap<PendingCommandPoolMeta, Vec<DynCommandPoolInner>>,

    // submitted pools
    // TODO: Would be less allocations if this was a static array of vecs
    submitted_pools: BTreeMap<u64, Vec<DynCommandPoolInner>>,

    max_frames_in_flight: u64,
    current_frame_index: u64,

    drop_tx: Sender<DynCommandPoolInner>,
    drop_rx: Receiver<DynCommandPoolInner>,

    // Just a debugging aid
    next_pool_id: u64,
}

/// An allocator for DynCommandPools, objects that are short-lived and NOT persisted across
/// frames. Meant for allocating command buffers that are usually single use and only for the
/// current frame. The allocator is multi-thread friendly, but the pools themselves are not. So
/// if writing command buffers from multiple threads, allocate a pool per thread.
#[derive(Clone)]
pub struct DynCommandPoolAllocator {
    inner: Arc<Mutex<DynCommandPoolAllocatorInner>>,
}

impl DynCommandPoolAllocator {
    /// Create an allocator for DynCommandPools.
    pub fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        let inner = DynCommandPoolAllocatorInner {
            max_frames_in_flight: max_frames_in_flight as u64,
            pending_pools: Default::default(),
            submitted_pools: Default::default(),
            unused_pools: Default::default(),
            current_frame_index: 0,
            drop_tx,
            drop_rx,
            next_pool_id: 0,
        };

        DynCommandPoolAllocator {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Allocates a pool. DynPools wrap CommandPools. The parameters match inputs for
    /// CommandPool::new. `delay_submission_by_frame_count` indicates how many frames will pass
    /// before the commands will be submitted (which affects how long-lived they will be). DO NOT
    /// submit command buffers earlier than this as the commands pools themselves are pooled and
    /// may be available to writing in future frames.
    ///
    /// The common case for delay_submission_by_frame_count is to pass 0. You might pass 1 if for
    /// example, you are building a command buffer for frame N + 1 while frame N is not yet
    /// submitted.
    pub fn allocate_dyn_pool(
        &self,
        queue: &RafxQueue,
        command_pool_def: &RafxCommandPoolDef,
        delay_submission_by_frame_count: u64,
    ) -> RafxResult<DynCommandPool> {
        let mut guard = self.inner.lock().unwrap();

        // Determine what frame this will be committed in
        let submits_in_frame_index = guard.current_frame_index + delay_submission_by_frame_count;

        // Build a key to search for an existing pool to reuse
        let meta = PendingCommandPoolMeta {
            submits_in_frame_index,
            command_pool_meta: CommandPoolMeta {
                queue_id: queue.queue_id(),
                command_pool_def: command_pool_def.clone(),
            },
        };

        log::trace!("DynCommandPoolAllocator::allocate_dyn_pool {:?}", meta);

        Self::drain_drop_rx(&mut *guard);

        // Try to find something in the pending collection and reuse it
        if let Some(pools) = guard.pending_pools.get_mut(&meta) {
            if let Some(pool) = pools.pop() {
                log::trace!(
                    "DynCommandPoolAllocator::allocate_dyn_pool {:?} reusing pending pool DynCommandPool({})",
                    meta,
                    pool.pool_id
                );
                assert_eq!(pool.submits_in_frame_index, submits_in_frame_index);
                return Ok(DynCommandPool::new(pool, guard.drop_tx.clone()));
            }
        }

        // If we don't have a "dirty" pool for this frame yet, try to reuse an existing unused one
        if let Some(pools) = guard.unused_pools.get_mut(&meta.command_pool_meta) {
            if let Some(mut pool) = pools.pop() {
                log::trace!(
                    "DynCommandPoolAllocator::allocate_dyn_pool {:?} reusing unused pool DynCommandPool({})",
                    meta,
                    pool.pool_id
                );
                pool.submits_in_frame_index = submits_in_frame_index;
                return Ok(DynCommandPool::new(pool, guard.drop_tx.clone()));
            }
        }

        let pool_id = guard.next_pool_id;
        guard.next_pool_id += 1;

        log::trace!(
            "DynCommandPoolAllocator::allocate_dyn_pool {:?} creating new DynCommandPool({})",
            meta,
            pool_id
        );

        let command_pool_meta = CommandPoolMeta {
            queue_id: queue.queue_id(),
            command_pool_def: command_pool_def.clone(),
        };

        let command_pool = queue.create_command_pool(command_pool_def)?;

        let inner = DynCommandPoolInner {
            command_pool,
            command_pool_meta,
            allocated_command_buffers: Vec::default(),
            submits_in_frame_index,
            pool_id,
        };

        Ok(DynCommandPool::new(inner, guard.drop_tx.clone()))
    }

    /// Call every frame to recycle command pools that are no-longer in flight
    #[profiling::function]
    pub fn on_frame_complete(&self) -> RafxResult<()> {
        let mut guard = self.inner.lock().unwrap();
        log::trace!("DynCommandPoolAllocator::on_frame_complete: DynCommandPoolAllocator on_frame_complete finishing frame {}", guard.current_frame_index);

        {
            profiling::scope!("drain_drop_rx");
            Self::drain_drop_rx(&mut *guard);
        }

        // Find any pending pools that should submit during this frame
        let mut pending_pool_keys = Vec::default();
        for key in guard.pending_pools.keys() {
            if key.submits_in_frame_index == guard.current_frame_index {
                pending_pool_keys.push(key.clone());
            }
        }

        // Move them to the submitted pools collection
        for key in pending_pool_keys {
            let mut pending_pools = guard.pending_pools.remove(&key).unwrap();

            for pending_pool in &pending_pools {
                log::trace!(
                    "DynCommandPoolAllocator::on_frame_complete: DynCommandPool({}) being moved to submitted pool list",
                    pending_pool.pool_id,
                );
            }

            guard
                .submitted_pools
                .entry(key.submits_in_frame_index)
                .or_default()
                .append(&mut pending_pools);
        }

        // Find all the submitted pools that are old enough to no longer be in flight
        let mut submitted_pool_keys = Vec::default();
        for &submits_in_frame_index in guard.submitted_pools.keys() {
            // We can use >= here because we're bumping current_frame_index at the end of this
            // function
            if guard.current_frame_index >= submits_in_frame_index + guard.max_frames_in_flight {
                submitted_pool_keys.push(submits_in_frame_index);
            } else {
                // The map is sorted by frame count
                break;
            }
        }

        // Move them to the unused collection
        for key in submitted_pool_keys {
            let submitted_pools = guard.submitted_pools.remove(&key).unwrap();
            for mut submitted_pool in submitted_pools {
                log::trace!(
                    "DynCommandPoolAllocator::on_frame_complete: DynCommandPool({}) being moved to unused pool map",
                    submitted_pool.pool_id,
                );

                let meta = submitted_pool.command_pool_meta.clone();
                {
                    profiling::scope!("reset_command_pool");
                    submitted_pool.reset_command_pool()?;
                }

                guard
                    .unused_pools
                    .entry(meta)
                    .or_default()
                    .push(submitted_pool);
            }
        }

        log::trace!("DynCommandPoolAllocator::on_frame_complete: DynCommandPoolAllocator on_frame_complete completed finishing frame {}", guard.current_frame_index);

        // Bump current frame index
        guard.current_frame_index += 1;
        Ok(())
    }

    fn drain_drop_rx(inner: &mut DynCommandPoolAllocatorInner) {
        for pool in inner.drop_rx.try_iter() {
            if pool.submits_in_frame_index >= inner.current_frame_index {
                // insert in pending
                let meta = PendingCommandPoolMeta {
                    submits_in_frame_index: pool.submits_in_frame_index,
                    command_pool_meta: pool.command_pool_meta.clone(),
                };

                log::trace!(
                    "DynCommandPoolAllocator::drain_drop_rx: dropped DynCommandPool({}) moved in pending map {:?}",
                    pool.pool_id,
                    meta,
                );

                inner.pending_pools.entry(meta).or_default().push(pool);
            } else {
                log::trace!(
                    "DynCommandPoolAllocator::drain_drop_rx: dropped DynCommandPool({}) moved to submitted map {}",
                    pool.pool_id,
                    pool.submits_in_frame_index
                );

                // insert in submitted
                inner
                    .submitted_pools
                    .entry(pool.submits_in_frame_index)
                    .or_default()
                    .push(pool);
            }
        }
    }
}
