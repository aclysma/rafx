use super::d3d;
use super::d3d12;
use crate::{
    RafxAddressMode, RafxBlendFactor, RafxBlendOp, RafxBlendState, RafxBlendStateTargets,
    RafxCompareOp, RafxCullMode, RafxDepthState, RafxFillMode, RafxFrontFace,
    RafxPrimitiveTopology, RafxRasterizerState, RafxResourceState, RafxResourceType,
    RafxShaderStageFlags, RafxStencilOp, MAX_RENDER_TARGET_ATTACHMENTS,
};

impl Into<d3d12::D3D12_TEXTURE_ADDRESS_MODE> for RafxAddressMode {
    fn into(self) -> d3d12::D3D12_TEXTURE_ADDRESS_MODE {
        match self {
            RafxAddressMode::Mirror => d3d12::D3D12_TEXTURE_ADDRESS_MODE_MIRROR,
            RafxAddressMode::Repeat => d3d12::D3D12_TEXTURE_ADDRESS_MODE_WRAP,
            RafxAddressMode::ClampToEdge => d3d12::D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            RafxAddressMode::ClampToBorder => d3d12::D3D12_TEXTURE_ADDRESS_MODE_BORDER,
        }
    }
}

impl Into<d3d12::D3D12_SHADER_VISIBILITY> for RafxShaderStageFlags {
    fn into(self) -> d3d12::D3D12_SHADER_VISIBILITY {
        if self.intersects(RafxShaderStageFlags::COMPUTE) {
            return d3d12::D3D12_SHADER_VISIBILITY_ALL;
        }

        let mut stage_count = 0;
        let mut visibility = d3d12::D3D12_SHADER_VISIBILITY_ALL;

        if self.intersects(RafxShaderStageFlags::VERTEX) {
            stage_count += 1;
            visibility = d3d12::D3D12_SHADER_VISIBILITY_VERTEX;
        }

        if self.intersects(RafxShaderStageFlags::GEOMETRY) {
            stage_count += 1;
            visibility = d3d12::D3D12_SHADER_VISIBILITY_GEOMETRY;
        }

        if self.intersects(RafxShaderStageFlags::TESSELLATION_CONTROL) {
            stage_count += 1;
            visibility = d3d12::D3D12_SHADER_VISIBILITY_HULL;
        }

        if self.intersects(RafxShaderStageFlags::TESSELLATION_EVALUATION) {
            stage_count += 1;
            visibility = d3d12::D3D12_SHADER_VISIBILITY_DOMAIN;
        }

        if self.intersects(RafxShaderStageFlags::FRAGMENT) {
            stage_count += 1;
            visibility = d3d12::D3D12_SHADER_VISIBILITY_PIXEL;
        }

        if stage_count == 1 {
            visibility
        } else {
            d3d12::D3D12_SHADER_VISIBILITY_ALL
        }
    }
}

pub fn resource_type_descriptor_range_type(
    resource_type: RafxResourceType
) -> Option<d3d12::D3D12_DESCRIPTOR_RANGE_TYPE> {
    if resource_type.intersects(RafxResourceType::UNIFORM_BUFFER | RafxResourceType::ROOT_CONSTANT)
    {
        return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_CBV);
    }

    if resource_type
        .intersects(RafxResourceType::BUFFER_READ_WRITE | RafxResourceType::TEXTURE_READ_WRITE)
    {
        return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_UAV);
    }

    if resource_type.intersects(RafxResourceType::SAMPLER) {
        return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER);
    }

    if resource_type.intersects(
        RafxResourceType::BUFFER
            | RafxResourceType::TEXTURE
            | RafxResourceType::COMBINED_IMAGE_SAMPLER,
    ) {
        return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_SRV);
    }

    None
}

