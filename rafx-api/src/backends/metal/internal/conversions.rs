use crate::{
    RafxBlendFactor, RafxBlendOp, RafxColorClearValue, RafxCompareOp, RafxCullMode, RafxFillMode,
    RafxFilterType, RafxFrontFace, RafxIndexType, RafxLoadOp, RafxMemoryUsage, RafxMipMapMode,
    RafxPrimitiveTopology, RafxSampleCount, RafxStencilOp, RafxStoreOp, RafxVertexAttributeRate,
};
use cocoa_foundation::foundation::NSUInteger;
use metal_rs::{
    MTLBlendFactor, MTLBlendOperation, MTLCPUCacheMode, MTLClearColor, MTLCompareFunction,
    MTLCullMode, MTLIndexType, MTLLoadAction, MTLPrimitiveTopologyClass, MTLPrimitiveType,
    MTLResourceOptions, MTLSamplerMinMagFilter, MTLSamplerMipFilter, MTLStencilOperation,
    MTLStorageMode, MTLStoreAction, MTLTriangleFillMode, MTLVertexStepFunction, MTLWinding,
};

impl Into<MTLSamplerMinMagFilter> for RafxFilterType {
    fn into(self) -> MTLSamplerMinMagFilter {
        match self {
            RafxFilterType::Nearest => MTLSamplerMinMagFilter::Nearest,
            RafxFilterType::Linear => MTLSamplerMinMagFilter::Linear,
        }
    }
}

impl Into<MTLSamplerMipFilter> for RafxMipMapMode {
    fn into(self) -> MTLSamplerMipFilter {
        match self {
            RafxMipMapMode::Nearest => MTLSamplerMipFilter::Nearest,
            RafxMipMapMode::Linear => MTLSamplerMipFilter::Linear,
        }
    }
}

impl Into<NSUInteger> for RafxSampleCount {
    fn into(self) -> NSUInteger {
        match self {
            RafxSampleCount::SampleCount1 => 1,
            RafxSampleCount::SampleCount2 => 2,
            RafxSampleCount::SampleCount4 => 4,
            RafxSampleCount::SampleCount8 => 8,
            RafxSampleCount::SampleCount16 => 16,
        }
    }
}

impl Into<MTLVertexStepFunction> for RafxVertexAttributeRate {
    fn into(self) -> MTLVertexStepFunction {
        match self {
            RafxVertexAttributeRate::Vertex => MTLVertexStepFunction::PerVertex,
            RafxVertexAttributeRate::Instance => MTLVertexStepFunction::PerInstance,
        }
    }
}

impl Into<MTLPrimitiveTopologyClass> for RafxPrimitiveTopology {
    fn into(self) -> MTLPrimitiveTopologyClass {
        match self {
            RafxPrimitiveTopology::PointList => MTLPrimitiveTopologyClass::Point,
            RafxPrimitiveTopology::LineList => MTLPrimitiveTopologyClass::Line,
            RafxPrimitiveTopology::LineStrip => MTLPrimitiveTopologyClass::Line,
            RafxPrimitiveTopology::TriangleList => MTLPrimitiveTopologyClass::Triangle,
            RafxPrimitiveTopology::TriangleStrip => MTLPrimitiveTopologyClass::Triangle,
            RafxPrimitiveTopology::PatchList => MTLPrimitiveTopologyClass::Triangle,
        }
    }
}

impl Into<MTLPrimitiveType> for RafxPrimitiveTopology {
    fn into(self) -> MTLPrimitiveType {
        match self {
            RafxPrimitiveTopology::PointList => MTLPrimitiveType::Point,
            RafxPrimitiveTopology::LineList => MTLPrimitiveType::Line,
            RafxPrimitiveTopology::LineStrip => MTLPrimitiveType::LineStrip,
            RafxPrimitiveTopology::TriangleList => MTLPrimitiveType::Triangle,
            RafxPrimitiveTopology::TriangleStrip => MTLPrimitiveType::TriangleStrip,
            RafxPrimitiveTopology::PatchList => MTLPrimitiveType::Triangle,
        }
    }
}

