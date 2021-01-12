fn slice_size_in_bytes<T>(slice: &[T]) -> usize {
    let range = slice.as_ptr_range();
    (range.end as *const u8 as usize) - (range.start as *const u8 as usize)
}

pub struct RenderPipelineColorAttachmentDescriptorDef {
    pub pixel_format: metal::MTLPixelFormat,
    pub blending_enabled: bool,
    pub rgb_blend_operation: metal::MTLBlendOperation,
    pub alpha_blend_operation: metal::MTLBlendOperation,
    pub source_rgb_blend_factor: metal::MTLBlendFactor,
    pub source_alpha_blend_factor: metal::MTLBlendFactor,
    pub destination_rgb_blend_factor: metal::MTLBlendFactor,
    pub destination_alpha_blend_factor: metal::MTLBlendFactor,
}

pub struct RenderPipelineDescriptorDef {
    pub vertex_shader: Option<String>,
    pub fragment_shader: Option<String>,
    pub color_attachments: Vec<RenderPipelineColorAttachmentDescriptorDef>,
}

pub struct RenderpassColorAttachmentDef {
    pub attachment_index: usize,
    pub load_action: metal::MTLLoadAction,
    pub clear_color: metal::MTLClearColor,
    pub store_action: metal::MTLStoreAction,
}

pub struct RenderpassDef {
    pub color_attachments: Vec<RenderpassColorAttachmentDef>,
}

mod device;
pub use device::*;

mod queue;
pub use queue::*;

mod surface;
pub use surface::*;

mod presentable_frame;
pub use presentable_frame::*;

mod shader_module;
pub use shader_module::*;

mod graphics_pipeline;
pub use graphics_pipeline::*;

mod buffer;
pub use buffer::*;

// mod command_pool;
// pub use command_pool::*;

mod command_buffer;
pub use command_buffer::*;

mod render_command_encoder;
pub use render_command_encoder::*;

mod renderpass;
pub use renderpass::*;

mod texture;
pub use texture::*;

pub struct RafxFenceMetal;
pub struct RafxSemaphoreMetal;
pub struct RafxSwapchainMetal;
#[derive(Clone, Debug)]
pub struct RafxRenderTargetMetal;
#[derive(Clone, Debug)]
pub struct RafxShaderMetal;
#[derive(Clone, Debug)]
pub struct RafxRootSignatureMetal;
#[derive(Debug)]
pub struct RafxPipelineMetal;
#[derive(Debug, Clone)]
pub struct RafxSamplerMetal;
#[derive(Clone, Debug)]
pub struct RafxDescriptorSetArrayMetal;
#[derive(Clone, Debug)]
pub struct RafxDescriptorSetHandleMetal;
pub struct RafxApiMetal;
#[derive(Clone)]
pub struct RafxDeviceContextMetal;
pub struct RafxCommandPoolMetal;
