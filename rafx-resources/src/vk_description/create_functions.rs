use super::types as dsc;
use crate::vk_description::{AttachmentIndex, SubpassInfo};
use ash::prelude::*;
use ash::version::DeviceV1_0;
use ash::vk;

#[profiling::function]
pub fn create_shader_module(
    device: &ash::Device,
    shader_module: &dsc::ShaderModule,
) -> VkResult<vk::ShaderModule> {
    let create_info = vk::ShaderModuleCreateInfo::builder().code(&shader_module.code);

    unsafe { device.create_shader_module(&*create_info, None) }
}

#[profiling::function]
pub fn create_descriptor_set_layout(
    device: &ash::Device,
    descriptor_set_layout: &dsc::DescriptorSetLayout,
    immutable_samplers: &[Option<Vec<vk::Sampler>>],
) -> VkResult<vk::DescriptorSetLayout> {
    let mut builders =
        Vec::with_capacity(descriptor_set_layout.descriptor_set_layout_bindings.len());

    for (binding, immutable_samplers) in descriptor_set_layout
        .descriptor_set_layout_bindings
        .iter()
        .zip(immutable_samplers)
    {
        let mut builder = vk::DescriptorSetLayoutBinding::builder()
            .binding(binding.binding)
            .descriptor_type(binding.descriptor_type.into())
            .descriptor_count(binding.descriptor_count)
            .stage_flags(binding.stage_flags.into());

        if let Some(immutable_samplers) = immutable_samplers {
            builder = builder.immutable_samplers(immutable_samplers);
        }

        builders.push(builder.build());
    }

    let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&builders);

    unsafe { device.create_descriptor_set_layout(&*create_info, None) }
}

