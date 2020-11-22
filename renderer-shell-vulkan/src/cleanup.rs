use crate::image::VkImageRaw;
use crate::{VkBuffer, VkBufferRaw, VkDeviceContext, VkImage};
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk;
use std::collections::VecDeque;
use std::mem::ManuallyDrop;
use std::num::Wrapping;

//use crossbeam_channel::{Sender, Receiver};

/// Implement to customize how VkResourceDropSink drops resources
pub trait VkResource {
    //TODO: Error type that is compatible with VMA
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()>;
}

struct VkDropSinkResourceInFlight<T: VkResource> {
    resource: T,
    live_until_frame: Wrapping<u32>,
}

/// This handles waiting for N frames to pass before dropping the resource. "Dropping" could mean
/// different things depending on the resource.. for example a VkImage we would want to literally
/// drop and a vk::ImageView we would want to call device.destroy_image_view. The reason for not
/// unilaterally supporting drop for everything is that it requires the resource carrying around
/// a channel or Arc with the data required to actually perform the drop
pub struct VkResourceDropSink<T: VkResource> {
    // We are assuming that all resources can survive for the same amount of time so the data in
    // this VecDeque will naturally be orderered such that things that need to be destroyed sooner
    // are at the front
    resources_in_flight: VecDeque<VkDropSinkResourceInFlight<T>>,

    // All resources pushed into the sink will be destroyed after N frames
    max_in_flight_frames: Wrapping<u32>,

    // Incremented when on_frame_complete is called
    frame_index: Wrapping<u32>,
}

impl<T: VkResource> VkResourceDropSink<T> {
    /// Create a drop sink that will destroy resources after N frames. Keep in mind that if for
    /// example you want to push a single resource per frame, up to N+1 resources will exist
    /// in the sink. If max_in_flight_frames is 2, then you would have a resource that has
    /// likely not been submitted to the GPU yet, plus a resource per the N frames that have
    /// been submitted
    pub fn new(max_in_flight_frames: u32) -> Self {
        VkResourceDropSink {
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
            .push_back(VkDropSinkResourceInFlight::<T> {
                resource,
                live_until_frame: self.frame_index + self.max_in_flight_frames + Wrapping(1),
            });
    }

    /// Call when we are ready to drop another set of resources, most likely when a frame is
    /// presented or a new frame begins
    pub fn on_frame_complete(
        &mut self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
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
        for resource_to_drop in resources_to_drop {
            T::destroy(device_context, resource_to_drop.resource)?;
        }

        Ok(())
    }

    /// Immediately destroy everything. We assume the device is idle and nothing is in flight.
    /// Calling this function when the device is not idle could result in a deadlock
    pub fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        unsafe {
            device_context.device().device_wait_idle()?;
        }

        for resource in self.resources_in_flight.drain(..) {
            T::destroy(device_context, resource.resource)?;
        }

        Ok(())
    }
}

// We assume destroy was called
impl<T: VkResource> Drop for VkResourceDropSink<T> {
    fn drop(&mut self) {
        assert!(self.resources_in_flight.is_empty())
    }
}

//
// A simple helper to put a thread-friendly shell around VkResourceDropSink
//
/*
pub struct VkResourceDropSinkChannel<T: VkResource> {
    tx: Sender<T>,
    rx: Receiver<T>,
}

impl<T: VkResource> VkResourceDropSinkChannel<T> {
    pub fn retire(
        &mut self,
        resource: T,
    ) {
        self.tx.send(resource).unwrap();
    }

    pub fn retire_queued_resources(
        &self,
        drop_sink: &mut VkResourceDropSink<T>,
    ) {
        for resource in self.rx.try_iter() {
            drop_sink.retire(resource);
        }
    }
}

impl<T: VkResource> VkResourceDropSinkChannel<T> {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        VkResourceDropSinkChannel { tx, rx }
    }
}

impl<T: VkResource> Clone for VkResourceDropSinkChannel<T> {
    fn clone(&self) -> Self {
        VkResourceDropSinkChannel {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}
*/

//
// Blanket implementation for anything that is ManuallyDrop
//
impl<T> VkResource for ManuallyDrop<T> {
    fn destroy(
        _device_context: &VkDeviceContext,
        mut resource: Self,
    ) -> VkResult<()> {
        unsafe {
            ManuallyDrop::drop(&mut resource);
            Ok(())
        }
    }
}

impl VkResource for VkBufferRaw {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        device_context
            .allocator()
            .destroy_buffer(resource.buffer, &resource.allocation);
        Ok(())
    }
}

impl VkResource for VkImageRaw {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        if let Some(allocation) = &resource.allocation {
            device_context
                .allocator()
                .destroy_image(resource.image, allocation);
            Ok(())
        } else {
            // This path where we don't deallocate is appropriate for swapchain images or other
            // images that are owned externally from renderer systems
            Ok(())
        }
    }
}

//
// Implementation for ImageViews
//
impl VkResource for vk::ImageView {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context.device().destroy_image_view(resource, None);
            Ok(())
        }
    }
}

//
// Implementation for Samplers
//
impl VkResource for vk::Sampler {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context.device().destroy_sampler(resource, None);
            Ok(())
        }
    }
}

