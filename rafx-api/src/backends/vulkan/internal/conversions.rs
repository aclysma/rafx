use crate::{
    RafxAddressMode, RafxBlendFactor, RafxBlendOp, RafxColorClearValue, RafxColorFlags,
    RafxCompareOp, RafxCullMode, RafxDepthStencilClearValue, RafxFillMode, RafxFilterType,
    RafxFrontFace, RafxIndexType, RafxLoadOp, RafxMemoryUsage, RafxMipMapMode,
    RafxPrimitiveTopology, RafxSampleCount, RafxShaderStageFlags, RafxStencilOp, RafxStoreOp,
    RafxVertexAttributeRate,
};
use ash::vk;

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

impl Into<vk::VertexInputRate> for RafxVertexAttributeRate {
    fn into(self) -> vk::VertexInputRate {
        match self {
            RafxVertexAttributeRate::Vertex => vk::VertexInputRate::VERTEX,
            RafxVertexAttributeRate::Instance => vk::VertexInputRate::INSTANCE,
        }
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

impl Into<vk::AttachmentStoreOp> for RafxStoreOp {
    fn into(self) -> vk::AttachmentStoreOp {
        match self {
            RafxStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
            RafxStoreOp::Store => vk::AttachmentStoreOp::STORE,
        }
    }
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

impl Into<vk::IndexType> for RafxIndexType {
    fn into(self) -> vk::IndexType {
        match self {
            RafxIndexType::Uint32 => vk::IndexType::UINT32,
            RafxIndexType::Uint16 => vk::IndexType::UINT16,
        }
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

impl Into<vk::CullModeFlags> for RafxCullMode {
    fn into(self) -> vk::CullModeFlags {
        match self {
            RafxCullMode::None => vk::CullModeFlags::NONE,
            RafxCullMode::Back => vk::CullModeFlags::BACK,
            RafxCullMode::Front => vk::CullModeFlags::FRONT,
        }
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

impl Into<vk::PolygonMode> for RafxFillMode {
    fn into(self) -> vk::PolygonMode {
        match self {
            RafxFillMode::Solid => vk::PolygonMode::FILL,
            RafxFillMode::Wireframe => vk::PolygonMode::LINE,
        }
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

impl Into<vk::SamplerMipmapMode> for RafxMipMapMode {
    fn into(self) -> vk::SamplerMipmapMode {
        match self {
            RafxMipMapMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            RafxMipMapMode::Linear => vk::SamplerMipmapMode::LINEAR,
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
