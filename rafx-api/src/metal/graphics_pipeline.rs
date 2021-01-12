use super::{RafxShaderModuleMetal, RenderPipelineDescriptorDef};
use crate::RafxResult;
use cocoa_foundation::foundation::NSUInteger;

pub struct RafxGraphicsPipelineMetal {
    render_pipeline_state: metal::RenderPipelineState,
}

impl RafxGraphicsPipelineMetal {
    pub fn new(
        device: &metal::Device,
        shader_module: &RafxShaderModuleMetal,
        def: &RenderPipelineDescriptorDef,
    ) -> RafxResult<RafxGraphicsPipelineMetal> {
        let library = shader_module.library();
        let pipeline_state_descriptor = metal::RenderPipelineDescriptor::new();
        if let Some(shader) = &def.vertex_shader {
            let shader_fn = library.get_function(shader, None)?;
            pipeline_state_descriptor.set_vertex_function(Some(&shader_fn));
        }

        if let Some(shader) = &def.fragment_shader {
            let shader_fn = library.get_function(shader, None)?;
            pipeline_state_descriptor.set_fragment_function(Some(&shader_fn));
        }

        for (index, color_attachment) in def.color_attachments.iter().enumerate() {
            let attachment = pipeline_state_descriptor
                .color_attachments()
                .object_at(index as NSUInteger)
                .unwrap();
            attachment.set_pixel_format(color_attachment.pixel_format);

            attachment.set_blending_enabled(color_attachment.blending_enabled);
            attachment.set_rgb_blend_operation(color_attachment.rgb_blend_operation);
            attachment.set_alpha_blend_operation(color_attachment.alpha_blend_operation);
            attachment.set_source_rgb_blend_factor(color_attachment.source_rgb_blend_factor);
            attachment.set_source_alpha_blend_factor(color_attachment.source_alpha_blend_factor);
            attachment
                .set_destination_rgb_blend_factor(color_attachment.destination_rgb_blend_factor);
            attachment.set_destination_alpha_blend_factor(
                color_attachment.destination_alpha_blend_factor,
            );
        }

        let render_pipeline_state = device.new_render_pipeline_state(&pipeline_state_descriptor)?;

        Ok(RafxGraphicsPipelineMetal {
            render_pipeline_state,
        })
    }

    pub fn render_pipeline_state(&self) -> &metal::RenderPipelineState {
        &self.render_pipeline_state
    }
}
