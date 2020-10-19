use crate::renderpass::{
    VkOpaqueRenderPass, VkMsaaRenderPass, VkBloomRenderPassResources, VkBloomExtractRenderPass,
    VkBloomBlurRenderPass, VkBloomCombineRenderPass, VkUiRenderPass,
};
use renderer::vulkan::{VkDeviceContext, VkSwapchain};
use crate::game_renderer::{GameRendererInner, RenderpassAttachmentImage};
use renderer::assets::resources::{
    ResourceManager, DynDescriptorSet, ResourceArc, ImageViewResource, ResourceLookupSet,
    CommandPool,
};
use renderer::assets::vk_description::SwapchainSurfaceInfo;
use ash::prelude::VkResult;
use ash::vk;
use renderer::assets::vk_description as dsc;
use renderer::vulkan::VkImageRaw;

pub struct SwapchainResources {
    // The images presented by the swapchain
    //TODO: We don't properly support multiple swapchains right now. This would ideally be a map
    // of window/surface to info for the swapchain
    pub swapchain_images: Vec<ResourceArc<ImageViewResource>>,

    pub color_attachment: RenderpassAttachmentImage,
    pub depth_attachment: RenderpassAttachmentImage,

    pub static_command_pool: CommandPool,

    pub debug_material_per_frame_data: DynDescriptorSet,
    pub bloom_resources: VkBloomRenderPassResources,
    pub bloom_extract_material_dyn_set: DynDescriptorSet,
    pub bloom_combine_material_dyn_set: DynDescriptorSet,

    pub opaque_renderpass: VkOpaqueRenderPass,
    pub msaa_renderpass: VkMsaaRenderPass,
    pub bloom_extract_renderpass: VkBloomExtractRenderPass,
    pub bloom_blur_renderpass: VkBloomBlurRenderPass,
    pub bloom_combine_renderpass: VkBloomCombineRenderPass,
    pub ui_renderpass: VkUiRenderPass,

    pub swapchain_surface_info: SwapchainSurfaceInfo,
}

impl SwapchainResources {
    pub fn new(
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
        game_renderer: &mut GameRendererInner,
        resource_manager: &mut ResourceManager,
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

            let (image_key, resource) = resource_manager.resources_mut().insert_raw_image(raw);
            let image_view = resource_manager
                .resources_mut()
                .get_or_create_image_view(image_key, &image_view_meta)?;

            swapchain_images.push(image_view);
        }

        //
        // Create images/views we use as attachments
        //
        let color_attachment = RenderpassAttachmentImage::new(
            resource_manager.resources_mut(),
            device_context,
            &swapchain.swapchain_info,
            swapchain.color_format,
            vk::ImageAspectFlags::COLOR,
            // the msaa image won't actually be sampled, but it's being passed from the debug renderpass to the
            // composite renderpass with layout ShaderReadOnlyOptimal for the non-msaa case. If msaa is enabled
            // it will get resolved to the resolved image and we will sample that. If msaa is off, we don't even
            // create an msaa image
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_DST,
            swapchain_surface_info.msaa_level,
        )?;

        let depth_attachment = RenderpassAttachmentImage::new(
            resource_manager.resources_mut(),
            device_context,
            &swapchain.swapchain_info,
            swapchain.depth_format,
            vk::ImageAspectFlags::DEPTH,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            swapchain_surface_info.msaa_level,
        )?;

        let mut static_command_pool = CommandPool::new(
            &device_context,
            device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
        )?;