impl Into<d3d12::D3D12_RESOURCE_STATES> for RafxResourceState {
    fn into(self) -> d3d12::D3D12_RESOURCE_STATES {
        let mut state = d3d12::D3D12_RESOURCE_STATE_COMMON;

        if self == RafxResourceState::GENERIC_READ {
            return d3d12::D3D12_RESOURCE_STATE_GENERIC_READ;
        }
        if self == RafxResourceState::COMMON {
            return d3d12::D3D12_RESOURCE_STATE_COMMON;
        }
        if self == RafxResourceState::PRESENT {
            return d3d12::D3D12_RESOURCE_STATE_PRESENT;
        }

        if self.intersects(RafxResourceState::VERTEX_AND_CONSTANT_BUFFER) {
            state |= d3d12::D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER;
        }
        if self.intersects(RafxResourceState::INDEX_BUFFER) {
            state |= d3d12::D3D12_RESOURCE_STATE_INDEX_BUFFER;
        }
        if self.intersects(RafxResourceState::RENDER_TARGET) {
            state |= d3d12::D3D12_RESOURCE_STATE_RENDER_TARGET;
        }
        if self.intersects(RafxResourceState::UNORDERED_ACCESS) {
            state |= d3d12::D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        }
        if self.intersects(RafxResourceState::DEPTH_WRITE) {
            state |= d3d12::D3D12_RESOURCE_STATE_DEPTH_WRITE;
        }
        if self.intersects(RafxResourceState::DEPTH_READ) {
            state |= d3d12::D3D12_RESOURCE_STATE_DEPTH_READ;
        }
        if self.intersects(RafxResourceState::STREAM_OUT) {
            state |= d3d12::D3D12_RESOURCE_STATE_STREAM_OUT;
        }
        if self.intersects(RafxResourceState::INDIRECT_ARGUMENT) {
            state |= d3d12::D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT;
        }
        if self.intersects(RafxResourceState::COPY_DST) {
            state |= d3d12::D3D12_RESOURCE_STATE_COPY_DEST;
        }
        if self.intersects(RafxResourceState::COPY_SRC) {
            state |= d3d12::D3D12_RESOURCE_STATE_COPY_SOURCE;
        }
        if self.intersects(RafxResourceState::NON_PIXEL_SHADER_RESOURCE) {
            state |= d3d12::D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
        }
        if self.intersects(RafxResourceState::PIXEL_SHADER_RESOURCE) {
            state |= d3d12::D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
        }

        state
    }
}

pub fn rasterizer_state_rasterizer_desc(
    rasterizer_state: &RafxRasterizerState
) -> d3d12::D3D12_RASTERIZER_DESC {
    d3d12::D3D12_RASTERIZER_DESC {
        FillMode: rasterizer_state.fill_mode.into(),
        CullMode: rasterizer_state.cull_mode.into(),
        FrontCounterClockwise: (rasterizer_state.front_face == RafxFrontFace::CounterClockwise)
            .into(),
        DepthBias: rasterizer_state.depth_bias,
        DepthBiasClamp: 0.0,
        SlopeScaledDepthBias: rasterizer_state.depth_bias_slope_scaled,
        DepthClipEnable: (!rasterizer_state.depth_clamp_enable).into(),
        MultisampleEnable: rasterizer_state.multisample.into(),
        AntialiasedLineEnable: false.into(),
        ForcedSampleCount: 0,
        ConservativeRaster: d3d12::D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
    }
}

