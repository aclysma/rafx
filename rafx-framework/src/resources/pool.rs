use rafx_api::{RafxDescriptorSetArray, RafxDeviceContext, RafxResult};
use std::collections::VecDeque;
use std::num::Wrapping;

pub type PoolResourceAllocatorAllocFn<T> =
    dyn Fn(&RafxDeviceContext) -> RafxResult<T> + Send + Sync;

/// Implement to customize how PoolAllocator resets and destroys pools
pub trait PooledResourceImpl {
    fn reset(&mut self) -> RafxResult<()>;
}

struct PoolResourceInFlight<T: PooledResourceImpl> {
    pool: T,
    live_until_frame: Wrapping<u32>,
}

/// This handles waiting for N frames to pass before resetting the pool. "Restting" could mean
/// different things depending on the resource. This allocator also has a callback for allocating
/// new pools for use. A maximum pool count should be provided so that an unbounded leak of pools
/// can be detected.
pub struct PooledResourceAllocator<T: PooledResourceImpl> {
    device_context: RafxDeviceContext,

    // Allocates a new pool
    allocate_fn: Box<PoolResourceAllocatorAllocFn<T>>,

    // We are assuming that all pools can survive for the same amount of time so the data in
    // this VecDeque will naturally be orderered such that things that need to be reset sooner
    // are at the front
    in_flight_pools: VecDeque<PoolResourceInFlight<T>>,

    // Pools that have been reset and are ready for allocation
    reset_pools: Vec<T>,

    // All pools that are retired will be reset after N frames
    max_in_flight_frames: Wrapping<u32>,

    // Incremented when on_frame_complete is called
    frame_index: Wrapping<u32>,

    // Number of pools we have created in total
    created_pool_count: u32,

    // Max number of pools to create (sum includes allocated pools, pools in flight, and reset pools
    max_pool_count: u32,
}

impl<T: PooledResourceImpl> PooledResourceAllocator<T> {
    /// Create a pool allocator that will reset resources after N frames. Keep in mind that if for
    /// example you want to push a single resource per frame, up to N+1 resources will exist
    /// in the sink. If max_in_flight_frames is 2, then you would have a resource that has
    /// likely not been submitted to the GPU yet, plus a resource per the N frames that have
    /// been submitted
    pub fn new<F: Fn(&RafxDeviceContext) -> RafxResult<T> + Send + Sync + 'static>(
        device_context: &RafxDeviceContext,
        max_in_flight_frames: u32,
        max_pool_count: u32,
        allocate_fn: F,
    ) -> Self {
        PooledResourceAllocator {
            device_context: device_context.clone(),
            allocate_fn: Box::new(allocate_fn),
            in_flight_pools: Default::default(),
            reset_pools: Default::default(),
            max_in_flight_frames: Wrapping(max_in_flight_frames),
            frame_index: Wrapping(0),
            created_pool_count: 0,
            max_pool_count,
        }
    }

    /// Allocate a pool - either reusing an old one that has been reset or creating a new one. Will
    /// assert that we do not exceed max_pool_count. The pool is allowed to exist until retire_pool
    /// is called. After this point, we will wait for N frames before restting it.
    pub fn allocate_pool(&mut self) -> RafxResult<T> {
        self.reset_pools.pop().map(Ok).unwrap_or_else(|| {
            self.created_pool_count += 1;
            assert!(self.created_pool_count <= self.max_pool_count);
            (self.allocate_fn)(&self.device_context)
        })
    }

    /// Schedule the pool to reset after we complete N frames
    pub fn retire_pool(
        &mut self,
        pool: T,
    ) {
        self.in_flight_pools.push_back(PoolResourceInFlight {
            pool,
            live_until_frame: self.frame_index + self.max_in_flight_frames + Wrapping(1),
        });
    }

    /// Call when we are ready to reset another set of resources, most likely when a frame is
    /// presented or a new frame begins
    pub fn update(&mut self) -> RafxResult<()> {
        self.frame_index += Wrapping(1);

        // Determine how many pools we can drain
        let mut pools_to_drain = 0;
        for in_flight_pool in &self.in_flight_pools {
            // If frame_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if in_flight_pool.live_until_frame - self.frame_index > Wrapping(std::u32::MAX / 2) {
                pools_to_drain += 1;
            } else {
                break;
            }
        }

        // Reset them and add them to the list of pools ready to be allocated
        let pools_to_reset: Vec<_> = self.in_flight_pools.drain(0..pools_to_drain).collect();
        for mut pool_to_reset in pools_to_reset {
            T::reset(&mut pool_to_reset.pool)?;
            self.reset_pools.push(pool_to_reset.pool);
        }

        Ok(())
    }

    /// Immediately destroy everything. We assume the device is idle and nothing is in flight.
    /// Calling this function when the device is not idle could result in a deadlock
    pub fn destroy(&mut self) -> RafxResult<()> {
        for pool in self.in_flight_pools.drain(..) {
            std::mem::drop(pool.pool);
        }

        for pool in self.reset_pools.drain(..) {
            std::mem::drop(pool);
        }

        Ok(())
    }
}

// We assume destroy was called
impl<T: PooledResourceImpl> Drop for PooledResourceAllocator<T> {
    fn drop(&mut self) {
        assert!(self.in_flight_pools.is_empty());
        assert!(self.reset_pools.is_empty())
    }
}

//
// Implementation for descriptor pools
//
impl PooledResourceImpl for RafxDescriptorSetArray {
    fn reset(&mut self) -> RafxResult<()> {
        Ok(())
    }
}

pub type DescriptorSetArrayPoolAllocator = PooledResourceAllocator<RafxDescriptorSetArray>;
