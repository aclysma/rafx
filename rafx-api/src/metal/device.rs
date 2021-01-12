use crate::{RafxApiDef, RafxQueueType, RafxResult};
use raw_window_handle::HasRawWindowHandle;

use super::{
    RafxBufferMetal, RafxGraphicsPipelineMetal, RafxRenderpassMetal, RafxShaderModuleMetal,
    RafxSurfaceMetal, RenderPipelineDescriptorDef, RenderpassDef,
};
use crate::metal::RafxQueueMetal;

/// Metal-specific configuration
#[derive(Default)]
pub struct RafxApiDefMetal {
    // Currently no metal-specific configuration
}

#[derive(Clone)]
pub struct RafxDeviceMetal {
    device: metal::Device,
}

impl RafxDeviceMetal {
    pub fn new(
        _window: &dyn HasRawWindowHandle,
        _api_def: &RafxApiDef,
        _metal_api_def: &RafxApiDefMetal,
    ) -> RafxResult<Self> {
        let device = metal::Device::system_default().expect("no device found");

        Ok(RafxDeviceMetal { device })
    }

    pub fn device(&self) -> &metal::Device {
        &self.device
    }

    pub fn create_queue(
        &self,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueMetal> {
        RafxQueueMetal::new(self.device(), queue_type)
    }

    // pub fn create_command_pool(
    //     &self,
    //     command_pool_def: &RafxCommandPoolDef,
    // ) -> RafxResult<RafxCommandPoolMetal> {
    //     Ok(RafxCommandPoolMetal::new(command_pool_def))
    // }

    // pub fn create_command_buffer(
    //     &self,
    //     command_pool: &mut RafxCommandPoolMetal,
    //     command_buffer_def: &RafxCommandBufferDef,
    // ) -> RafxResult<RafxCommandBufferMetal> {
    //     Ok(RafxCommandBufferMetal::new(
    //         command_pool.command_queue(),
    //         command_buffer_def,
    //     ))
    // }

    pub fn create_surface(
        &self,
        raw_window_handle: &dyn HasRawWindowHandle,
        width: u32,
        height: u32,
    ) -> RafxResult<RafxSurfaceMetal> {
        RafxSurfaceMetal::new(self.device(), raw_window_handle, width, height)
    }

    pub fn create_shader_module_from_source(
        &self,
        source: &str,
        compile_options: &metal::CompileOptions,
    ) -> RafxResult<RafxShaderModuleMetal> {
        RafxShaderModuleMetal::new_from_source(self.device(), source, compile_options)
    }

    pub fn create_shader_module_from_library_file<P: AsRef<std::path::Path>>(
        &self,
        file: P,
    ) -> RafxResult<RafxShaderModuleMetal> {
        RafxShaderModuleMetal::new_from_library_file(self.device(), file)
    }

    pub fn create_graphics_pipeline<'a>(
        &self,
        shader_module: &RafxShaderModuleMetal,
        def: &RenderPipelineDescriptorDef,
    ) -> RafxResult<RafxGraphicsPipelineMetal> {
        RafxGraphicsPipelineMetal::new(self.device(), shader_module, def)
    }

    pub fn create_buffer(
        &self,
        size_in_bytes: u64,
    ) -> RafxResult<RafxBufferMetal> {
        Ok(RafxBufferMetal::new(&self.device, size_in_bytes))
    }

    pub fn create_buffer_with_data<T: Copy>(
        &self,
        data: &[T],
    ) -> RafxResult<RafxBufferMetal> {
        let data_size_in_bytes = super::slice_size_in_bytes(data) as u64;
        let buffer = self.create_buffer(data_size_in_bytes)?;
        buffer.copy_to_buffer(data);
        Ok(buffer)
    }

    pub fn create_renderpass(
        &self,
        def: RenderpassDef,
    ) -> RafxRenderpassMetal {
        RafxRenderpassMetal::new(def)
    }

    // pub fn create_command_buffer(&self) -> RafxCommandBufferMetal {
    //     RafxCommandBufferMetal::new(self.command_queue())
    // }

    // pub fn create_render_command_encoder(
    //     &self,
    //     command_buffer: &RafxCommandBufferMetal,
    //     renderpass: &RafxRenderpassMetal,
    //     attachments: &[&RafxTextureMetal]
    // ) -> RafxRenderCommandEncoderMetal {
    //     RafxRenderCommandEncoderMetal::new(command_buffer, renderpass, attachments)
    // }
}