pub fn depth_state_depth_stencil_desc(
    depth_state: &RafxDepthState
) -> d3d12::D3D12_DEPTH_STENCIL_DESC {
    d3d12::D3D12_DEPTH_STENCIL_DESC {
        DepthEnable: depth_state.depth_test_enable.into(),
        DepthWriteMask: if depth_state.depth_write_enable {
            d3d12::D3D12_DEPTH_WRITE_MASK_ALL
        } else {
            d3d12::D3D12_DEPTH_WRITE_MASK_ZERO
        },
        DepthFunc: depth_state.depth_compare_op.into(),
        StencilEnable: depth_state.stencil_test_enable.into(),
        StencilReadMask: depth_state.stencil_read_mask,
        StencilWriteMask: depth_state.stencil_write_mask,
        FrontFace: d3d12::D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: depth_state.front_stencil_fail_op.into(),
            StencilDepthFailOp: depth_state.front_depth_fail_op.into(),
            StencilPassOp: depth_state.front_stencil_pass_op.into(),
            StencilFunc: depth_state.front_stencil_compare_op.into(),
        },
        BackFace: d3d12::D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: depth_state.back_stencil_fail_op.into(),
            StencilDepthFailOp: depth_state.back_depth_fail_op.into(),
            StencilPassOp: depth_state.back_stencil_pass_op.into(),
            StencilFunc: depth_state.back_stencil_compare_op.into(),
        },
    }
}

pub fn blend_state_blend_state_desc(
    blend_state: &RafxBlendState,
    color_attachment_count: usize,
) -> d3d12::D3D12_BLEND_DESC {
    blend_state.verify(color_attachment_count);

    let mut blend_desc = d3d12::D3D12_BLEND_DESC::default();
    blend_desc.AlphaToCoverageEnable = false.into(); // blend_state.alpha_to_coverage_enable?
    blend_desc.IndependentBlendEnable = blend_state.independent_blend.into();

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

                let def = &blend_state.render_target_blend_states[def_index];

                let blend_enable = def.src_factor != RafxBlendFactor::One
                    || def.dst_factor != RafxBlendFactor::Zero
                    || def.src_factor_alpha != RafxBlendFactor::One
                    || def.dst_factor_alpha != RafxBlendFactor::Zero;

                let desc = &mut blend_desc.RenderTarget[attachment_index as usize];
                desc.BlendEnable = blend_enable.into();
                desc.RenderTargetWriteMask = def.masks.bits();
                desc.BlendOp = def.blend_op.into();
                desc.SrcBlend = def.src_factor.into();
                desc.DestBlend = def.dst_factor.into();
                desc.BlendOpAlpha = def.blend_op_alpha.into();
                desc.SrcBlendAlpha = def.src_factor_alpha.into();
                desc.DestBlendAlpha = def.dst_factor_alpha.into();
            }
        }
    }

    blend_desc
}

// impl Into<Option<d3d12::D3D12_DESCRIPTOR_RANGE_TYPE>> for RafxResourceType {
//     fn into(self) -> Option<d3d12::D3D12_DESCRIPTOR_RANGE_TYPE> {
//         if self.intersects(RafxResourceType::UNIFORM_BUFFER | RafxResourceType::ROOT_CONSTANT) {
//             return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_CBV);
//         }
//
//         if self.intersects(RafxResourceType::BUFFER_READ_WRITE | RafxResourceType::TEXTURE_READ_WRITE) {
//             return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_UAV);
//         }
//
//         if self.intersects(RafxResourceType::SAMPLER) {
//             return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER);
//         }
//
//         if self.intersects(RafxResourceType::BUFFER | RafxResourceType::TEXTURE | RafxResourceType::COMBINED_IMAGE_SAMPLER) {
//             return Some(d3d12::D3D12_DESCRIPTOR_RANGE_TYPE_SRV);
//         }
//
//         None
//     }
// }

/*
impl Into<CFStringRef> for RafxSwapchainColorSpace {
    fn into(self) -> CFStringRef {
        unsafe {
            match self {
                RafxSwapchainColorSpace::Srgb => super::extra_ffi::kCGColorSpaceSRGB,
                RafxSwapchainColorSpace::SrgbExtended => {
                    super::extra_ffi::kCGColorSpaceExtendedLinearSRGB
                }
                RafxSwapchainColorSpace::DisplayP3Extended => {
                    super::extra_ffi::kCGColorSpaceExtendedLinearDisplayP3
                }
            }
        }
    }
}

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

impl Into<MTLVertexStepFunction> for RafxVertexAttributeRate {
    fn into(self) -> MTLVertexStepFunction {
        match self {
            RafxVertexAttributeRate::Vertex => MTLVertexStepFunction::PerVertex,
            RafxVertexAttributeRate::Instance => MTLVertexStepFunction::PerInstance,
        }
    }
}
*/