impl Into<MTLIndexType> for RafxIndexType {
    fn into(self) -> MTLIndexType {
        match self {
            RafxIndexType::Uint32 => MTLIndexType::UInt32,
            RafxIndexType::Uint16 => MTLIndexType::UInt16,
        }
    }
}

impl Into<MTLTriangleFillMode> for RafxFillMode {
    fn into(self) -> MTLTriangleFillMode {
        match self {
            RafxFillMode::Solid => MTLTriangleFillMode::Fill,
            RafxFillMode::Wireframe => MTLTriangleFillMode::Lines,
        }
    }
}

impl Into<MTLWinding> for RafxFrontFace {
    fn into(self) -> MTLWinding {
        match self {
            RafxFrontFace::CounterClockwise => MTLWinding::CounterClockwise,
            RafxFrontFace::Clockwise => MTLWinding::Clockwise,
        }
    }
}

impl Into<MTLCullMode> for RafxCullMode {
    fn into(self) -> MTLCullMode {
        match self {
            RafxCullMode::None => MTLCullMode::None,
            RafxCullMode::Back => MTLCullMode::Back,
            RafxCullMode::Front => MTLCullMode::Front,
        }
    }
}

impl Into<MTLStencilOperation> for RafxStencilOp {
    fn into(self) -> MTLStencilOperation {
        match self {
            RafxStencilOp::Keep => MTLStencilOperation::Keep,
            RafxStencilOp::Zero => MTLStencilOperation::Zero,
            RafxStencilOp::Replace => MTLStencilOperation::Replace,
            RafxStencilOp::IncrementAndClamp => MTLStencilOperation::IncrementClamp,
            RafxStencilOp::DecrementAndClamp => MTLStencilOperation::DecrementClamp,
            RafxStencilOp::Invert => MTLStencilOperation::Invert,
            RafxStencilOp::IncrementAndWrap => MTLStencilOperation::IncrementWrap,
            RafxStencilOp::DecrementAndWrap => MTLStencilOperation::DecrementWrap,
        }
    }
}

impl Into<MTLCompareFunction> for RafxCompareOp {
    fn into(self) -> MTLCompareFunction {
        match self {
            RafxCompareOp::Never => MTLCompareFunction::Never,
            RafxCompareOp::Less => MTLCompareFunction::Less,
            RafxCompareOp::Equal => MTLCompareFunction::Equal,
            RafxCompareOp::LessOrEqual => MTLCompareFunction::LessEqual,
            RafxCompareOp::Greater => MTLCompareFunction::Greater,
            RafxCompareOp::NotEqual => MTLCompareFunction::NotEqual,
            RafxCompareOp::GreaterOrEqual => MTLCompareFunction::GreaterEqual,
            RafxCompareOp::Always => MTLCompareFunction::Always,
        }
    }
}

impl Into<MTLBlendOperation> for RafxBlendOp {
    fn into(self) -> MTLBlendOperation {
        match self {
            RafxBlendOp::Add => MTLBlendOperation::Add,
            RafxBlendOp::Subtract => MTLBlendOperation::Subtract,
            RafxBlendOp::ReverseSubtract => MTLBlendOperation::ReverseSubtract,
            RafxBlendOp::Min => MTLBlendOperation::Min,
            RafxBlendOp::Max => MTLBlendOperation::Max,
        }
    }
}