#[profiling::function]
pub fn create_pipeline_layout(
    device: &ash::Device,
    pipeline_layout: &dsc::PipelineLayout,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> VkResult<vk::PipelineLayout> {
    let push_constant_ranges: Vec<_> = pipeline_layout
        .push_constant_ranges
        .iter()
        .map(|push_constant_range| push_constant_range.as_builder().build())
        .collect();

    let create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(descriptor_set_layouts)
        .push_constant_ranges(push_constant_ranges.as_slice());

    unsafe { device.create_pipeline_layout(&*create_info, None) }
}

#[profiling::function]
pub fn create_renderpass(
    device: &ash::Device,
    renderpass: &dsc::RenderPass,
    swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
) -> VkResult<vk::RenderPass> {
    let attachments: Vec<_> = renderpass
        .attachments
        .iter()
        .map(|attachment| attachment.as_builder(&swapchain_surface_info).build())
        .collect();

    // One vec per subpass
    let mut color_attachments: Vec<Vec<vk::AttachmentReference>> =
        Vec::with_capacity(renderpass.subpasses.len());
    let mut input_attachments: Vec<Vec<vk::AttachmentReference>> =
        Vec::with_capacity(renderpass.subpasses.len());
    let mut resolve_attachments: Vec<Vec<vk::AttachmentReference>> =
        Vec::with_capacity(renderpass.subpasses.len());

    // One element per subpass that has a depth stencil attachment specified
    let mut depth_stencil_attachments: Vec<vk::AttachmentReference> =
        Vec::with_capacity(renderpass.subpasses.len());

    let mut subpasses: Vec<_> = Vec::with_capacity(renderpass.subpasses.len());

    for subpass in &renderpass.subpasses {
        color_attachments.push(
            subpass
                .color_attachments
                .iter()
                .map(|attachment| attachment.as_builder().build())
                .collect(),
        );
        input_attachments.push(
            subpass
                .input_attachments
                .iter()
                .map(|attachment| attachment.as_builder().build())
                .collect(),
        );

        // The resolve attachment array must be unused or of length == color attachments. If
        // the number of subpass resolves doesn't match the color attachments, truncate or
        // insert attachment references with AttachmentIndex::Unused
        if subpass.resolve_attachments.len() > subpass.color_attachments.len() {
            log::warn!("A renderpass definition has more resolve attachments than color attachments. The additional resolve attachments will be discarded");
        }

        let mut subpass_resolve_attachments: Vec<_> = subpass
            .resolve_attachments
            .iter()
            .map(|attachment| attachment.as_builder().build())
            .collect();
        if !subpass_resolve_attachments.is_empty() {
            let unused_attachment = dsc::AttachmentReference {
                attachment: dsc::AttachmentIndex::Unused,
                layout: Default::default(),
            }
            .as_builder()
            .build();
            subpass_resolve_attachments.resize(color_attachments.len(), unused_attachment);
        }
        resolve_attachments.push(subpass_resolve_attachments);

        let mut subpass_description_builder = vk::SubpassDescription::builder()
            .pipeline_bind_point(subpass.pipeline_bind_point.into())
            .color_attachments(color_attachments.last().unwrap())
            .input_attachments(input_attachments.last().unwrap());

        // Only specify resolve attachments if we have more than zero of them
        {
            let subpass_resolve_attachments = resolve_attachments.last().unwrap();
            if !subpass_resolve_attachments.is_empty() {
                subpass_description_builder =
                    subpass_description_builder.resolve_attachments(subpass_resolve_attachments);
            }
        }

        // Only specify a depth stencil attachment if we have one
        if let Some(depth_stencil_attachment) = &subpass.depth_stencil_attachment {
            depth_stencil_attachments.push(depth_stencil_attachment.as_builder().build());
            subpass_description_builder = subpass_description_builder
                .depth_stencil_attachment(depth_stencil_attachments.last().unwrap());
        }

        let subpass_description = subpass_description_builder.build();

        subpasses.push(subpass_description);
    }

    let dependencies: Vec<_> = renderpass
        .dependencies
        .iter()
        .map(|dependency| dependency.as_builder().build())
        .collect();

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    unsafe { device.create_render_pass(&*create_info, None) }
}

#[profiling::function]
pub fn create_framebuffer(
    device: &ash::Device,
    renderpass: vk::RenderPass,
    attachments: &[vk::ImageView],
    framebuffer_meta: &dsc::FramebufferMeta,
) -> VkResult<vk::Framebuffer> {
    let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
        .render_pass(renderpass)
        .attachments(attachments)
        .width(framebuffer_meta.width)
        .height(framebuffer_meta.height)
        .layers(framebuffer_meta.layers);

    unsafe { device.create_framebuffer(&frame_buffer_create_info, None) }
}

#[profiling::function]
#[allow(clippy::too_many_arguments)]
pub fn create_graphics_pipelines(
    device: &ash::Device,
    vertex_input_state: &dsc::PipelineVertexInputState,
    fixed_function_state: &dsc::FixedFunctionState,
    pipeline_layout: vk::PipelineLayout,
    renderpass: vk::RenderPass,
    renderpass_dsc: &dsc::RenderPass,
    shader_modules_meta: &[dsc::ShaderModuleMeta],
    shader_modules: &[vk::ShaderModule],
    swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    framebuffer_meta: &dsc::FramebufferMeta,
) -> VkResult<Vec<vk::Pipeline>> {
    let input_assembly_state = fixed_function_state
        .input_assembly_state
        .as_builder()
        .build();

    let vertex_input_attribute_descriptions: Vec<_> = vertex_input_state
        .attribute_descriptions
        .iter()
        .map(|attribute| attribute.clone().into())
        .collect();

    let vertex_input_binding_descriptions: Vec<_> = vertex_input_state
        .binding_descriptions
        .iter()
        .map(|binding| binding.as_builder().build())
        .collect();

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_attribute_descriptions(vertex_input_attribute_descriptions.as_slice())
        .vertex_binding_descriptions(&vertex_input_binding_descriptions);

    let scissors: Vec<_> = fixed_function_state
        .viewport_state
        .scissors
        .iter()
        .map(|scissors| scissors.to_rect2d(swapchain_surface_info, framebuffer_meta))
        .collect();

    let viewports: Vec<_> = fixed_function_state
        .viewport_state
        .viewports
        .iter()
        .map(|viewport| {
            viewport
                .as_builder(swapchain_surface_info, framebuffer_meta)
                .build()
        })
        .collect();

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .scissors(&scissors)
        .viewports(&viewports);

    let rasterization_state = fixed_function_state.rasterization_state.as_builder();

    let color_blend_attachments: Vec<_> = fixed_function_state
        .color_blend_state
        .attachments
        .iter()
        .map(|attachment| attachment.as_builder().build())
        .collect();
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op(fixed_function_state.color_blend_state.logic_op.into())
        .logic_op_enable(fixed_function_state.color_blend_state.logic_op_enable)
        .blend_constants(
            fixed_function_state
                .color_blend_state
                .blend_constants_as_f32(),
        )
        .attachments(&color_blend_attachments);

    let dynamic_states: Vec<vk::DynamicState> = fixed_function_state
        .dynamic_state
        .dynamic_states
        .iter()
        .map(|dynamic_state| dynamic_state.clone().into())
        .collect();
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

    let depth_stencil_state = fixed_function_state.depth_stencil_state.as_builder();

    let mut stages = Vec::with_capacity(shader_modules_meta.len());
    let mut entry_names: Vec<std::ffi::CString> = Vec::with_capacity(shader_modules_meta.len());
    for (meta, module) in shader_modules_meta.iter().zip(shader_modules) {
        entry_names.push(std::ffi::CString::new(meta.entry_name.clone()).unwrap());
        stages.push(
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(meta.stage.into())
                .module(*module)
                .name(entry_names.last().unwrap())
                .build(),
        );
    }

    let mut multisample_states = Vec::with_capacity(renderpass_dsc.subpasses.len());
    for subpass in &renderpass_dsc.subpasses {
        let mut subpass_info = SubpassInfo {
            surface_info: swapchain_surface_info.clone(),
            subpass_sample_count_flags: vk::SampleCountFlags::empty(),
        };
        for attachment_reference in &subpass.color_attachments {
            if let AttachmentIndex::Index(index) = attachment_reference.attachment {
                let attachment = &renderpass_dsc.attachments[index as usize];
                subpass_info.subpass_sample_count_flags |= attachment
                    .samples
                    .as_vk_sample_count_flags(&swapchain_surface_info);
            }
        }

        if let Some(attachment_reference) = &subpass.depth_stencil_attachment {
            if let AttachmentIndex::Index(index) = attachment_reference.attachment {
                let attachment = &renderpass_dsc.attachments[index as usize];
                subpass_info.subpass_sample_count_flags |= attachment
                    .samples
                    .as_vk_sample_count_flags(&swapchain_surface_info);
            }
        }

        multisample_states.push(
            fixed_function_state
                .multisample_state
                .as_builder(&subpass_info)
                .build(),
        );
    }

    let pipeline_infos: Vec<_> = (0..renderpass_dsc.subpasses.len())
        .map(|subpass_index| {
            vk::GraphicsPipelineCreateInfo::builder()
                .input_assembly_state(&input_assembly_state)
                .vertex_input_state(&vertex_input_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_states[subpass_index])
                .color_blend_state(&color_blend_state)
                .dynamic_state(&dynamic_state)
                .depth_stencil_state(&depth_stencil_state)
                .layout(pipeline_layout)
                .render_pass(renderpass)
                .stages(&stages)
                .subpass(subpass_index as u32)
                .build()
        })
        .collect();

    unsafe {
        match device.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None) {
            Ok(result) => Ok(result),
            Err(e) => Err(e.1),
        }
    }
}

