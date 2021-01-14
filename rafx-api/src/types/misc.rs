use ash::vk;

use serde::{Deserialize, Serialize};

use crate::{RafxBuffer, RafxRenderTarget, RafxSampler, RafxTexture};
use rafx_base::DecimalF32;
use std::hash::{Hash, Hasher};

/// Controls if validation is enabled or not. The requirements/behaviors of validation is
/// API-specific.
#[derive(Copy, Clone)]
pub enum RafxValidationMode {
    /// Do not enable validation. Even if validation is turned on through external means, do not
    /// intentionally fail initialization
    Disabled,

    /// Enable validation if possible. (Details on requirements to enable at runtime are
    /// API-specific)
    EnabledIfAvailable,

    /// Enable validation, and fail if we cannot enable it or detect that it is not enabled through
    /// external means. (Details on this are API-specific)
    Enabled,
}

impl Default for RafxValidationMode {
    fn default() -> Self {
        RafxValidationMode::Disabled
    }
}

pub struct RafxDeviceInfo {
    pub uniform_buffer_alignment: u32,
    pub upload_buffer_texture_alignment: u32,
    pub upload_buffer_texture_row_alignment: u32,
    // max_vertex_input_binding_count: u32,
    // max_root_signature_dwords: u32,
    // wave_lane_count: u32,
    // wave_ops_support_flags: u32,
    // gpu_vendor_preset: u32,
    // metal_argument_buffer_max_textures: u32,
    // metal_heaps: u32,
    // metal_placement_heaps: u32,
    // metal_draw_index_vertex_offset_supported: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RafxQueueType {
    Graphics,
    Compute,
    Transfer,
}

#[derive(Copy, Clone, Debug)]
pub enum RafxColorType {
    Linear,
    Srgb,
}

// /// Texture will allocate its own memory (COMMITTED resource)
// TEXTURE_CREATION_FLAG_OWN_MEMORY_BIT = 0x01,
// /// Use on-tile memory to store this texture
// TEXTURE_CREATION_FLAG_ON_TILE = 0x20,
// /// Force 2D instead of automatically determining dimension based on width, height, depth
// TEXTURE_CREATION_FLAG_FORCE_2D = 0x80,
// /// Force 3D instead of automatically determining dimension based on width, height, depth
// TEXTURE_CREATION_FLAG_FORCE_3D = 0x100,
// /// Display target
// TEXTURE_CREATION_FLAG_ALLOW_DISPLAY_TARGET = 0x200,
// /// Create an sRGB texture.
// TEXTURE_CREATION_FLAG_SRGB = 0x400,

