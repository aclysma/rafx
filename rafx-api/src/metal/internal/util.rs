use crate::{
    RafxAddressMode, RafxBlendState, RafxBlendStateTargets, RafxColorRenderTargetBinding,
    RafxDepthState, RafxDeviceInfo, RafxResourceType, RafxStoreOp, MAX_RENDER_TARGET_ATTACHMENTS,
};
use cocoa_foundation::foundation::NSUInteger;
use metal_rs::{
    MTLArgumentAccess, MTLCompareFunction, MTLDataType, MTLResourceUsage, MTLSamplerAddressMode,
    MTLStoreAction, RenderPipelineColorAttachmentDescriptorArrayRef,
};

pub fn vertex_buffer_adjusted_buffer_index(binding: u32) -> NSUInteger {
    // Argument buffers will be 0-4
    // vertex buffers will be 30 - n
    (30 - binding) as _
}

pub(crate) fn resource_type_mtl_data_type(resource_type: RafxResourceType) -> Option<MTLDataType> {
    if resource_type.intersects(
        RafxResourceType::UNIFORM_BUFFER
            | RafxResourceType::BUFFER
            | RafxResourceType::BUFFER_READ_WRITE,
    ) {
        Some(MTLDataType::Pointer)
    } else if resource_type
        .intersects(RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE)
    {
        Some(MTLDataType::Texture)
    } else if resource_type.intersects(RafxResourceType::SAMPLER) {
        Some(MTLDataType::Sampler)
    } else {
        None
    }
}

pub(crate) fn resource_type_mtl_resource_usage(
    resource_type: RafxResourceType
) -> MTLResourceUsage {
    let mut usage = MTLResourceUsage::empty();

    if resource_type.intersects(RafxResourceType::TEXTURE) {
        usage |= MTLResourceUsage::Sample;
    }

    if resource_type.intersects(RafxResourceType::TEXTURE_READ_WRITE) {
        usage |= MTLResourceUsage::Read | MTLResourceUsage::Write;
    }

    if resource_type.intersects(RafxResourceType::UNIFORM_BUFFER) {
        usage |= MTLResourceUsage::Read;
    }

    if resource_type.intersects(RafxResourceType::BUFFER) {
        usage |= MTLResourceUsage::Read;
    }

    if resource_type.intersects(RafxResourceType::BUFFER_READ_WRITE) {
        usage |= MTLResourceUsage::Read | MTLResourceUsage::Write;
    }

    if resource_type
        .intersects(RafxResourceType::TEXEL_BUFFER | RafxResourceType::TEXEL_BUFFER_READ_WRITE)
    {
        usage |= MTLResourceUsage::Sample;
    }

    usage
}

pub(crate) fn resource_type_mtl_argument_access(
    resource_type: RafxResourceType
) -> MTLArgumentAccess {
    let usage = resource_type_mtl_resource_usage(resource_type);
    if usage.intersects(MTLResourceUsage::Write) {
        MTLArgumentAccess::ReadWrite
    } else {
        MTLArgumentAccess::ReadOnly
    }
}

pub(crate) fn address_mode_mtl_sampler_address_mode(
    address_mode: RafxAddressMode,
    device_info: &RafxDeviceInfo,
) -> MTLSamplerAddressMode {
    match address_mode {
        RafxAddressMode::Mirror => MTLSamplerAddressMode::MirrorRepeat,
        RafxAddressMode::Repeat => MTLSamplerAddressMode::Repeat,
        RafxAddressMode::ClampToEdge => MTLSamplerAddressMode::ClampToEdge,
        RafxAddressMode::ClampToBorder => {
            if device_info.supports_clamp_to_border_color {
                MTLSamplerAddressMode::ClampToBorderColor
            } else {
                MTLSamplerAddressMode::ClampToZero
            }
        }
    }
}