#[profiling::function]
pub fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    image_view_meta: &dsc::ImageViewMeta,
) -> VkResult<vk::ImageView> {
    unsafe {
        let create_info = image_view_meta.as_builder(image);

        device.create_image_view(&*create_info, None)
    }
}

#[profiling::function]
pub fn create_sampler(
    device: &ash::Device,
    sampler: &dsc::Sampler,
) -> VkResult<vk::Sampler> {
    unsafe {
        let create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(sampler.mag_filter.into())
            .min_filter(sampler.min_filter.into())
            .mipmap_mode(sampler.mipmap_mode.into())
            .address_mode_u(sampler.address_mode_u.into())
            .address_mode_v(sampler.address_mode_v.into())
            .address_mode_w(sampler.address_mode_w.into())
            .mip_lod_bias(sampler.mip_lod_bias.to_f32())
            .anisotropy_enable(sampler.anisotropy_enable)
            .max_anisotropy(sampler.max_anisotropy.to_f32())
            .compare_enable(sampler.compare_enable)
            .compare_op(sampler.compare_op.into())
            .min_lod(sampler.min_lod.to_f32())
            .max_lod(sampler.max_lod.to_f32())
            .border_color(sampler.border_color.into())
            .unnormalized_coordinates(sampler.unnormalized_coordinates);

        device.create_sampler(&*create_info, None)
    }
}