impl Into<d3d::D3D_PRIMITIVE_TOPOLOGY> for RafxPrimitiveTopology {
    fn into(self) -> d3d::D3D_PRIMITIVE_TOPOLOGY {
        match self {
            RafxPrimitiveTopology::PointList => d3d::D3D_PRIMITIVE_TOPOLOGY_POINTLIST,
            RafxPrimitiveTopology::LineList => d3d::D3D_PRIMITIVE_TOPOLOGY_LINELIST,
            RafxPrimitiveTopology::LineStrip => d3d::D3D_PRIMITIVE_TOPOLOGY_LINESTRIP,
            RafxPrimitiveTopology::TriangleList => d3d::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            RafxPrimitiveTopology::TriangleStrip => d3d::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
            RafxPrimitiveTopology::PatchList => unimplemented!(), // will require additional shader reflection info
        }
    }
}

impl Into<d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE> for RafxPrimitiveTopology {
    fn into(self) -> d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE {
        match self {
            RafxPrimitiveTopology::PointList => d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE_POINT,
            RafxPrimitiveTopology::LineList => d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE,
            RafxPrimitiveTopology::LineStrip => d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE,
            RafxPrimitiveTopology::TriangleList => d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            RafxPrimitiveTopology::TriangleStrip => d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            RafxPrimitiveTopology::PatchList => d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE_PATCH,
        }
    }
}

/*
impl Into<MTLIndexType> for RafxIndexType {
    fn into(self) -> MTLIndexType {
        match self {
            RafxIndexType::Uint32 => MTLIndexType::UInt32,
            RafxIndexType::Uint16 => MTLIndexType::UInt16,
        }
    }
}
*/
impl Into<d3d12::D3D12_FILL_MODE> for RafxFillMode {
    fn into(self) -> d3d12::D3D12_FILL_MODE {
        match self {
            RafxFillMode::Solid => d3d12::D3D12_FILL_MODE_SOLID,
            RafxFillMode::Wireframe => d3d12::D3D12_FILL_MODE_WIREFRAME,
        }
    }
}

impl Into<d3d12::D3D12_CULL_MODE> for RafxCullMode {
    fn into(self) -> d3d12::D3D12_CULL_MODE {
        match self {
            RafxCullMode::None => d3d12::D3D12_CULL_MODE_NONE,
            RafxCullMode::Back => d3d12::D3D12_CULL_MODE_BACK,
            RafxCullMode::Front => d3d12::D3D12_CULL_MODE_FRONT,
        }
    }
}

impl Into<d3d12::D3D12_STENCIL_OP> for RafxStencilOp {
    fn into(self) -> d3d12::D3D12_STENCIL_OP {
        match self {
            RafxStencilOp::Keep => d3d12::D3D12_STENCIL_OP_KEEP,
            RafxStencilOp::Zero => d3d12::D3D12_STENCIL_OP_ZERO,
            RafxStencilOp::Replace => d3d12::D3D12_STENCIL_OP_REPLACE,
            RafxStencilOp::IncrementAndClamp => d3d12::D3D12_STENCIL_OP_INCR_SAT,
            RafxStencilOp::DecrementAndClamp => d3d12::D3D12_STENCIL_OP_DECR_SAT,
            RafxStencilOp::Invert => d3d12::D3D12_STENCIL_OP_INVERT,
            RafxStencilOp::IncrementAndWrap => d3d12::D3D12_STENCIL_OP_INCR,
            RafxStencilOp::DecrementAndWrap => d3d12::D3D12_STENCIL_OP_DECR,
        }
    }
}

