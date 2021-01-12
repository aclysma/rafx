use crate::metal::{RafxTextureMetal, RenderpassDef};

pub struct RafxRenderCommandEncoderMetal {
    encoder: metal::RenderCommandEncoder,
}

impl RafxRenderCommandEncoderMetal {
    pub fn new(
        command_buffer: &metal::CommandBuffer,
        renderpass_def: &RenderpassDef,
        attachments: &[&RafxTextureMetal],
    ) -> Self {
        let render_pass_descriptor = metal::RenderPassDescriptor::new();
        for (color_attachment_index, color_attachment_def) in
            renderpass_def.color_attachments.iter().enumerate()
        {
            let color_attachment = render_pass_descriptor
                .color_attachments()
                .object_at(color_attachment_index as u64)
                .unwrap();

            color_attachment.set_texture(Some(
                attachments[color_attachment_def.attachment_index].texture(),
            ));
            color_attachment.set_load_action(color_attachment_def.load_action);
            color_attachment.set_clear_color(color_attachment_def.clear_color);
            color_attachment.set_store_action(color_attachment_def.store_action);
        }

        // let color_attachment = render_pass_descriptor.color_attachments().object_at(0).unwrap();
        // color_attachment.set_texture(Some(drawable.texture()));
        // color_attachment.set_load_action(metal::MTLLoadAction::Clear);
        // color_attachment.set_clear_color(metal::MTLClearColor::new(0.5, 0.2, 0.2, 1.0));
        // color_attachment.set_store_action(metal::MTLStoreAction::Store);

        let encoder = command_buffer
            .new_render_command_encoder(render_pass_descriptor)
            .to_owned();
        RafxRenderCommandEncoderMetal { encoder }
    }

    pub fn encoder(&self) -> &metal::RenderCommandEncoder {
        &self.encoder
    }
}
