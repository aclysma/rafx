use renderer_features::renderpass::{VkOpaqueRenderPass, VkDebugRenderPass, VkBloomRenderPassResources, VkBloomExtractRenderPass, VkBloomBlurRenderPass, VkBloomCombineRenderPass};
use renderer_shell_vulkan::{VkDeviceContext, VkSwapchain};
use crate::game_renderer::GameRendererInner;
use renderer_assets::resource_managers::{ResourceManager, DynDescriptorSet};
use renderer_assets::pipeline_description::SwapchainSurfaceInfo;
use ash::prelude::VkResult;

pub struct SwapchainResources {
    pub debug_material_per_frame_data: DynDescriptorSet,
    pub bloom_resources: VkBloomRenderPassResources,
    pub bloom_extract_material_dyn_set: DynDescriptorSet,
    pub bloom_combine_material_dyn_set: DynDescriptorSet,

    pub opaque_renderpass: VkOpaqueRenderPass,
    pub debug_renderpass: VkDebugRenderPass,
    pub bloom_extract_renderpass: VkBloomExtractRenderPass,
    pub bloom_blur_renderpass: VkBloomBlurRenderPass,
    pub bloom_combine_renderpass: VkBloomCombineRenderPass,
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

        log::trace!("Create VkOpaqueRenderPass");
        //TODO: We probably want to move to just using a pipeline here and not a specific material
        let opaque_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.sprite_material,
            &swapchain_surface_info,
            0,
        );

        let opaque_renderpass = VkOpaqueRenderPass::new(
            device_context,
            swapchain,
            opaque_pipeline_info,
        )?;

        log::trace!("Create VkDebugRenderPass");
        let debug_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.debug_material,
            &swapchain_surface_info,
            0,
        );

        let debug_renderpass = VkDebugRenderPass::new(
            device_context,
            swapchain,
            debug_pipeline_info,
        )?;

        log::trace!("Create VkBloomExtractRenderPass");

        let bloom_resources = VkBloomRenderPassResources::new(
            device_context,
            swapchain,
            resource_manager,
            game_renderer.static_resources.bloom_blur_material.clone()
        )?;

        let bloom_extract_layout = resource_manager.get_descriptor_set_info(
            &game_renderer.static_resources.bloom_extract_material,
            0,
            0
        );

        let bloom_extract_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.bloom_extract_material,
            &swapchain_surface_info,
            0,
        );

        let bloom_extract_renderpass = VkBloomExtractRenderPass::new(
            device_context,
            swapchain,
            bloom_extract_pipeline_info,
            &bloom_resources
        )?;

        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        let mut bloom_extract_material_dyn_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&bloom_extract_layout.descriptor_set_layout)?;
        bloom_extract_material_dyn_set.set_image_raw(0, swapchain.color_attachment.resolved_image_view());
        bloom_extract_material_dyn_set.flush(&mut descriptor_set_allocator);

        log::trace!("Create VkBloomBlurRenderPass");

        let bloom_blur_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.bloom_blur_material,
            &swapchain_surface_info,
            0,
        );

        let bloom_blur_renderpass = VkBloomBlurRenderPass::new(
            device_context,
            swapchain,
            bloom_blur_pipeline_info,
            resource_manager,
            &bloom_resources
        )?;

        log::trace!("Create VkBloomCombineRenderPass");

        let bloom_combine_layout = resource_manager.get_descriptor_set_info(
            &game_renderer.static_resources.bloom_combine_material,
            0,
            0
        );

        let bloom_combine_pipeline_info = resource_manager.get_pipeline_info(
            &game_renderer.static_resources.bloom_combine_material,
            &swapchain_surface_info,
            0,
        );

        let bloom_combine_renderpass = VkBloomCombineRenderPass::new(
            device_context,
            swapchain,
            bloom_combine_pipeline_info,
            &bloom_resources
        )?;

        let mut bloom_combine_material_dyn_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&bloom_combine_layout.descriptor_set_layout)?;
        bloom_combine_material_dyn_set.set_image_raw(0, bloom_resources.color_image_view);
        bloom_combine_material_dyn_set.set_image_raw(1, bloom_resources.bloom_image_views[0]);
        bloom_combine_material_dyn_set.flush(&mut descriptor_set_allocator);

        let debug_per_frame_layout = resource_manager.get_descriptor_set_info(&game_renderer.static_resources.debug_material, 0, 0);
        let debug_material_per_frame_data = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&debug_per_frame_layout.descriptor_set_layout)?;

        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(SwapchainResources {
            debug_material_per_frame_data,
            bloom_resources,
            bloom_extract_material_dyn_set,
            bloom_combine_material_dyn_set,
            opaque_renderpass,
            debug_renderpass,
            bloom_extract_renderpass,
            bloom_blur_renderpass,
            bloom_combine_renderpass,
            swapchain_surface_info,
        })
    }
}
