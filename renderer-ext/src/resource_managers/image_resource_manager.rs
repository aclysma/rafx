use std::mem;
use ash::{vk, Device};
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{
    VkDevice, VkUpload, VkTransferUpload, VkTransferUploadState, VkResourceDropSink,
    VkDescriptorPoolAllocator, VkDeviceContext,
};
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;
use atelier_assets::core::AssetUuid;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use std::process::exit;
use image::{GenericImageView, ImageFormat, load};
use ash::vk::{ShaderStageFlags, DescriptorPoolResetFlags};
use crate::image_utils::DecodedTexture;
use std::collections::VecDeque;
use imgui::FontSource::DefaultFontData;
use crossbeam_channel::{Receiver, Sender};
use std::time::Duration;
use std::error::Error;
use std::num::Wrapping;
use itertools::max;
use renderer_shell_vulkan::cleanup::CombinedDropSink;
use crate::asset_storage::ResourceHandle;
use fnv::FnvHashMap;
use crate::pipeline::image::ImageAsset;

/// Represents an image that will replace another image
pub struct ImageResourceUpdate {
    pub asset_uuid: AssetUuid,
    pub resource_handle: ResourceHandle<ImageAsset>,

    pub image: ManuallyDrop<VkImage>,
}

/// Represents the current state of the sprite and the GPU resources associated with it
pub struct ImageResource {
    pub image: ManuallyDrop<VkImage>,
    pub image_view: vk::ImageView,
}

/// Keeps track of sprites/images and manages descriptor sets that allow shaders to bind to images
/// and use them
pub struct ImageResourceManager {
    device_context: VkDeviceContext,

    // The raw texture resources
    images_by_uuid: FnvHashMap<AssetUuid, ResourceHandle<ImageAsset>>,
    images: Vec<Option<ImageResource>>,
    drop_sink: CombinedDropSink,

    // For sending image updates in a thread-safe manner
    image_update_tx: Sender<ImageResourceUpdate>,
    image_update_rx: Receiver<ImageResourceUpdate>,
}

impl ImageResourceManager {
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> VkResult<Self> {
        let images_by_uuid = Default::default();
        let images = Vec::new();

        let (image_update_tx, image_update_rx) = crossbeam_channel::unbounded();

        let drop_sink = CombinedDropSink::new(max_frames_in_flight + 1);

        Ok(ImageResourceManager {
            device_context: device_context.clone(),
            images_by_uuid,
            images,
            drop_sink,
            image_update_tx,
            image_update_rx,
        })
    }

    pub fn image_by_handle(
        &self,
        resource_handle: ResourceHandle<ImageAsset>,
    ) -> Option<&ImageResource> {
        //TODO: Stale handle detection?
        self.images[resource_handle.index() as usize].as_ref()
    }

    pub fn image_handle_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<ResourceHandle<ImageAsset>> {
        self.images_by_uuid.get(asset_uuid).map(|x| *x)
    }

    pub fn image_by_uuid(
        &self,
        asset_uuid: &AssetUuid,
    ) -> Option<&ImageResource> {
        self.images_by_uuid
            .get(asset_uuid)
            .and_then(|handle| self.images[handle.index() as usize].as_ref())
    }

    pub fn image_update_tx(&self) -> &Sender<ImageResourceUpdate> {
        &self.image_update_tx
    }

    pub fn update(&mut self) {
        // This will handle any resources that need to be dropped
        self.drop_sink
            .on_frame_complete(self.device_context.device());

        // Check if we have any image updates to process
        self.apply_image_updates();
    }

    /// Checks if there are pending image updates, and if there are, regenerates the descriptor sets
    fn apply_image_updates(&mut self) {
        let mut updates = Vec::with_capacity(self.image_update_rx.len());
        while let Ok(update) = self.image_update_rx.recv_timeout(Duration::from_secs(0)) {
            updates.push(update);
        }

        if !updates.is_empty() {
            self.do_apply_image_updates(updates);
        }
    }

    /// Runs through the incoming image updates and applies them to the list of sprites
    fn do_apply_image_updates(
        &mut self,
        updates: Vec<ImageResourceUpdate>,
    ) {
        let mut max_index = self.images.len();
        for update in &updates {
            max_index = max_index.max(update.resource_handle.index() as usize + 1);
        }

        self.images.resize_with(max_index, || None);

        let mut old_images = vec![];
        for update in updates {
            let image = update.image;
            let resource_handle = update.resource_handle;
            let asset_uuid = update.asset_uuid;

            self.images_by_uuid
                .entry(asset_uuid)
                .or_insert(resource_handle);

            let image_view =
                Self::create_texture_image_view(self.device_context.device(), &image.image);

            // Do a swap so if there is an old sprite we can properly destroy it
            let mut image = Some(ImageResource { image, image_view });
            std::mem::swap(
                &mut image,
                &mut self.images[resource_handle.index() as usize],
            );
            if image.is_some() {
                old_images.push(image);
            }
        }

        // retire old images/views
        for image in old_images.drain(..) {
            let image = image.unwrap();
            self.drop_sink.retire_image(image.image);
            self.drop_sink.retire_image_view(image.image_view);
        }
    }

    pub fn create_texture_image_view(
        logical_device: &ash::Device,
        image: &vk::Image,
    ) -> vk::ImageView {
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let image_view_info = vk::ImageViewCreateInfo::builder()
            .image(*image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(*subresource_range);

        unsafe {
            logical_device
                .create_image_view(&image_view_info, None)
                .unwrap()
        }
    }
}

impl Drop for ImageResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying ImageResourceManager");

        for image in self.images.drain(..) {
            if let Some(image) = image {
                self.drop_sink.retire_image(image.image);
                self.drop_sink.retire_image_view(image.image_view);
            }
        }
        self.drop_sink.destroy(self.device_context.device());

        log::debug!("destroyed ImageResourceManager");
    }
}