pub(crate) fn blend_def_to_attachment(
    blend_state: &RafxBlendState,
    attachments: &RenderPipelineColorAttachmentDescriptorArrayRef,
    color_attachment_count: usize,
) {
    blend_state.verify(color_attachment_count);

    if !blend_state.render_target_blend_states.is_empty() {
        for attachment_index in 0..MAX_RENDER_TARGET_ATTACHMENTS {
            if blend_state
                .render_target_mask
                .intersects(RafxBlendStateTargets::from_bits(1 << attachment_index).unwrap())
            {
                // Blend state can either be specified per target or once for all
                let def_index = if blend_state.independent_blend {
                    attachment_index
                } else {
                    0
                };

                let descriptor = attachments.object_at(attachment_index as _).unwrap();
                let def = &blend_state.render_target_blend_states[def_index];
                descriptor.set_blending_enabled(def.blend_enabled());
                descriptor.set_rgb_blend_operation(def.blend_op.into());
                descriptor.set_alpha_blend_operation(def.blend_op_alpha.into());
                descriptor.set_source_rgb_blend_factor(def.src_factor.into());
                descriptor.set_source_alpha_blend_factor(def.src_factor_alpha.into());
                descriptor.set_destination_rgb_blend_factor(def.dst_factor.into());
                descriptor.set_destination_alpha_blend_factor(def.dst_factor_alpha.into());
            };
        }
    }
}

pub(crate) fn depth_state_to_descriptor(
    depth_state: &RafxDepthState
) -> metal_rs::DepthStencilDescriptor {
    let descriptor = metal_rs::DepthStencilDescriptor::new();

    let depth_compare_function = if depth_state.depth_test_enable {
        depth_state.depth_compare_op.into()
    } else {
        MTLCompareFunction::Always
    };

    descriptor.set_depth_compare_function(depth_compare_function);
    descriptor.set_depth_write_enabled(depth_state.depth_write_enable);

    let back_face_stencil = descriptor.back_face_stencil().unwrap();
    let stencil_compare_function = if depth_state.stencil_test_enable {
        depth_state.back_stencil_compare_op.into()
    } else {
        MTLCompareFunction::Always
    };
    back_face_stencil.set_stencil_compare_function(stencil_compare_function);
    back_face_stencil.set_depth_failure_operation(depth_state.back_depth_fail_op.into());
    back_face_stencil.set_stencil_failure_operation(depth_state.back_stencil_fail_op.into());
    back_face_stencil.set_depth_stencil_pass_operation(depth_state.back_stencil_pass_op.into());
    back_face_stencil.set_read_mask(depth_state.stencil_read_mask as u32);
    back_face_stencil.set_write_mask(depth_state.stencil_write_mask as u32);

    let front_face_stencil = descriptor.front_face_stencil().unwrap();
    let stencil_compare_function = if depth_state.stencil_test_enable {
        depth_state.front_stencil_compare_op.into()
    } else {
        MTLCompareFunction::Always
    };
    front_face_stencil.set_stencil_compare_function(stencil_compare_function);
    front_face_stencil.set_depth_failure_operation(depth_state.front_depth_fail_op.into());
    front_face_stencil.set_stencil_failure_operation(depth_state.front_stencil_fail_op.into());
    front_face_stencil.set_depth_stencil_pass_operation(depth_state.front_stencil_pass_op.into());
    front_face_stencil.set_read_mask(depth_state.stencil_read_mask as u32);
    front_face_stencil.set_write_mask(depth_state.stencil_write_mask as u32);

    descriptor
}

pub fn color_render_target_binding_mtl_store_op(
    color_binding: &RafxColorRenderTargetBinding
) -> MTLStoreAction {
    let resolve = color_binding.resolve_target.is_some()
        && color_binding.resolve_store_op == RafxStoreOp::Store;
    if color_binding.store_op == RafxStoreOp::Store {
        if resolve {
            MTLStoreAction::StoreAndMultisampleResolve
        } else {
            MTLStoreAction::Store
        }
    } else {
        if resolve {
            MTLStoreAction::MultisampleResolve
        } else {
            MTLStoreAction::DontCare
        }
    }
}