bitflags::bitflags! {
    pub struct RafxResourceState: u32 {
        const UNDEFINED = 0;
        const VERTEX_AND_CONSTANT_BUFFER = 0x1;
        const INDEX_BUFFER = 0x2;
        const RENDER_TARGET = 0x4;
        const UNORDERED_ACCESS = 0x8;
        const DEPTH_WRITE = 0x10;
        const DEPTH_READ = 0x20;
        const NON_PIXEL_SHADER_RESOURCE = 0x40;
        const PIXEL_SHADER_RESOURCE = 0x80;
        const SHADER_RESOURCE = 0x40 | 0x80;
        const STREAM_OUT = 0x100;
        const INDIRECT_ARGUMENT = 0x200;
        const COPY_DST = 0x400;
        const COPY_SRC = 0x800;
        const GENERIC_READ = (((((0x1 | 0x2) | 0x40) | 0x80) | 0x200) | 0x800);
        const PRESENT = 0x1000;
        const COMMON = 0x2000;
    }
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RafxExtents2D {
    pub width: u32,
    pub height: u32,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RafxExtents3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxSampleCount {
    SampleCount1,
    SampleCount2,
    SampleCount4,
    SampleCount8,
    SampleCount16,
}

impl Default for RafxSampleCount {
    fn default() -> Self {
        RafxSampleCount::SampleCount1
    }
}

impl Into<vk::SampleCountFlags> for RafxSampleCount {
    fn into(self) -> vk::SampleCountFlags {
        match self {
            RafxSampleCount::SampleCount1 => vk::SampleCountFlags::TYPE_1,
            RafxSampleCount::SampleCount2 => vk::SampleCountFlags::TYPE_2,
            RafxSampleCount::SampleCount4 => vk::SampleCountFlags::TYPE_4,
            RafxSampleCount::SampleCount8 => vk::SampleCountFlags::TYPE_8,
            RafxSampleCount::SampleCount16 => vk::SampleCountFlags::TYPE_16,
        }
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
    pub struct RafxResourceType: u32 {
        const UNDEFINED = 0;
        const SAMPLER = 1<<0;
        // SRV
        const TEXTURE = 1<<1;
        // UAV
        const TEXTURE_READ_WRITE = 1<<2;
        // SRV
        const BUFFER = 1<<3;
        //const BUFFER_RAW = 1<<4 | RafxResourceType::BUFFER.bits();
        // UAV
        const BUFFER_READ_WRITE = 1<<5;
        //const BUFFER_READ_WRITE_RAW = 1<<6 | RafxResourceType::BUFFER_READ_WRITE.bits();
        // Uniform
        const UNIFORM_BUFFER = 1<<7;
        // Push constant / Root constant
        const ROOT_CONSTANT = 1<<8;
        // Input assembler
        const VERTEX_BUFFER = 1<<9;
        const INDEX_BUFFER = 1<<10;
        const INDIRECT_BUFFER = 1<<11;
        // Cubemap SRV
        const TEXTURE_CUBE = 1<<12 | RafxResourceType::TEXTURE.bits();
        // RTV
        const RENDER_TARGET_MIP_SLICES = 1<<13;
        const RENDER_TARGET_ARRAY_SLICES = 1<<14;
        const RENDER_TARGET_DEPTH_SLICES = 1<<15;
        // Vulkan-only stuff
        const INPUT_ATTACHMENT = 1<<16;
        const TEXEL_BUFFER = 1<<17;
        const TEXEL_BUFFER_READ_WRITE = 1<<18;
        const COMBINED_IMAGE_SAMPLER = 1<<19;
        // Metal-only stuff
        const ARGUMENT_BUFFER = 1<<20;
        const INDIRECT_COMMAND_BUFFER = 1<<21;
        const RENDER_PIPELINE_STATE = 1<<22;
        const RENDER_TARGET_COLOR = 1<<23;
        const RENDER_TARGET_DEPTH_STENCIL = 1<<24;
    }
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
    pub struct RafxColorFlags: u8 {
        const RED = 1;
        const GREEN = 2;
        const BLUE = 4;
        const ALPHA = 8;
        const ALL = 0x0F;
    }
}

impl Default for RafxColorFlags {
    fn default() -> Self {
        RafxColorFlags::ALL
    }
}

impl Into<vk::ColorComponentFlags> for RafxColorFlags {
    fn into(self) -> vk::ColorComponentFlags {
        let mut flags = vk::ColorComponentFlags::empty();
        if self.intersects(RafxColorFlags::RED) {
            flags |= vk::ColorComponentFlags::R
        }
        if self.intersects(RafxColorFlags::GREEN) {
            flags |= vk::ColorComponentFlags::G
        }
        if self.intersects(RafxColorFlags::BLUE) {
            flags |= vk::ColorComponentFlags::B
        }
        if self.intersects(RafxColorFlags::ALPHA) {
            flags |= vk::ColorComponentFlags::A
        }
        flags
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RafxMemoryUsage {
    Unknown,
    GpuOnly,
    CpuOnly,
    CpuToGpu,
    GpuToCpu,
}

impl Into<vk_mem::MemoryUsage> for RafxMemoryUsage {
    fn into(self) -> vk_mem::MemoryUsage {
        use vk_mem::MemoryUsage;
        match self {
            RafxMemoryUsage::Unknown => MemoryUsage::Unknown,
            RafxMemoryUsage::GpuOnly => MemoryUsage::GpuOnly,
            RafxMemoryUsage::CpuOnly => MemoryUsage::CpuOnly,
            RafxMemoryUsage::CpuToGpu => MemoryUsage::CpuToGpu,
            RafxMemoryUsage::GpuToCpu => MemoryUsage::GpuToCpu,
        }
    }
}

#[derive(Clone)]
pub enum RafxPresentSuccessResult {
    Success,
    SuccessSuboptimal,

    // While this is an "error" being returned as success, it is expected and recoverable while
    // other errors usually aren't. This way the ? operator can still be used to bail out the
    // unrecoverable errors and the different flavors of "success" should be explicitly handled
    // in a match
    DeviceReset,
}

#[derive(PartialEq)]
pub enum RafxFenceStatus {
    /// The fence was submitted to the command buffer and signaled as completed by the GPU
    Complete,
    /// The fence will be signaled as complete later by the GPU
    Incomplete,
    /// The fence was never submitted, or was submitted and already returned complete once, putting
    /// it back into the unsubmitted state
    Unsubmitted,
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
    pub struct RafxBlendStateTargets : u8 {
        const BLEND_STATE_TARGET_0 = 0x01;
        const BLEND_STATE_TARGET_1 = 0x02;
        const BLEND_STATE_TARGET_2 = 0x04;
        const BLEND_STATE_TARGET_3 = 0x08;
        const BLEND_STATE_TARGET_4 = 0x10;
        const BLEND_STATE_TARGET_5 = 0x20;
        const BLEND_STATE_TARGET_6 = 0x40;
        const BLEND_STATE_TARGET_7 = 0x80;
        const BLEND_STATE_TARGET_ALL = 0xFF;
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
    pub struct RafxShaderStageFlags : u32 {
        const NONE = 0;
        const VERTEX = 1;
        const TESSELLATION_CONTROL = 2;
        const TESSELLATION_EVALUATION = 4;
        const GEOMETRY = 8;
        const FRAGMENT = 16;
        const COMPUTE = 32;
        const ALL_GRAPHICS = 0x1F;
        const ALL = 0x7FFF_FFFF;
    }
}

pub const ALL_SHADER_STAGE_FLAGS: [RafxShaderStageFlags; 6] = [
    RafxShaderStageFlags::VERTEX,
    RafxShaderStageFlags::TESSELLATION_CONTROL,
    RafxShaderStageFlags::TESSELLATION_EVALUATION,
    RafxShaderStageFlags::GEOMETRY,
    RafxShaderStageFlags::FRAGMENT,
    RafxShaderStageFlags::COMPUTE,
];

impl Into<vk::ShaderStageFlags> for RafxShaderStageFlags {
    fn into(self) -> vk::ShaderStageFlags {
        let mut result = vk::ShaderStageFlags::empty();

        if self.intersects(RafxShaderStageFlags::VERTEX) {
            result |= vk::ShaderStageFlags::VERTEX;
        }

        if self.intersects(RafxShaderStageFlags::TESSELLATION_CONTROL) {
            result |= vk::ShaderStageFlags::TESSELLATION_CONTROL;
        }

        if self.intersects(RafxShaderStageFlags::TESSELLATION_EVALUATION) {
            result |= vk::ShaderStageFlags::TESSELLATION_EVALUATION;
        }

        if self.intersects(RafxShaderStageFlags::GEOMETRY) {
            result |= vk::ShaderStageFlags::GEOMETRY;
        }

        if self.intersects(RafxShaderStageFlags::FRAGMENT) {
            result |= vk::ShaderStageFlags::FRAGMENT;
        }

        if self.intersects(RafxShaderStageFlags::COMPUTE) {
            result |= vk::ShaderStageFlags::COMPUTE;
        }

        if self.contains(RafxShaderStageFlags::ALL_GRAPHICS) {
            result |= vk::ShaderStageFlags::ALL_GRAPHICS;
        }

        result
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RafxPipelineType {
    Graphics,
    Compute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RafxVertexAttributeRate {
    Vertex,
    Instance,
}

impl Default for RafxVertexAttributeRate {
    fn default() -> Self {
        RafxVertexAttributeRate::Vertex
    }
}

impl Into<vk::VertexInputRate> for RafxVertexAttributeRate {
    fn into(self) -> vk::VertexInputRate {
        match self {
            RafxVertexAttributeRate::Vertex => vk::VertexInputRate::VERTEX,
            RafxVertexAttributeRate::Instance => vk::VertexInputRate::INSTANCE,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum RafxLoadOp {
    DontCare,
    Load,
    Clear,
}

impl Default for RafxLoadOp {
    fn default() -> Self {
        RafxLoadOp::DontCare
    }
}

impl Into<vk::AttachmentLoadOp> for RafxLoadOp {
    fn into(self) -> vk::AttachmentLoadOp {
        match self {
            RafxLoadOp::DontCare => vk::AttachmentLoadOp::DONT_CARE,
            RafxLoadOp::Load => vk::AttachmentLoadOp::LOAD,
            RafxLoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxPrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    PatchList,
}

impl Into<vk::PrimitiveTopology> for RafxPrimitiveTopology {
    fn into(self) -> vk::PrimitiveTopology {
        match self {
            RafxPrimitiveTopology::PointList => vk::PrimitiveTopology::POINT_LIST,
            RafxPrimitiveTopology::LineList => vk::PrimitiveTopology::LINE_LIST,
            RafxPrimitiveTopology::LineStrip => vk::PrimitiveTopology::LINE_STRIP,
            RafxPrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
            RafxPrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
            RafxPrimitiveTopology::PatchList => vk::PrimitiveTopology::PATCH_LIST,
        }
    }
}

#[derive(Copy, Clone)]
pub enum RafxIndexType {
    Uint32,
    Uint16,
}

impl Default for RafxIndexType {
    fn default() -> Self {
        RafxIndexType::Uint32
    }
}

impl Into<vk::IndexType> for RafxIndexType {
    fn into(self) -> vk::IndexType {
        match self {
            RafxIndexType::Uint32 => vk::IndexType::UINT32,
            RafxIndexType::Uint16 => vk::IndexType::UINT16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxBlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    SrcAlphaSaturate,
    ConstantColor,
    OneMinusConstantColor,
}

impl Default for RafxBlendFactor {
    fn default() -> Self {
        RafxBlendFactor::Zero
    }
}

impl Into<vk::BlendFactor> for RafxBlendFactor {
    fn into(self) -> vk::BlendFactor {
        match self {
            RafxBlendFactor::Zero => vk::BlendFactor::ZERO,
            RafxBlendFactor::One => vk::BlendFactor::ONE,
            RafxBlendFactor::SrcColor => vk::BlendFactor::SRC_COLOR,
            RafxBlendFactor::OneMinusSrcColor => vk::BlendFactor::ONE_MINUS_SRC_COLOR,
            RafxBlendFactor::DstColor => vk::BlendFactor::DST_COLOR,
            RafxBlendFactor::OneMinusDstColor => vk::BlendFactor::ONE_MINUS_DST_COLOR,
            RafxBlendFactor::SrcAlpha => vk::BlendFactor::SRC_ALPHA,
            RafxBlendFactor::OneMinusSrcAlpha => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            RafxBlendFactor::DstAlpha => vk::BlendFactor::DST_ALPHA,
            RafxBlendFactor::OneMinusDstAlpha => vk::BlendFactor::ONE_MINUS_DST_ALPHA,
            RafxBlendFactor::SrcAlphaSaturate => vk::BlendFactor::SRC_ALPHA_SATURATE,
            RafxBlendFactor::ConstantColor => vk::BlendFactor::CONSTANT_COLOR,
            RafxBlendFactor::OneMinusConstantColor => vk::BlendFactor::ONE_MINUS_CONSTANT_COLOR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxBlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

impl Default for RafxBlendOp {
    fn default() -> Self {
        RafxBlendOp::Add
    }
}

impl Into<vk::BlendOp> for RafxBlendOp {
    fn into(self) -> vk::BlendOp {
        match self {
            RafxBlendOp::Add => vk::BlendOp::ADD,
            RafxBlendOp::Subtract => vk::BlendOp::SUBTRACT,
            RafxBlendOp::ReverseSubtract => vk::BlendOp::REVERSE_SUBTRACT,
            RafxBlendOp::Min => vk::BlendOp::MIN,
            RafxBlendOp::Max => vk::BlendOp::MAX,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxCompareOp {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

impl Default for RafxCompareOp {
    fn default() -> Self {
        RafxCompareOp::Never
    }
}

impl Into<vk::CompareOp> for RafxCompareOp {
    fn into(self) -> vk::CompareOp {
        match self {
            RafxCompareOp::Never => vk::CompareOp::NEVER,
            RafxCompareOp::Less => vk::CompareOp::LESS,
            RafxCompareOp::Equal => vk::CompareOp::EQUAL,
            RafxCompareOp::LessOrEqual => vk::CompareOp::LESS_OR_EQUAL,
            RafxCompareOp::Greater => vk::CompareOp::GREATER,
            RafxCompareOp::NotEqual => vk::CompareOp::NOT_EQUAL,
            RafxCompareOp::GreaterOrEqual => vk::CompareOp::GREATER_OR_EQUAL,
            RafxCompareOp::Always => vk::CompareOp::ALWAYS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxStencilOp {
    Keep,
    Zero,
    Replace,
    IncrementAndClamp,
    DecrementAndClamp,
    Invert,
    IncrementAndWrap,
    DecrementAndWrap,
}

impl Default for RafxStencilOp {
    fn default() -> Self {
        RafxStencilOp::Keep
    }
}

impl Into<vk::StencilOp> for RafxStencilOp {
    fn into(self) -> vk::StencilOp {
        match self {
            RafxStencilOp::Keep => vk::StencilOp::KEEP,
            RafxStencilOp::Zero => vk::StencilOp::ZERO,
            RafxStencilOp::Replace => vk::StencilOp::REPLACE,
            RafxStencilOp::IncrementAndClamp => vk::StencilOp::INCREMENT_AND_CLAMP,
            RafxStencilOp::DecrementAndClamp => vk::StencilOp::DECREMENT_AND_CLAMP,
            RafxStencilOp::Invert => vk::StencilOp::INVERT,
            RafxStencilOp::IncrementAndWrap => vk::StencilOp::INCREMENT_AND_WRAP,
            RafxStencilOp::DecrementAndWrap => vk::StencilOp::DECREMENT_AND_WRAP,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxCullMode {
    None,
    Back,
    Front,
    Both,
}

impl Default for RafxCullMode {
    fn default() -> Self {
        RafxCullMode::None
    }
}

impl Into<vk::CullModeFlags> for RafxCullMode {
    fn into(self) -> vk::CullModeFlags {
        match self {
            RafxCullMode::None => vk::CullModeFlags::NONE,
            RafxCullMode::Back => vk::CullModeFlags::BACK,
            RafxCullMode::Front => vk::CullModeFlags::FRONT,
            RafxCullMode::Both => vk::CullModeFlags::FRONT_AND_BACK,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxFrontFace {
    CounterClockwise,
    Clockwise,
}

impl Default for RafxFrontFace {
    fn default() -> Self {
        RafxFrontFace::CounterClockwise
    }
}

impl Into<vk::FrontFace> for RafxFrontFace {
    fn into(self) -> vk::FrontFace {
        match self {
            RafxFrontFace::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
            RafxFrontFace::Clockwise => vk::FrontFace::CLOCKWISE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxFillMode {
    Solid,
    Wireframe,
}

impl Default for RafxFillMode {
    fn default() -> Self {
        RafxFillMode::Solid
    }
}

impl Into<vk::PolygonMode> for RafxFillMode {
    fn into(self) -> vk::PolygonMode {
        match self {
            RafxFillMode::Solid => vk::PolygonMode::FILL,
            RafxFillMode::Wireframe => vk::PolygonMode::LINE,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxFilterType {
    Nearest,
    Linear,
}

impl Default for RafxFilterType {
    fn default() -> Self {
        RafxFilterType::Nearest
    }
}

impl Into<vk::Filter> for RafxFilterType {
    fn into(self) -> vk::Filter {
        match self {
            RafxFilterType::Nearest => vk::Filter::NEAREST,
            RafxFilterType::Linear => vk::Filter::LINEAR,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxAddressMode {
    Mirror,
    Repeat,
    ClampToEdge,
    ClampToBorder,
}

impl Default for RafxAddressMode {
    fn default() -> Self {
        RafxAddressMode::Mirror
    }
}

impl Into<vk::SamplerAddressMode> for RafxAddressMode {
    fn into(self) -> vk::SamplerAddressMode {
        match self {
            RafxAddressMode::Mirror => vk::SamplerAddressMode::MIRRORED_REPEAT,
            RafxAddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            RafxAddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            RafxAddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxMipMapMode {
    Nearest,
    Linear,
}

impl Default for RafxMipMapMode {
    fn default() -> Self {
        RafxMipMapMode::Nearest
    }
}

impl Into<vk::SamplerMipmapMode> for RafxMipMapMode {
    fn into(self) -> vk::SamplerMipmapMode {
        match self {
            RafxMipMapMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            RafxMipMapMode::Linear => vk::SamplerMipmapMode::LINEAR,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct RafxColorClearValue(pub [f32; 4]);

impl Hash for RafxColorClearValue {
    fn hash<H: Hasher>(
        &self,
        mut state: &mut H,
    ) {
        for &value in &self.0 {
            DecimalF32(value).hash(&mut state);
        }
    }
}

impl Into<vk::ClearValue> for RafxColorClearValue {
    fn into(self) -> vk::ClearValue {
        vk::ClearValue {
            color: vk::ClearColorValue { float32: self.0 },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RafxDepthStencilClearValue {
    pub depth: f32,
    pub stencil: u32,
}

impl Default for RafxDepthStencilClearValue {
    fn default() -> Self {
        RafxDepthStencilClearValue {
            depth: 0.0,
            stencil: 0,
        }
    }
}

impl Hash for RafxDepthStencilClearValue {
    fn hash<H: Hasher>(
        &self,
        mut state: &mut H,
    ) {
        DecimalF32(self.depth).hash(&mut state);
        self.stencil.hash(&mut state);
    }
}

impl Into<vk::ClearValue> for RafxDepthStencilClearValue {
    fn into(self) -> vk::ClearValue {
        vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: self.depth,
                stencil: self.stencil,
            },
        }
    }
}

// pub enum RafxBarrierSplit {
//     None,
//     BeginOnly,
//     EndOnly,
// }
//
// impl Default for RafxBarrierSplit {
//     fn default() -> Self {
//         RafxBarrierSplit::None
//     }
// }

pub enum RafxBarrierQueueTransition {
    None,
    // Use this on the SRC queue, but supply the DST queue type (the src queue is inferred by the
    // queue on which the barrier is submitted)
    ReleaseTo(RafxQueueType),
    // Use this on the DST queue, but supply the SRC queue type (the dst queue is inferred by the
    // queue on which the barrier is submitted)
    AcquireFrom(RafxQueueType),
}

impl Default for RafxBarrierQueueTransition {
    fn default() -> Self {
        RafxBarrierQueueTransition::None
    }
}

pub struct RafxBufferBarrier<'a> {
    pub buffer: &'a RafxBuffer,
    pub src_state: RafxResourceState,
    pub dst_state: RafxResourceState,
    //pub barrier_split: RafxBarrierSplit,
    pub queue_transition: RafxBarrierQueueTransition,
}

pub struct RafxBarrierSubresource {
    pub mip_level: u8,
    pub array_layer: u16,
}

pub struct RafxTextureBarrier<'a> {
    pub texture: &'a RafxTexture,
    pub src_state: RafxResourceState,
    pub dst_state: RafxResourceState,
    //pub barrier_split: RafxBarrierSplit,
    pub queue_transition: RafxBarrierQueueTransition,
    //pub subresource: Option<RafxBarrierSubresource>,
    pub array_slice: Option<u16>,
    pub mip_slice: Option<u8>,
}

pub struct RafxRenderTargetBarrier<'a> {
    pub render_target: &'a RafxRenderTarget,
    pub src_state: RafxResourceState,
    pub dst_state: RafxResourceState,
    //pub barrier_split: RafxBarrierSplit,
    pub queue_transition: RafxBarrierQueueTransition,
    //pub subresource: Option<RafxBarrierSubresource>,
    pub array_slice: Option<u16>,
    pub mip_slice: Option<u8>,
}

impl<'a> RafxRenderTargetBarrier<'a> {
    pub fn state_transition(
        render_target: &'a RafxRenderTarget,
        src_state: RafxResourceState,
        dst_state: RafxResourceState,
    ) -> RafxRenderTargetBarrier {
        RafxRenderTargetBarrier {
            render_target,
            src_state,
            dst_state,
            //barrier_split: RafxBarrierSplit::None,
            queue_transition: RafxBarrierQueueTransition::None,
            array_slice: None,
            mip_slice: None,
        }
    }
}

#[derive(Clone)]
pub struct RafxSwapchainImage {
    pub render_target: RafxRenderTarget,
    pub swapchain_image_index: u32,
}

#[derive(Debug)]
pub struct RafxColorRenderTargetBinding<'a> {
    pub render_target: &'a RafxRenderTarget,
    pub load_op: RafxLoadOp,
    pub mip_slice: Option<u8>,
    pub array_slice: Option<u16>,
    pub clear_value: RafxColorClearValue,
    pub resolve_target: Option<&'a RafxRenderTarget>,
    pub resolve_mip_slice: Option<u8>,
    pub resolve_array_slice: Option<u16>,
}

#[derive(Debug)]
pub struct RafxDepthRenderTargetBinding<'a> {
    pub render_target: &'a RafxRenderTarget,
    pub depth_load_op: RafxLoadOp,
    pub stencil_load_op: RafxLoadOp,
    pub mip_slice: Option<u8>,
    pub array_slice: Option<u16>,
    pub clear_value: RafxDepthStencilClearValue,
}

pub struct RafxVertexBufferBinding<'a> {
    pub buffer: &'a RafxBuffer,
    pub offset: u64,
}

pub struct RafxIndexBufferBinding<'a> {
    pub buffer: &'a RafxBuffer,
    pub offset: u64,
    pub index_type: RafxIndexType,
}

pub struct RafxCmdCopyBufferToTextureParams {
    pub buffer_offset: u64,
    pub array_layer: u16,
    pub mip_level: u8,
}

pub struct RafxCmdBlitParams {
    pub src_state: RafxResourceState,
    pub dst_state: RafxResourceState,
    pub src_extents: [RafxExtents3D; 2],
    pub dst_extents: [RafxExtents3D; 2],
    pub src_mip_level: u8,
    pub dst_mip_level: u8,
    pub array_slices: Option<[u16; 2]>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct RafxDescriptorIndex(pub(crate) u32);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RafxDescriptorKey<'a> {
    Undefined,
    Name(&'a str),
    Binding(u32),
    DescriptorIndex(RafxDescriptorIndex),
}

impl<'a> Default for RafxDescriptorKey<'a> {
    fn default() -> Self {
        RafxDescriptorKey::Undefined
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct RafxOffsetSize {
    pub offset: u64,
    pub size: u64,
}

#[derive(Default, Debug)]
pub struct RafxDescriptorElements<'a> {
    pub textures: Option<&'a [&'a RafxTexture]>,
    pub samplers: Option<&'a [&'a RafxSampler]>,
    pub buffers: Option<&'a [&'a RafxBuffer]>,
    pub buffer_offset_sizes: Option<&'a [RafxOffsetSize]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RafxTextureBindType {
    // Color or depth only
    Srv,
    // stencial?
    SrvStencil,
    // Bind all mip levels of the 0th provided texture
    UavMipChain,
    // Bind a particular mip slice of all provided textures
    UavMipSlice(u32),
}

#[derive(Debug)]
pub struct RafxDescriptorUpdate<'a> {
    pub array_index: u32,
    pub descriptor_key: RafxDescriptorKey<'a>,
    pub elements: RafxDescriptorElements<'a>,
    pub dst_element_offset: u32,
    // Srv when read-only, UavMipSlice(0) when read-write
    pub texture_bind_type: Option<RafxTextureBindType>,
}

impl<'a> Default for RafxDescriptorUpdate<'a> {
    fn default() -> Self {
        RafxDescriptorUpdate {
            array_index: 0,
            descriptor_key: RafxDescriptorKey::Undefined,
            elements: RafxDescriptorElements::default(),
            dst_element_offset: 0,
            texture_bind_type: None,
        }
    }
}