impl Into<d3d12::D3D12_COMPARISON_FUNC> for RafxCompareOp {
    fn into(self) -> d3d12::D3D12_COMPARISON_FUNC {
        match self {
            RafxCompareOp::Never => d3d12::D3D12_COMPARISON_FUNC_NEVER,
            RafxCompareOp::Less => d3d12::D3D12_COMPARISON_FUNC_LESS,
            RafxCompareOp::Equal => d3d12::D3D12_COMPARISON_FUNC_EQUAL,
            RafxCompareOp::LessOrEqual => d3d12::D3D12_COMPARISON_FUNC_LESS_EQUAL,
            RafxCompareOp::Greater => d3d12::D3D12_COMPARISON_FUNC_GREATER,
            RafxCompareOp::NotEqual => d3d12::D3D12_COMPARISON_FUNC_NOT_EQUAL,
            RafxCompareOp::GreaterOrEqual => d3d12::D3D12_COMPARISON_FUNC_GREATER_EQUAL,
            RafxCompareOp::Always => d3d12::D3D12_COMPARISON_FUNC_ALWAYS,
        }
    }
}

impl Into<d3d12::D3D12_BLEND_OP> for RafxBlendOp {
    fn into(self) -> d3d12::D3D12_BLEND_OP {
        match self {
            RafxBlendOp::Add => d3d12::D3D12_BLEND_OP_ADD,
            RafxBlendOp::Subtract => d3d12::D3D12_BLEND_OP_SUBTRACT,
            RafxBlendOp::ReverseSubtract => d3d12::D3D12_BLEND_OP_REV_SUBTRACT,
            RafxBlendOp::Min => d3d12::D3D12_BLEND_OP_MIN,
            RafxBlendOp::Max => d3d12::D3D12_BLEND_OP_MAX,
        }
    }
}

impl Into<d3d12::D3D12_BLEND> for RafxBlendFactor {
    fn into(self) -> d3d12::D3D12_BLEND {
        match self {
            RafxBlendFactor::Zero => d3d12::D3D12_BLEND_ZERO,
            RafxBlendFactor::One => d3d12::D3D12_BLEND_ONE,
            RafxBlendFactor::SrcColor => d3d12::D3D12_BLEND_SRC_COLOR,
            RafxBlendFactor::OneMinusSrcColor => d3d12::D3D12_BLEND_INV_SRC_COLOR,
            RafxBlendFactor::DstColor => d3d12::D3D12_BLEND_DEST_COLOR,
            RafxBlendFactor::OneMinusDstColor => d3d12::D3D12_BLEND_INV_DEST_COLOR,
            RafxBlendFactor::SrcAlpha => d3d12::D3D12_BLEND_SRC_ALPHA,
            RafxBlendFactor::OneMinusSrcAlpha => d3d12::D3D12_BLEND_INV_SRC_ALPHA,
            RafxBlendFactor::DstAlpha => d3d12::D3D12_BLEND_DEST_ALPHA,
            RafxBlendFactor::OneMinusDstAlpha => d3d12::D3D12_BLEND_INV_DEST_ALPHA,
            RafxBlendFactor::SrcAlphaSaturate => d3d12::D3D12_BLEND_SRC_ALPHA_SAT,
            RafxBlendFactor::ConstantColor => d3d12::D3D12_BLEND_BLEND_FACTOR,
            RafxBlendFactor::OneMinusConstantColor => d3d12::D3D12_BLEND_INV_BLEND_FACTOR,
        }
    }
}

/*
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
*/