//
// Implementation for pipelines
//
impl VkResource for vk::Pipeline {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context.device().destroy_pipeline(resource, None);
            Ok(())
        }
    }
}

//
// Implementation for renderpasses
//
impl VkResource for vk::RenderPass {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context.device().destroy_render_pass(resource, None);
            Ok(())
        }
    }
}

//
// Implementation for framebuffers
//
impl VkResource for vk::Framebuffer {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context.device().destroy_framebuffer(resource, None);
            Ok(())
        }
    }
}

//
// Implementation for pipeline layouts
//
impl VkResource for vk::PipelineLayout {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context
                .device()
                .destroy_pipeline_layout(resource, None);
            Ok(())
        }
    }
}

//
// Implementation for pipeline layouts
//
impl VkResource for vk::DescriptorSetLayout {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context
                .device()
                .destroy_descriptor_set_layout(resource, None);
            Ok(())
        }
    }
}

//
// Implementation for shader modules
//
impl VkResource for vk::ShaderModule {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        unsafe {
            device_context
                .device()
                .destroy_shader_module(resource, None);
            Ok(())
        }
    }
}

/// Provides DropSinks for all the things in a single struct
pub struct VkCombinedDropSink {
    buffers: VkResourceDropSink<ManuallyDrop<VkBuffer>>,
    images: VkResourceDropSink<ManuallyDrop<VkImage>>,
    image_views: VkResourceDropSink<vk::ImageView>,
    pipelines: VkResourceDropSink<vk::Pipeline>,
    render_passes: VkResourceDropSink<vk::RenderPass>,
    framebuffers: VkResourceDropSink<vk::Framebuffer>,
    pipeline_layouts: VkResourceDropSink<vk::PipelineLayout>,
    descriptor_sets: VkResourceDropSink<vk::DescriptorSetLayout>,
    shader_modules: VkResourceDropSink<vk::ShaderModule>,
}

impl VkCombinedDropSink {
    pub fn new(max_in_flight_frames: u32) -> Self {
        VkCombinedDropSink {
            buffers: VkResourceDropSink::new(max_in_flight_frames),
            images: VkResourceDropSink::new(max_in_flight_frames),
            image_views: VkResourceDropSink::new(max_in_flight_frames),
            pipelines: VkResourceDropSink::new(max_in_flight_frames),
            render_passes: VkResourceDropSink::new(max_in_flight_frames),
            framebuffers: VkResourceDropSink::new(max_in_flight_frames),
            pipeline_layouts: VkResourceDropSink::new(max_in_flight_frames),
            descriptor_sets: VkResourceDropSink::new(max_in_flight_frames),
            shader_modules: VkResourceDropSink::new(max_in_flight_frames),
        }
    }

    pub fn retire_buffer(
        &mut self,
        resource: ManuallyDrop<VkBuffer>,
    ) {
        self.buffers.retire(resource);
    }

    pub fn retire_image(
        &mut self,
        resource: ManuallyDrop<VkImage>,
    ) {
        self.images.retire(resource);
    }

    pub fn retire_image_view(
        &mut self,
        resource: vk::ImageView,
    ) {
        self.image_views.retire(resource);
    }

    pub fn retire_pipeline(
        &mut self,
        resource: vk::Pipeline,
    ) {
        self.pipelines.retire(resource);
    }

    pub fn retire_render_pass(
        &mut self,
        resource: vk::RenderPass,
    ) {
        self.render_passes.retire(resource);
    }

    pub fn retire_framebuffer(
        &mut self,
        resource: vk::Framebuffer,
    ) {
        self.framebuffers.retire(resource);
    }

    pub fn retire_pipeline_layout(
        &mut self,
        resource: vk::PipelineLayout,
    ) {
        self.pipeline_layouts.retire(resource);
    }

    pub fn retire_descriptor_set_layout(
        &mut self,
        resource: vk::DescriptorSetLayout,
    ) {
        self.descriptor_sets.retire(resource);
    }

    pub fn retire_shader_module(
        &mut self,
        resource: vk::ShaderModule,
    ) {
        self.shader_modules.retire(resource);
    }

    pub fn on_frame_complete(
        &mut self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        self.image_views.on_frame_complete(device_context)?;
        self.images.on_frame_complete(device_context)?;
        self.buffers.on_frame_complete(device_context)?;
        self.pipelines.on_frame_complete(device_context)?;
        self.render_passes.on_frame_complete(device_context)?;
        self.framebuffers.on_frame_complete(device_context)?;
        self.pipeline_layouts.on_frame_complete(device_context)?;
        self.descriptor_sets.on_frame_complete(device_context)?;
        self.shader_modules.on_frame_complete(device_context)?;
        Ok(())
    }

    pub fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        self.image_views.destroy(device_context)?;
        self.images.destroy(device_context)?;
        self.buffers.destroy(device_context)?;
        self.pipelines.destroy(device_context)?;
        self.render_passes.destroy(device_context)?;
        self.framebuffers.destroy(device_context)?;
        self.pipeline_layouts.destroy(device_context)?;
        self.descriptor_sets.destroy(device_context)?;
        self.shader_modules.destroy(device_context)?;
        Ok(())
    }
}
