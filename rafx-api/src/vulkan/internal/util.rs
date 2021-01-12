use crate::vulkan::RafxDeviceContextVulkan;
use crate::{RafxFormat, RafxPipelineType, RafxQueueType, RafxResourceState, RafxResourceType};
use ash::vk;

pub(crate) fn pipeline_type_pipeline_bind_point(
    pipeline_type: RafxPipelineType
) -> vk::PipelineBindPoint {
    match pipeline_type {
        RafxPipelineType::Graphics => vk::PipelineBindPoint::GRAPHICS,
        RafxPipelineType::Compute => vk::PipelineBindPoint::COMPUTE,
    }
}

pub(crate) fn resource_type_buffer_usage_flags(
    resource_type: RafxResourceType,
    has_format: bool,
) -> vk::BufferUsageFlags {
    let mut usage_flags = vk::BufferUsageFlags::TRANSFER_SRC;

    if resource_type.intersects(RafxResourceType::UNIFORM_BUFFER) {
        usage_flags |= vk::BufferUsageFlags::UNIFORM_BUFFER;
    }

    if resource_type.intersects(RafxResourceType::BUFFER_READ_WRITE) {
        usage_flags |= vk::BufferUsageFlags::STORAGE_BUFFER;
        if has_format {
            usage_flags |= vk::BufferUsageFlags::STORAGE_TEXEL_BUFFER;
        }
    }

    if resource_type.intersects(RafxResourceType::BUFFER) {
        usage_flags |= vk::BufferUsageFlags::STORAGE_BUFFER;
        if has_format {
            usage_flags |= vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER;
        }
    }

    if resource_type.intersects(RafxResourceType::INDEX_BUFFER) {
        usage_flags |= vk::BufferUsageFlags::INDEX_BUFFER;
    }

    if resource_type.intersects(RafxResourceType::VERTEX_BUFFER) {
        usage_flags |= vk::BufferUsageFlags::VERTEX_BUFFER;
    }

    if resource_type.intersects(RafxResourceType::INDIRECT_BUFFER) {
        usage_flags |= vk::BufferUsageFlags::INDIRECT_BUFFER;
    }

    usage_flags
}

pub(crate) fn resource_type_image_usage_flags(
    resource_type: RafxResourceType
) -> vk::ImageUsageFlags {
    let mut usage_flags = vk::ImageUsageFlags::empty();

    if resource_type.intersects(RafxResourceType::TEXTURE) {
        usage_flags |= vk::ImageUsageFlags::SAMPLED;
    }

    if resource_type.intersects(RafxResourceType::TEXTURE_READ_WRITE) {
        usage_flags |= vk::ImageUsageFlags::STORAGE;
    }

    usage_flags
}

pub(crate) fn image_format_to_aspect_mask(
    format: RafxFormat /*, include_stencil: bool*/
) -> vk::ImageAspectFlags {
    match format {
        // Depth only
        RafxFormat::D16_UNORM | RafxFormat::X8_D24_UNORM_PACK32 | RafxFormat::D32_SFLOAT => {
            vk::ImageAspectFlags::DEPTH
        }
        // Stencil only
        RafxFormat::S8_UINT => vk::ImageAspectFlags::STENCIL,
        // Both
        RafxFormat::D16_UNORM_S8_UINT
        | RafxFormat::D24_UNORM_S8_UINT
        | RafxFormat::D32_SFLOAT_S8_UINT => {
            //if include_stencil {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            //} else {
            //    vk::ImageAspectFlags::DEPTH
            //}
        }
        // Otherwise assume color
        _ => vk::ImageAspectFlags::COLOR,
    }
}