impl Into<MTLBlendFactor> for RafxBlendFactor {
    fn into(self) -> MTLBlendFactor {
        match self {
            RafxBlendFactor::Zero => MTLBlendFactor::Zero,
            RafxBlendFactor::One => MTLBlendFactor::One,
            RafxBlendFactor::SrcColor => MTLBlendFactor::SourceColor,
            RafxBlendFactor::OneMinusSrcColor => MTLBlendFactor::OneMinusSourceColor,
            RafxBlendFactor::DstColor => MTLBlendFactor::DestinationColor,
            RafxBlendFactor::OneMinusDstColor => MTLBlendFactor::OneMinusDestinationColor,
            RafxBlendFactor::SrcAlpha => MTLBlendFactor::SourceAlpha,
            RafxBlendFactor::OneMinusSrcAlpha => MTLBlendFactor::OneMinusSourceAlpha,
            RafxBlendFactor::DstAlpha => MTLBlendFactor::DestinationAlpha,
            RafxBlendFactor::OneMinusDstAlpha => MTLBlendFactor::OneMinusDestinationAlpha,
            RafxBlendFactor::SrcAlphaSaturate => MTLBlendFactor::SourceAlphaSaturated,
            RafxBlendFactor::ConstantColor => MTLBlendFactor::BlendColor,
            RafxBlendFactor::OneMinusConstantColor => MTLBlendFactor::OneMinusBlendColor,
        }
    }
}

impl Into<MTLLoadAction> for RafxLoadOp {
    fn into(self) -> MTLLoadAction {
        match self {
            RafxLoadOp::DontCare => MTLLoadAction::DontCare,
            RafxLoadOp::Load => MTLLoadAction::Load,
            RafxLoadOp::Clear => MTLLoadAction::Clear,
        }
    }
}

impl Into<MTLStoreAction> for RafxStoreOp {
    fn into(self) -> MTLStoreAction {
        match self {
            RafxStoreOp::DontCare => MTLStoreAction::DontCare,
            RafxStoreOp::Store => MTLStoreAction::Store,
        }
    }
}

impl Into<MTLClearColor> for RafxColorClearValue {
    fn into(self) -> MTLClearColor {
        MTLClearColor::new(
            self.0[0] as f64,
            self.0[1] as f64,
            self.0[2] as f64,
            self.0[3] as f64,
        )
    }
}

impl RafxMemoryUsage {
    pub fn mtl_resource_options(self) -> MTLResourceOptions {
        match self {
            RafxMemoryUsage::Unknown => MTLResourceOptions::empty(),
            // TODO: This can be shared on iGPU/iOS/M1
            RafxMemoryUsage::GpuOnly => MTLResourceOptions::StorageModePrivate,
            RafxMemoryUsage::CpuOnly => {
                MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeDefaultCache
            }
            RafxMemoryUsage::CpuToGpu => {
                MTLResourceOptions::StorageModeShared
                    | MTLResourceOptions::CPUCacheModeWriteCombined
            }
            RafxMemoryUsage::GpuToCpu => {
                MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeDefaultCache
            }
        }
    }

    pub fn mtl_cpu_cache_mode(self) -> MTLCPUCacheMode {
        match self {
            RafxMemoryUsage::Unknown => MTLCPUCacheMode::DefaultCache,
            RafxMemoryUsage::GpuOnly => MTLCPUCacheMode::DefaultCache,
            RafxMemoryUsage::CpuOnly => MTLCPUCacheMode::DefaultCache,
            RafxMemoryUsage::CpuToGpu => MTLCPUCacheMode::WriteCombined,
            RafxMemoryUsage::GpuToCpu => MTLCPUCacheMode::DefaultCache,
        }
    }

    pub fn mtl_storage_mode(self) -> MTLStorageMode {
        match self {
            RafxMemoryUsage::Unknown => MTLStorageMode::Private,
            // TODO: This can be shared on iGPU/iOS/M1
            RafxMemoryUsage::GpuOnly => MTLStorageMode::Private,
            RafxMemoryUsage::CpuOnly => MTLStorageMode::Shared,
            RafxMemoryUsage::CpuToGpu => MTLStorageMode::Shared,
            RafxMemoryUsage::GpuToCpu => MTLStorageMode::Shared,
        }
    }
}