pub fn dxgi_to_srv_format(
    format: super::dxgi::Common::DXGI_FORMAT
) -> super::dxgi::Common::DXGI_FORMAT {
    use super::dxgi::Common as DxgiCommon;
    match format {
        DxgiCommon::DXGI_FORMAT_R32G8X24_TYPELESS => {
            DxgiCommon::DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS
        }
        DxgiCommon::DXGI_FORMAT_D32_FLOAT_S8X24_UINT => {
            DxgiCommon::DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS
        }
        DxgiCommon::DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS => {
            DxgiCommon::DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS
        }
        DxgiCommon::DXGI_FORMAT_X32_TYPELESS_G8X24_UINT => {
            DxgiCommon::DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS
        }

        DxgiCommon::DXGI_FORMAT_R32_TYPELESS => DxgiCommon::DXGI_FORMAT_R32_FLOAT,
        DxgiCommon::DXGI_FORMAT_D32_FLOAT => DxgiCommon::DXGI_FORMAT_R32_FLOAT,
        DxgiCommon::DXGI_FORMAT_R32_FLOAT => DxgiCommon::DXGI_FORMAT_R32_FLOAT,

        DxgiCommon::DXGI_FORMAT_R24G8_TYPELESS => DxgiCommon::DXGI_FORMAT_R24_UNORM_X8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_D24_UNORM_S8_UINT => DxgiCommon::DXGI_FORMAT_R24_UNORM_X8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R24_UNORM_X8_TYPELESS => {
            DxgiCommon::DXGI_FORMAT_R24_UNORM_X8_TYPELESS
        }
        DxgiCommon::DXGI_FORMAT_X24_TYPELESS_G8_UINT => {
            DxgiCommon::DXGI_FORMAT_R24_UNORM_X8_TYPELESS
        }

        DxgiCommon::DXGI_FORMAT_R16_TYPELESS => DxgiCommon::DXGI_FORMAT_R16_UNORM,
        DxgiCommon::DXGI_FORMAT_D16_UNORM => DxgiCommon::DXGI_FORMAT_R16_UNORM,
        DxgiCommon::DXGI_FORMAT_R16_UNORM => DxgiCommon::DXGI_FORMAT_R16_UNORM,

        DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS => DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM,

        _ => format,
    }
}

pub fn dxgi_to_uav_format(
    format: super::dxgi::Common::DXGI_FORMAT
) -> super::dxgi::Common::DXGI_FORMAT {
    use super::dxgi::Common as DxgiCommon;
    match format {
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM => DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM,
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM,
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS => DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM,

        DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM => DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM,
        DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM,
        DxgiCommon::DXGI_FORMAT_B8G8R8A8_TYPELESS => DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM,

        DxgiCommon::DXGI_FORMAT_B8G8R8X8_UNORM => DxgiCommon::DXGI_FORMAT_B8G8R8X8_UNORM,
        DxgiCommon::DXGI_FORMAT_B8G8R8X8_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_B8G8R8X8_UNORM,
        DxgiCommon::DXGI_FORMAT_B8G8R8X8_TYPELESS => DxgiCommon::DXGI_FORMAT_B8G8R8X8_UNORM,

        DxgiCommon::DXGI_FORMAT_R32_TYPELESS => DxgiCommon::DXGI_FORMAT_R32_FLOAT,
        DxgiCommon::DXGI_FORMAT_R32_FLOAT => DxgiCommon::DXGI_FORMAT_R32_FLOAT,

        _ => format,
    }
}

