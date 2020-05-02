use ash::prelude::VkResult;
use std::num::Wrapping;
use std::collections::VecDeque;
use ash::version::DeviceV1_0;
use std::mem::ManuallyDrop;
use ash::{Device, vk};
use crate::VkImage;

/// Implement to customize how VkResourceDropSink drops resources
pub trait VkDropSinkResourceImpl {
    fn destroy(device: &ash::Device, resource: Self) -> VkResult<()>;
}

struct VkDropSinkResourceInFlight<T: VkDropSinkResourceImpl> {
    resource: T,
    live_until_frame: Wrapping<u32>
}

/// This handles waiting for N frames to pass before dropping the resource. "Dropping" could mean
/// different things depending on the resource.. for example a VkImage we would want to literally
/// drop and a vk::ImageView we would want to call device.destroy_image_view. The reason for not
/// unilaterally supporting drop for everything is that it requires the resource carrying around
/// a channel or Arc with the data required to actually perform the drop
pub struct VkResourceDropSink<T: VkDropSinkResourceImpl> {
    // We are assuming that all resources can survive for the same amount of time so the data in
    // this VecDeque will naturally be orderered such that things that need to be destroyed sooner
    // are at the front
    resources_in_flight: VecDeque<VkDropSinkResourceInFlight<T>>,

    // All resources pushed into the sink will be destroyed after N frames
    max_in_flight_frames: Wrapping<u32>,

    // Incremented when on_frame_complete is called
    frame_index: Wrapping<u32>
}

impl<T: VkDropSinkResourceImpl> VkResourceDropSink<T> {
    /// Create a drop sink that will destroy resources after N frames. Keep in mind that if for
    /// example you want to push a single resource per frame, up to N+1 resources will exist
    /// in the sink. If max_in_flight_frames is 2, then you would have a resource that has
    /// likely not been submitted to the GPU yet, plus a resource per the N frames that have
    /// been submitted
    pub fn new(
        max_in_flight_frames: u32
    ) -> Self {
        VkResourceDropSink {
            resources_in_flight: Default::default(),
            max_in_flight_frames: Wrapping(max_in_flight_frames),
            frame_index: Wrapping(0)
        }
    }

    /// Schedule the resource to drop after we complete N frames
    pub fn retire(&mut self, resource: T) {
        self.resources_in_flight.push_back(VkDropSinkResourceInFlight::<T> {
            resource,
            live_until_frame: self.frame_index + self.max_in_flight_frames + Wrapping(1)
        });
    }

    /// Call when we are ready to drop another set of resources, most likely when a frame is
    /// presented or a new frame begins
    pub fn on_frame_complete(&mut self, device: &ash::Device) {
        self.frame_index += Wrapping(1);

        // Determine how many resources we should drain
        let mut resources_to_drop = 0;
        for resource_in_flight in &self.resources_in_flight {
            // If frame_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if resource_in_flight.live_until_frame - self.frame_index > Wrapping(std::u32::MAX / 2) {
                resources_to_drop += 1;
            } else {
                break;
            }
        }

        // Reset them and add them to the list of pools ready to be allocated
        let resources_to_drop : Vec<_> = self.resources_in_flight.drain(0..resources_to_drop).collect();
        for mut resource_to_drop in resources_to_drop {
            unsafe {
                T::destroy(device, resource_to_drop.resource);
            }
        }
    }

    /// Immediately destroy everything. We assume the device is idle and nothing is in flight.
    /// Calling this function when the device is not idle could result in a deadlock
    pub fn destroy(&mut self, device: &ash::Device) -> VkResult<()> {
        unsafe {
            device.device_wait_idle()?;
        }

        for resource in self.resources_in_flight.drain(..) {
            unsafe {
                T::destroy(device, resource.resource)?;
            }
        }

        Ok(())
    }
}

impl<T: VkDropSinkResourceImpl> Drop for VkResourceDropSink<T> {
    fn drop(&mut self) {
        assert!(self.resources_in_flight.is_empty())
    }
}


impl<T> VkDropSinkResourceImpl for ManuallyDrop<T> {
    fn destroy(device: &Device, mut resource: Self) -> VkResult<()> {
        unsafe {
            ManuallyDrop::drop(&mut resource);
            Ok(())
        }
    }
}

impl VkDropSinkResourceImpl for vk::ImageView {
    fn destroy(device: &Device, resource: Self) -> VkResult<()> {
        unsafe {
            device.destroy_image_view(resource, None);
            Ok(())
        }
    }
}

pub struct ImageDropSink {
    images: VkResourceDropSink<ManuallyDrop<VkImage>>,
    image_views: VkResourceDropSink<vk::ImageView>,
}

impl ImageDropSink {
    pub fn new(
        max_in_flight_frames: u32
    ) -> Self {
        ImageDropSink {
            images: VkResourceDropSink::new(max_in_flight_frames),
            image_views: VkResourceDropSink::new(max_in_flight_frames)
        }
    }

    pub fn retire_image(&mut self, image: ManuallyDrop<VkImage>) {
        self.images.retire(image);
    }

    pub fn retire_image_view(&mut self, image_view: vk::ImageView) {
        self.image_views.retire(image_view);
    }

    pub fn on_frame_complete(&mut self, device: &ash::Device) {
        self.image_views.on_frame_complete(device);
        self.images.on_frame_complete(device);
    }

    pub fn destroy(&mut self, device: &ash::Device) -> VkResult<()> {
        self.image_views.destroy(device)?;
        self.images.destroy(device)?;
        Ok(())
    }
}





