use crate::metal::{RafxRenderCommandEncoderMetal, RafxRenderpassMetal, RafxTextureMetal};
use crate::{RafxCommandBufferDef, RafxResult};

#[derive(Debug)]
pub struct RafxCommandBufferMetal {
    //command_queue: metal::CommandQueue,
    command_buffer: Option<metal::CommandBuffer>, //TODO: Encoders
}

impl RafxCommandBufferMetal {
    pub fn new(
        _command_queue: &metal::CommandQueue,
        _command_buffer_def: &RafxCommandBufferDef,
    ) -> Self {
        //let command_buffer = command_queue.new_command_buffer_with_unretained_references().to_owned();
        RafxCommandBufferMetal {
            //command_queue: command_queue.clone(),
            command_buffer: None,
        }
    }

    pub fn begin(&self) -> RafxResult<()> {
        //TODO: new_command_buffer_with_unretained_references?
        //self.command_buffer = Some(self.command_queue.new_command_buffer().to_owned());
        Ok(())
    }

    pub fn end(&self) -> RafxResult<()> {
        //TODO: Do something with it?
        //self.command_buffer = None;
        Ok(())
    }

    pub fn begin_renderpass(
        &self,
        renderpass: &RafxRenderpassMetal,
        attachments: &[&RafxTextureMetal],
    ) -> RafxRenderCommandEncoderMetal {
        RafxRenderCommandEncoderMetal::new(
            self.command_buffer.as_ref().unwrap(),
            &renderpass.def,
            attachments,
        )
    }
}
