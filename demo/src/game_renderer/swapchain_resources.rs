use crate::game_renderer::GameRendererInner;
use ash::prelude::VkResult;
use rafx::resources::vk_description as dsc;
use rafx::resources::vk_description::SwapchainSurfaceInfo;
use rafx::resources::{ImageViewResource, ResourceArc, ResourceManager};
use rafx::vulkan::VkImageRaw;
use rafx::vulkan::{SwapchainInfo, VkDeviceContext, VkSwapchain};

pub struct SwapchainResources {
    // The images presented by the swapchain
    //TODO: We don't properly support multiple swapchains right now. This would ideally be a map
    // of window/surface to info for the swapchain
    pub swapchain_images: Vec<ResourceArc<ImageViewResource>>,

    pub swapchain_info: SwapchainInfo,
    pub swapchain_surface_info: SwapchainSurfaceInfo,
}

impl SwapchainResources {
    pub fn new(
        _device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
        _game_renderer: &mut GameRendererInner,
        resource_manager: &mut ResourceManager,
        swapchain_info: SwapchainInfo,
        swapchain_surface_info: SwapchainSurfaceInfo,
    ) -> VkResult<SwapchainResources> {
        log::debug!("creating swapchain resources");

        //
        // Create resources for the swapchain images. This allows renderer systems to use them
        // interchangably with non-swapchain images
        //
        let image_view_meta = dsc::ImageViewMeta {
            view_type: dsc::ImageViewType::Type2D,
            subresource_range: dsc::ImageSubresourceRange {
                aspect_mask: dsc::ImageAspectFlag::Color.into(),
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            components: dsc::ComponentMapping::default(),
            format: swapchain.swapchain_info.surface_format.format.into(),
        };

        let mut swapchain_images = Vec::with_capacity(swapchain.swapchain_images.len());
        for &image in &swapchain.swapchain_images {
            let raw = VkImageRaw {
                allocation: None,
                image,
            };

            let image = resource_manager.resources().insert_raw_image(raw);
            let image_view = resource_manager
                .resources()
                .get_or_create_image_view(&image, &image_view_meta)?;

            swapchain_images.push(image_view);
        }

        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(SwapchainResources {
            swapchain_images,
            swapchain_info,
            swapchain_surface_info,
        })
    }
}
