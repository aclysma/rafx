use rafx_api::RafxResult;
use std::collections::VecDeque;
use std::num::Wrapping;

struct DropSinkResourceInFlight<T> {
    // prefixed with _ to silence "field not used" warning. The purpose of the var is to hold the
    // resource for a while then drop this entire structure
    _resource: T,
    live_until_frame: Wrapping<u32>,
}

/// This handles waiting for N frames to pass before dropping the resource.
pub struct ResourceDropSink<T> {
    // We are assuming that all resources can survive for the same amount of time so the data in
    // this VecDeque will naturally be orderered such that things that need to be destroyed sooner
    // are at the front
    resources_in_flight: VecDeque<DropSinkResourceInFlight<T>>,

    // All resources pushed into the sink will be destroyed after N frames
    max_in_flight_frames: Wrapping<u32>,

    // Incremented when on_frame_complete is called
    frame_index: Wrapping<u32>,
}

impl<T> ResourceDropSink<T> {
    /// Create a drop sink that will destroy resources after N frames. Keep in mind that if for
    /// example you want to push a single resource per frame, up to N+1 resources will exist
    /// in the sink. If max_in_flight_frames is 2, then you would have a resource that has
    /// likely not been submitted to the GPU yet, plus a resource per the N frames that have
    /// been submitted
    pub fn new(max_in_flight_frames: u32) -> Self {
        ResourceDropSink {
            resources_in_flight: Default::default(),
            max_in_flight_frames: Wrapping(max_in_flight_frames),
            frame_index: Wrapping(0),
        }
    }

    /// Schedule the resource to drop after we complete N frames
    pub fn retire(
        &mut self,
        resource: T,
    ) {
        self.resources_in_flight
            .push_back(DropSinkResourceInFlight::<T> {
                _resource: resource,
                live_until_frame: self.frame_index + self.max_in_flight_frames + Wrapping(1),
            });
    }

    /// Call when we are ready to drop another set of resources, most likely when a frame is
    /// presented or a new frame begins
    pub fn on_frame_complete(&mut self) -> RafxResult<()> {
        self.frame_index += Wrapping(1);

        // Determine how many resources we should drain
        let mut resources_to_drop = 0;
        for resource_in_flight in &self.resources_in_flight {
            // If frame_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if resource_in_flight.live_until_frame - self.frame_index > Wrapping(std::u32::MAX / 2)
            {
                resources_to_drop += 1;
            } else {
                break;
            }
        }

        // Reset them and add them to the list of pools ready to be allocated
        let resources_to_drop: Vec<_> = self
            .resources_in_flight
            .drain(0..resources_to_drop)
            .collect();

        std::mem::drop(resources_to_drop);

        Ok(())
    }

    /// Immediately destroy everything. We assume the device is idle and nothing is in flight.
    /// Calling this function when the device is not idle could result in a deadlock
    pub fn destroy(&mut self) -> RafxResult<()> {
        for resource in self.resources_in_flight.drain(..) {
            std::mem::drop(resource);
        }

        Ok(())
    }
}

// We assume destroy was called
impl<T> Drop for ResourceDropSink<T> {
    fn drop(&mut self) {
        assert!(self.resources_in_flight.is_empty())
    }
}
