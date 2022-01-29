use crossbeam_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::num::Wrapping;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
struct FreeListSuballocatorAllocationOffsetLength {
    offset: u32,
    length: u32,
}

#[derive(Clone)]
struct FreeListSuballocatorAllocationInner {
    raw_offset_length: FreeListSuballocatorAllocationOffsetLength,
    aligned_offset: u32,
    free_tx: Sender<FreeListSuballocatorAllocationOffsetLength>,
}

impl Drop for FreeListSuballocatorAllocationInner {
    fn drop(&mut self) {
        self.free_tx.send(self.raw_offset_length).unwrap();
    }
}

#[derive(Clone)]
pub struct FreeListSuballocatorAllocation {
    inner: Arc<FreeListSuballocatorAllocationInner>,
}

impl FreeListSuballocatorAllocation {
    pub fn aligned_offset(&self) -> u32 {
        self.inner.aligned_offset
    }
}

#[derive(Clone, Copy)]
struct FreeListSuballocatorRegion {
    offset: u32,
    length: u32,
}

struct PendingFree {
    offset_length: FreeListSuballocatorAllocationOffsetLength,
    live_until_frame: Wrapping<u32>,
}

pub struct FreeListSuballocator {
    free_regions: Vec<FreeListSuballocatorRegion>,

    free_tx: Sender<FreeListSuballocatorAllocationOffsetLength>,
    free_rx: Receiver<FreeListSuballocatorAllocationOffsetLength>,

    // We are assuming that all resources can survive for the same amount of time so the data in
    // this VecDeque will naturally be orderered such that things that need to be destroyed sooner
    // are at the front
    pending_frees: VecDeque<PendingFree>,

    // All pending frees will be destroyed after N frames
    max_in_flight_frames: Wrapping<u32>,

    // Incremented when on_frame_complete is called
    frame_index: Wrapping<u32>,
    //NOTE: We don't need to bother processing any queued data when FreeListSuballocator is dropped
    // because we do not own any real GPU resources.
}

impl FreeListSuballocator {
    pub fn new(heap_size: u32) -> Self {
        let (free_tx, free_rx) = crossbeam_channel::unbounded();

        let free_region = FreeListSuballocatorRegion {
            offset: 0,
            length: heap_size,
        };

        let mut free_regions = Vec::default();
        free_regions.push(free_region);

        Self {
            free_regions,
            free_tx,
            free_rx,
            pending_frees: VecDeque::default(),
            max_in_flight_frames: Wrapping(rafx::framework::MAX_FRAMES_IN_FLIGHT as u32),
            frame_index: Wrapping(0),
        }
    }

    pub fn on_frame_complete(&mut self) {
        // Pull any messages from dropped allocations into the VecDeque and tag them with the current frame index
        for offset_length in self.free_rx.try_iter() {
            self.pending_frees.push_back(PendingFree {
                offset_length,
                live_until_frame: self.frame_index + self.max_in_flight_frames,
            })
        }

        self.frame_index += Wrapping(1);

        // Determine how many pending frees are ready
        let mut ready_free_count = 0;
        for pending_free in &self.pending_frees {
            // If frame_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if pending_free.live_until_frame - self.frame_index > Wrapping(std::u32::MAX / 2) {
                ready_free_count += 1;
            } else {
                break;
            }
        }

        // Dequeue and free the blocks that have been unused for enough frames
        let ready_frees: Vec<_> = self.pending_frees.drain(0..ready_free_count).collect();

        for ready_free in ready_frees {
            Self::do_free(&mut self.free_regions, ready_free.offset_length);
        }
    }

    pub fn allocate(
        &mut self,
        length: u32,
        required_alignment: u32,
    ) -> Option<FreeListSuballocatorAllocation> {
        Self::do_allocate(&mut self.free_regions, length, required_alignment).map(
            |raw_offset_length| {
                let inner = FreeListSuballocatorAllocationInner {
                    raw_offset_length,
                    aligned_offset: rafx::base::memory::round_size_up_to_alignment_u32(
                        raw_offset_length.offset,
                        required_alignment,
                    ),
                    free_tx: self.free_tx.clone(),
                };

                FreeListSuballocatorAllocation {
                    inner: Arc::new(inner),
                }
            },
        )
    }

    fn do_allocate(
        free_regions: &mut Vec<FreeListSuballocatorRegion>,
        length: u32,
        required_alignment: u32,
    ) -> Option<FreeListSuballocatorAllocationOffsetLength> {
        // First, find an appropriate region to try to use
        let mut found_region_index = None;
        for i in (0..free_regions.len()).rev() {
            let free_region = free_regions[i];
            let aligned_offset = rafx::base::memory::round_size_up_to_alignment_u32(
                free_region.offset,
                required_alignment,
            );
            if aligned_offset + length <= free_region.offset + free_region.length {
                found_region_index = Some(i);
                break;
            }
        }

        // If a region was found, we need to remove it, split off the part we need, and put anything remaining back
        if let Some(found_region_index) = found_region_index {
            // Take the found region out of the list
            let found_region = free_regions.swap_remove(found_region_index);

            // Create an allocation with enough size to meet alignment requirements
            let aligned_offset = rafx::base::memory::round_size_up_to_alignment_u32(
                found_region.offset,
                required_alignment,
            );
            let allocation = FreeListSuballocatorAllocationOffsetLength {
                offset: found_region.offset,
                length: aligned_offset + length - found_region.offset,
            };

            // Create a free list region for the remaining memory, if any remains
            let remaining_length = found_region.length - allocation.length;
            if remaining_length > 0 {
                free_regions.push(FreeListSuballocatorRegion {
                    offset: found_region.offset + allocation.length,
                    length: remaining_length,
                });
            }

            println!("return allocation {:?}", allocation);
            return Some(allocation);
        }

        return None;
    }

    fn do_free(
        free_regions: &mut Vec<FreeListSuballocatorRegion>,
        offset_length: FreeListSuballocatorAllocationOffsetLength,
    ) {
        // Begin/end byte offset of the block being returned to the heap
        let allocation_begin = offset_length.offset;
        let allocation_end = offset_length.offset + offset_length.length;

        // Info about the blocks adjacent to the one being returned to the heap
        let mut previous_block_offset = None;
        let mut previous_block_length = None;
        let mut next_block_length = None;

        // Linear search for any adjacent blocks (a binary search could speed this up, but we
        // should avoid excessive allocation/free anyways)
        for i in (0..free_regions.len()).rev() {
            let free_region = free_regions[i];
            if allocation_begin == free_region.offset + free_region.length {
                previous_block_offset = Some(free_region.offset);
                previous_block_length = Some(free_region.length);

                // We will insert a new block that is merged with this block
                free_regions.swap_remove(i);
            }

            if allocation_end == free_region.offset {
                next_block_length = Some(free_region.length);

                // We will insert a new block that is merged with this block
                free_regions.swap_remove(i);
            }
        }

        // Insert the block, merging with previous/next blocks, if they were found
        let new_free_region = FreeListSuballocatorRegion {
            offset: previous_block_offset.unwrap_or(offset_length.offset),
            length: previous_block_length.unwrap_or(0)
                + next_block_length.unwrap_or(0)
                + offset_length.length,
        };

        free_regions.push(new_free_region);
    }
}