        log::debug!("Create VkOpaqueRenderPass");
        //TODO: We probably want to move to just using a pipeline here and not a specific material
        let opaque_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.sprite_material,
            &swapchain_surface_info,
            0,
        );

        let opaque_renderpass = VkOpaqueRenderPass::new(
            resource_manager.resources_mut(),
            device_context,
            &swapchain.swapchain_info,
            &swapchain_images,
            &color_attachment,
            &depth_attachment,
            opaque_pipeline_info,
        )?;

        log::debug!("Create VkDebugRenderPass");
        let msaa_renderpass =
            VkMsaaRenderPass::new(device_context, &swapchain.swapchain_info, &color_attachment)?;

        log::debug!("Create VkBloomExtractRenderPass");

        let bloom_resources = VkBloomRenderPassResources::new(
            device_context,
            swapchain,
            resource_manager,
            game_renderer.static_resources.bloom_blur_material.clone(),
        )?;

        let bloom_extract_layout = resource_manager.get_descriptor_set_info(
            &game_renderer.static_resources.bloom_extract_material,
            0,
            0,
        );

        let bloom_extract_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.bloom_extract_material,
            &swapchain_surface_info,
            0,
        );

        let bloom_extract_renderpass = VkBloomExtractRenderPass::new(
            resource_manager.resources_mut(),
            device_context,
            &swapchain.swapchain_info,
            &swapchain_images,
            bloom_extract_pipeline_info,
            &bloom_resources,
        )?;

        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        let mut bloom_extract_material_dyn_set = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(&bloom_extract_layout.descriptor_set_layout)?;
        bloom_extract_material_dyn_set.set_image_raw(0, color_attachment.resolved_image_view());
        bloom_extract_material_dyn_set.flush(&mut descriptor_set_allocator)?;

        log::debug!("Create VkBloomBlurRenderPass");

        let bloom_blur_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.bloom_blur_material,
            &swapchain_surface_info,
            0,
        );

        let bloom_blur_renderpass = VkBloomBlurRenderPass::new(
            resource_manager.resources_mut(),
            device_context,
            &swapchain.swapchain_info,
            bloom_blur_pipeline_info,
            &bloom_resources,
            &mut static_command_pool,
        )?;

        log::debug!("Create VkBloomCombineRenderPass");

        let bloom_combine_layout = resource_manager.get_descriptor_set_info(
            &game_renderer.static_resources.bloom_combine_material,
            0,
            0,
        );

        let bloom_combine_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.bloom_combine_material,
            &swapchain_surface_info,
            0,
        );

        let bloom_combine_renderpass = VkBloomCombineRenderPass::new(
            resource_manager.resources_mut(),
            device_context,
            &swapchain.swapchain_info,
            &swapchain_images,
            bloom_combine_pipeline_info,
        )?;

        let imgui_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.imgui_material,
            &swapchain_surface_info,
            0,
        );

        let ui_renderpass = VkUiRenderPass::new(
            resource_manager.resources_mut(),
            device_context,
            &swapchain.swapchain_info,
            &swapchain_images,
            imgui_pipeline_info,
        )?;

        let mut bloom_combine_material_dyn_set = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(&bloom_combine_layout.descriptor_set_layout)?;
        bloom_combine_material_dyn_set
            .set_image_raw(0, bloom_resources.color_image.get_raw().image_view);
        bloom_combine_material_dyn_set
            .set_image_raw(1, bloom_resources.bloom_images[0].get_raw().image_view);
        bloom_combine_material_dyn_set.flush(&mut descriptor_set_allocator)?;

        let debug_per_frame_layout = resource_manager.get_descriptor_set_info(
            &game_renderer.static_resources.debug3d_material,
            0,
            0,
        );
        let debug_material_per_frame_data = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(
                &debug_per_frame_layout.descriptor_set_layout,
            )?;

        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(SwapchainResources {
            swapchain_images,
            color_attachment,
            depth_attachment,
            static_command_pool,
            debug_material_per_frame_data,
            bloom_resources,
            bloom_extract_material_dyn_set,
            bloom_combine_material_dyn_set,
            opaque_renderpass,
            msaa_renderpass,
            bloom_extract_renderpass,
            bloom_blur_renderpass,
            bloom_combine_renderpass,
            ui_renderpass,
            swapchain_surface_info,
        })
    }
}