pub fn resource_type_to_descriptor_type(
    resource_type: RafxResourceType
) -> Option<vk::DescriptorType> {
    match resource_type {
        RafxResourceType::SAMPLER => Some(vk::DescriptorType::SAMPLER),
        RafxResourceType::TEXTURE => Some(vk::DescriptorType::SAMPLED_IMAGE),
        RafxResourceType::UNIFORM_BUFFER => Some(vk::DescriptorType::UNIFORM_BUFFER),
        RafxResourceType::TEXTURE_READ_WRITE => Some(vk::DescriptorType::STORAGE_IMAGE),
        RafxResourceType::BUFFER => Some(vk::DescriptorType::STORAGE_BUFFER),
        RafxResourceType::BUFFER_READ_WRITE => Some(vk::DescriptorType::STORAGE_BUFFER),
        RafxResourceType::INPUT_ATTACHMENT => Some(vk::DescriptorType::INPUT_ATTACHMENT),
        RafxResourceType::TEXEL_BUFFER => Some(vk::DescriptorType::UNIFORM_TEXEL_BUFFER),
        RafxResourceType::TEXEL_BUFFER_READ_WRITE => Some(vk::DescriptorType::STORAGE_TEXEL_BUFFER),
        RafxResourceType::COMBINED_IMAGE_SAMPLER => {
            Some(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        }
        _ => None,
    }
}

pub(crate) fn resource_state_to_access_flags(state: RafxResourceState) -> vk::AccessFlags {
    let mut flags = vk::AccessFlags::empty();
    if state.intersects(RafxResourceState::COPY_SRC) {
        flags |= vk::AccessFlags::TRANSFER_READ;
    }

    if state.intersects(RafxResourceState::COPY_DST) {
        flags |= vk::AccessFlags::TRANSFER_WRITE;
    }

    if state.intersects(RafxResourceState::VERTEX_AND_CONSTANT_BUFFER) {
        flags |= vk::AccessFlags::UNIFORM_READ | vk::AccessFlags::VERTEX_ATTRIBUTE_READ;
    }

    if state.intersects(RafxResourceState::INDEX_BUFFER) {
        flags |= vk::AccessFlags::INDEX_READ;
    }

    if state.intersects(RafxResourceState::UNORDERED_ACCESS) {
        flags |= vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE;
    }

    if state.intersects(RafxResourceState::INDIRECT_ARGUMENT) {
        flags |= vk::AccessFlags::INDIRECT_COMMAND_READ;
    }

    if state.intersects(RafxResourceState::RENDER_TARGET) {
        flags |= vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
    }

    if state.intersects(RafxResourceState::DEPTH_WRITE) {
        flags |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
    }

    if state.intersects(RafxResourceState::SHADER_RESOURCE) {
        flags |= vk::AccessFlags::SHADER_READ;
    }

    if state.intersects(RafxResourceState::PRESENT) {
        flags |= vk::AccessFlags::MEMORY_READ;
    }

    flags
}

pub(crate) fn resource_state_to_image_layout(state: RafxResourceState) -> Option<vk::ImageLayout> {
    if state.intersects(RafxResourceState::COPY_SRC) {
        Some(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
    } else if state.intersects(RafxResourceState::COPY_DST) {
        Some(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
    } else if state.intersects(RafxResourceState::RENDER_TARGET) {
        Some(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    } else if state.intersects(RafxResourceState::DEPTH_WRITE) {
        Some(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
    } else if state.intersects(RafxResourceState::UNORDERED_ACCESS) {
        Some(vk::ImageLayout::GENERAL)
    } else if state.intersects(RafxResourceState::SHADER_RESOURCE) {
        Some(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
    } else if state.intersects(RafxResourceState::PRESENT) {
        Some(vk::ImageLayout::PRESENT_SRC_KHR)
    } else if state.intersects(RafxResourceState::COMMON) {
        Some(vk::ImageLayout::GENERAL)
    } else if state == RafxResourceState::UNDEFINED {
        Some(vk::ImageLayout::UNDEFINED)
    } else {
        None
    }
}

pub(crate) fn queue_type_to_family_index(
    device_context: &RafxDeviceContextVulkan,
    queue_type: RafxQueueType,
) -> u32 {
    match queue_type {
        RafxQueueType::Graphics => {
            device_context
                .queue_family_indices()
                .graphics_queue_family_index
        }
        RafxQueueType::Compute => {
            device_context
                .queue_family_indices()
                .compute_queue_family_index
        }
        RafxQueueType::Transfer => {
            device_context
                .queue_family_indices()
                .transfer_queue_family_index
        }
    }
}

// Based on what is being accessed, determine what stages need to be blocked
pub(crate) fn determine_pipeline_stage_flags(
    queue_type: RafxQueueType,
    access_flags: vk::AccessFlags,
) -> vk::PipelineStageFlags {
    let mut flags = vk::PipelineStageFlags::empty();
    match queue_type {
        RafxQueueType::Graphics => {
            if access_flags
                .intersects(vk::AccessFlags::INDEX_READ | vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
            {
                flags |= vk::PipelineStageFlags::VERTEX_INPUT;
            }

            if access_flags.intersects(
                vk::AccessFlags::UNIFORM_READ
                    | vk::AccessFlags::SHADER_READ
                    | vk::AccessFlags::SHADER_WRITE,
            ) {
                flags |= vk::PipelineStageFlags::VERTEX_INPUT;
                flags |= vk::PipelineStageFlags::FRAGMENT_SHADER;
                flags |= vk::PipelineStageFlags::COMPUTE_SHADER;

                // Not supported
                //flags |= vk::PipelineStageFlags::GEOMETRY_SHADER;
                //flags |= vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER;
                //flags |= vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER;
                // raytracing
            }

            if access_flags.intersects(vk::AccessFlags::INPUT_ATTACHMENT_READ) {
                flags |= vk::PipelineStageFlags::FRAGMENT_SHADER;
            }

            if access_flags.intersects(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ) {
                flags |= vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
            }

            if access_flags.intersects(
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ) {
                flags |= vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
            }
        }
        RafxQueueType::Compute => {
            if access_flags.intersects(
                vk::AccessFlags::INDEX_READ
                    | vk::AccessFlags::VERTEX_ATTRIBUTE_READ
                    | vk::AccessFlags::INPUT_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ) {
                return vk::PipelineStageFlags::ALL_COMMANDS;
            }

            if access_flags.intersects(
                vk::AccessFlags::UNIFORM_READ
                    | vk::AccessFlags::SHADER_READ
                    | vk::AccessFlags::SHADER_WRITE,
            ) {
                flags |= vk::PipelineStageFlags::COMPUTE_SHADER;
            }
        }
        RafxQueueType::Transfer => {
            return vk::PipelineStageFlags::ALL_COMMANDS;
        }
    }

    //
    // Logic for both graphics/compute
    //
    if access_flags.intersects(vk::AccessFlags::INDIRECT_COMMAND_READ) {
        flags |= vk::PipelineStageFlags::DRAW_INDIRECT;
    }

    if access_flags.intersects(vk::AccessFlags::TRANSFER_READ | vk::AccessFlags::TRANSFER_WRITE) {
        flags |= vk::PipelineStageFlags::TRANSFER;
    }

    if access_flags.intersects(vk::AccessFlags::HOST_READ | vk::AccessFlags::HOST_WRITE) {
        flags |= vk::PipelineStageFlags::HOST;
    }

    if flags.is_empty() {
        flags |= vk::PipelineStageFlags::TOP_OF_PIPE;
    }

    flags
}