pub fn dxgi_to_typeless(
    format: super::dxgi::Common::DXGI_FORMAT
) -> super::dxgi::Common::DXGI_FORMAT {
    use super::dxgi::Common as DxgiCommon;
    match format {
        DxgiCommon::DXGI_FORMAT_R32G32B32A32_FLOAT => DxgiCommon::DXGI_FORMAT_R32G32B32A32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32B32A32_UINT => DxgiCommon::DXGI_FORMAT_R32G32B32A32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32B32A32_SINT => DxgiCommon::DXGI_FORMAT_R32G32B32A32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32B32A32_TYPELESS => {
            DxgiCommon::DXGI_FORMAT_R32G32B32A32_TYPELESS
        }

        DxgiCommon::DXGI_FORMAT_R32G32B32_FLOAT => DxgiCommon::DXGI_FORMAT_R32G32B32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32B32_UINT => DxgiCommon::DXGI_FORMAT_R32G32B32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32B32_SINT => DxgiCommon::DXGI_FORMAT_R32G32B32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32B32_TYPELESS => DxgiCommon::DXGI_FORMAT_R32G32B32_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R32G32_FLOAT => DxgiCommon::DXGI_FORMAT_R32G32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32_UINT => DxgiCommon::DXGI_FORMAT_R32G32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32_SINT => DxgiCommon::DXGI_FORMAT_R32G32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G32_TYPELESS => DxgiCommon::DXGI_FORMAT_R32G32_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R32_FLOAT => DxgiCommon::DXGI_FORMAT_R32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32_UINT => DxgiCommon::DXGI_FORMAT_R32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32_SINT => DxgiCommon::DXGI_FORMAT_R32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R9G9B9E5_SHAREDEXP => DxgiCommon::DXGI_FORMAT_R32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_D32_FLOAT => DxgiCommon::DXGI_FORMAT_R32_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32_TYPELESS => DxgiCommon::DXGI_FORMAT_R32_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R16G16B16A16_FLOAT => DxgiCommon::DXGI_FORMAT_R16G16B16A16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16B16A16_UNORM => DxgiCommon::DXGI_FORMAT_R16G16B16A16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16B16A16_UINT => DxgiCommon::DXGI_FORMAT_R16G16B16A16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16B16A16_SNORM => DxgiCommon::DXGI_FORMAT_R16G16B16A16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16B16A16_SINT => DxgiCommon::DXGI_FORMAT_R16G16B16A16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16B16A16_TYPELESS => {
            DxgiCommon::DXGI_FORMAT_R16G16B16A16_TYPELESS
        }

        DxgiCommon::DXGI_FORMAT_R16G16_FLOAT => DxgiCommon::DXGI_FORMAT_R16G16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16_UNORM => DxgiCommon::DXGI_FORMAT_R16G16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16_UINT => DxgiCommon::DXGI_FORMAT_R16G16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16_SNORM => DxgiCommon::DXGI_FORMAT_R16G16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16_SINT => DxgiCommon::DXGI_FORMAT_R16G16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16G16_TYPELESS => DxgiCommon::DXGI_FORMAT_R16G16_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R16_FLOAT => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16_UNORM => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16_UINT => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16_SNORM => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16_SINT => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_B4G4R4A4_UNORM => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_D16_UNORM => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_B5G6R5_UNORM => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_B5G5R5A1_UNORM => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R16_TYPELESS => DxgiCommon::DXGI_FORMAT_R16_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM => DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_UINT => DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_SNORM => DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_SINT => DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS => DxgiCommon::DXGI_FORMAT_R8G8B8A8_TYPELESS,

        DxgiCommon::DXGI_FORMAT_B8G8R8X8_UNORM => DxgiCommon::DXGI_FORMAT_B8G8R8X8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_B8G8R8X8_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_B8G8R8X8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_B8G8R8X8_TYPELESS => DxgiCommon::DXGI_FORMAT_B8G8R8X8_TYPELESS,

        DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM => DxgiCommon::DXGI_FORMAT_B8G8R8A8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_B8G8R8A8_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_B8G8R8A8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_B8G8R8A8_TYPELESS => DxgiCommon::DXGI_FORMAT_B8G8R8A8_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R8G8_UNORM => DxgiCommon::DXGI_FORMAT_R8G8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8_UINT => DxgiCommon::DXGI_FORMAT_R8G8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8_SNORM => DxgiCommon::DXGI_FORMAT_R8G8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8_SINT => DxgiCommon::DXGI_FORMAT_R8G8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8G8_TYPELESS => DxgiCommon::DXGI_FORMAT_R8G8_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R8_UNORM => DxgiCommon::DXGI_FORMAT_R8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8_UINT => DxgiCommon::DXGI_FORMAT_R8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8_SNORM => DxgiCommon::DXGI_FORMAT_R8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8_SINT => DxgiCommon::DXGI_FORMAT_R8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_A8_UNORM => DxgiCommon::DXGI_FORMAT_R8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R8_TYPELESS => DxgiCommon::DXGI_FORMAT_R8_TYPELESS,

        DxgiCommon::DXGI_FORMAT_R10G10B10_XR_BIAS_A2_UNORM => {
            DxgiCommon::DXGI_FORMAT_R10G10B10A2_TYPELESS
        }
        //DxgiCommon::DXGI_FORMAT_R10G10B10_SNORM_A2_UNORM
        DxgiCommon::DXGI_FORMAT_R10G10B10A2_UNORM => DxgiCommon::DXGI_FORMAT_R10G10B10A2_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R10G10B10A2_UINT => DxgiCommon::DXGI_FORMAT_R10G10B10A2_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R10G10B10A2_TYPELESS => {
            DxgiCommon::DXGI_FORMAT_R10G10B10A2_TYPELESS
        }

        DxgiCommon::DXGI_FORMAT_BC1_UNORM => DxgiCommon::DXGI_FORMAT_BC1_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC1_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_BC1_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC1_TYPELESS => DxgiCommon::DXGI_FORMAT_BC1_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC2_UNORM => DxgiCommon::DXGI_FORMAT_BC2_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC2_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_BC2_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC2_TYPELESS => DxgiCommon::DXGI_FORMAT_BC2_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC3_UNORM => DxgiCommon::DXGI_FORMAT_BC3_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC3_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_BC3_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC3_TYPELESS => DxgiCommon::DXGI_FORMAT_BC3_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC4_UNORM => DxgiCommon::DXGI_FORMAT_BC4_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC4_SNORM => DxgiCommon::DXGI_FORMAT_BC4_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC4_TYPELESS => DxgiCommon::DXGI_FORMAT_BC4_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC5_UNORM => DxgiCommon::DXGI_FORMAT_BC5_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC5_SNORM => DxgiCommon::DXGI_FORMAT_BC5_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC5_TYPELESS => DxgiCommon::DXGI_FORMAT_BC5_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC6H_UF16 => DxgiCommon::DXGI_FORMAT_BC6H_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC6H_SF16 => DxgiCommon::DXGI_FORMAT_BC6H_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC6H_TYPELESS => DxgiCommon::DXGI_FORMAT_BC6H_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC7_UNORM => DxgiCommon::DXGI_FORMAT_BC7_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC7_UNORM_SRGB => DxgiCommon::DXGI_FORMAT_BC7_TYPELESS,
        DxgiCommon::DXGI_FORMAT_BC7_TYPELESS => DxgiCommon::DXGI_FORMAT_BC7_TYPELESS,

        DxgiCommon::DXGI_FORMAT_D24_UNORM_S8_UINT => DxgiCommon::DXGI_FORMAT_R24G8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_X24_TYPELESS_G8_UINT => DxgiCommon::DXGI_FORMAT_R24G8_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R24G8_TYPELESS => DxgiCommon::DXGI_FORMAT_R24G8_TYPELESS,

        DxgiCommon::DXGI_FORMAT_X32_TYPELESS_G8X24_UINT => {
            DxgiCommon::DXGI_FORMAT_R32G8X24_TYPELESS
        }
        DxgiCommon::DXGI_FORMAT_D32_FLOAT_S8X24_UINT => DxgiCommon::DXGI_FORMAT_R32G8X24_TYPELESS,
        DxgiCommon::DXGI_FORMAT_R32G8X24_TYPELESS => DxgiCommon::DXGI_FORMAT_R32G8X24_TYPELESS,

        _ => unimplemented!(),
    }

    /*




        case TIF_DXGI_FORMAT_X32_TYPELESS_G8X24_UINT:
        case TIF_DXGI_FORMAT_D32_FLOAT_S8X24_UINT: return TIF_DXGI_FORMAT_R32G8X24_TYPELESS;


        }
        return TIF_DXGI_FORMAT_UNKNOWN;
    }
         */
}
