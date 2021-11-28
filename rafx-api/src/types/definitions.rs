use super::*;
use crate::{RafxRootSignature, RafxSampler, RafxShader, RafxShaderModule};
use rafx_base::DecimalF32;
use std::hash::{Hash, Hasher};

use fnv::FnvHasher;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// General configuration that all APIs will make best effort to respect
#[derive(Default)]
pub struct RafxApiDef {
    // Don't have anything that's universal across APIs to add here yet
}

#[derive(Clone, Debug, Default)]
pub struct RafxBufferElementData {
    // For storage buffers
    pub element_begin_index: u64,
    pub element_count: u64,
    pub element_stride: u64,
}

/// Used to create a `RafxBuffer`
#[derive(Clone, Debug)]
pub struct RafxBufferDef {
    pub size: u64,
    pub alignment: u32, // May be 0
    pub memory_usage: RafxMemoryUsage,
    pub queue_type: RafxQueueType,
    pub resource_type: RafxResourceType,
    pub always_mapped: bool,

    // Set to undefined unless texture/typed buffer
    pub format: RafxFormat,

    // For storage buffers
    pub elements: RafxBufferElementData,
}

impl Default for RafxBufferDef {
    fn default() -> Self {
        RafxBufferDef {
            size: 0,
            alignment: 0,
            memory_usage: RafxMemoryUsage::Unknown,
            queue_type: RafxQueueType::Graphics,
            resource_type: RafxResourceType::UNDEFINED,
            elements: Default::default(),
            format: RafxFormat::UNDEFINED,
            always_mapped: false,
        }
    }
}

impl RafxBufferDef {
    pub fn verify(&self) {
        assert_ne!(self.size, 0);
    }

    pub fn for_staging_buffer(
        size: usize,
        resource_type: RafxResourceType,
    ) -> RafxBufferDef {
        RafxBufferDef {
            size: size as u64,
            alignment: 0,
            memory_usage: RafxMemoryUsage::CpuToGpu,
            queue_type: RafxQueueType::Graphics,
            resource_type,
            elements: Default::default(),
            format: RafxFormat::UNDEFINED,
            always_mapped: false,
        }
    }

    pub fn for_staging_buffer_data<T: Copy>(
        data: &[T],
        resource_type: RafxResourceType,
    ) -> RafxBufferDef {
        Self::for_staging_buffer(rafx_base::memory::slice_size_in_bytes(data), resource_type)
    }

    pub fn for_staging_vertex_buffer(size: usize) -> RafxBufferDef {
        Self::for_staging_buffer(size, RafxResourceType::VERTEX_BUFFER)
    }

    pub fn for_staging_vertex_buffer_data<T: Copy>(data: &[T]) -> RafxBufferDef {
        Self::for_staging_buffer_data(data, RafxResourceType::VERTEX_BUFFER)
    }

    pub fn for_staging_index_buffer(size: usize) -> RafxBufferDef {
        Self::for_staging_buffer(size, RafxResourceType::INDEX_BUFFER)
    }

    pub fn for_staging_index_buffer_data<T: Copy>(data: &[T]) -> RafxBufferDef {
        Self::for_staging_buffer_data(data, RafxResourceType::INDEX_BUFFER)
    }

    pub fn for_staging_uniform_buffer(size: usize) -> RafxBufferDef {
        Self::for_staging_buffer(size, RafxResourceType::UNIFORM_BUFFER)
    }

    pub fn for_staging_uniform_buffer_data<T: Copy>(data: &[T]) -> RafxBufferDef {
        Self::for_staging_buffer_data(data, RafxResourceType::UNIFORM_BUFFER)
    }
}

/// Determines how many dimensions the texture will have.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RafxTextureDimensions {
    /// Assume 2D if depth = 1, otherwise 3d
    Auto,
    Dim1D,
    Dim2D,
    Dim3D,
}

impl Default for RafxTextureDimensions {
    fn default() -> Self {
        RafxTextureDimensions::Auto
    }
}

impl RafxTextureDimensions {
    pub fn determine_dimensions(
        self,
        extents: RafxExtents3D,
    ) -> RafxTextureDimensions {
        match self {
            RafxTextureDimensions::Auto => {
                if extents.depth > 1 {
                    RafxTextureDimensions::Dim3D
                } else {
                    RafxTextureDimensions::Dim2D
                }
            }
            RafxTextureDimensions::Dim1D => {
                assert_eq!(extents.height, 1);
                assert_eq!(extents.depth, 1);
                RafxTextureDimensions::Dim1D
            }
            RafxTextureDimensions::Dim2D => {
                assert_eq!(extents.depth, 1);
                RafxTextureDimensions::Dim2D
            }
            RafxTextureDimensions::Dim3D => RafxTextureDimensions::Dim3D,
        }
    }
}

/// Used to create a `RafxTexture`
#[derive(Clone, Debug)]
pub struct RafxTextureDef {
    pub extents: RafxExtents3D,
    // Corresponds to number of vulkan layers, metal array length, and dx12 array size. Generally
    // should be 1, except set to 6 for cubemaps
    pub array_length: u32,
    pub mip_count: u32,
    pub sample_count: RafxSampleCount,
    pub format: RafxFormat,
    pub resource_type: RafxResourceType,
    // descriptors?
    // pointer to image?
    pub dimensions: RafxTextureDimensions,
}

impl Default for RafxTextureDef {
    fn default() -> Self {
        RafxTextureDef {
            extents: RafxExtents3D {
                width: 0,
                height: 0,
                depth: 0,
            },
            array_length: 1,
            mip_count: 1,
            sample_count: RafxSampleCount::SampleCount1,
            format: RafxFormat::UNDEFINED,
            resource_type: RafxResourceType::TEXTURE,
            dimensions: RafxTextureDimensions::Auto,
        }
    }
}

impl RafxTextureDef {
    pub fn verify(&self) {
        assert!(self.extents.width > 0);
        assert!(self.extents.height > 0);
        assert!(self.extents.depth > 0);
        assert!(self.array_length > 0);
        assert!(self.mip_count > 0);
        assert!(self.mip_count < 2 || self.sample_count == RafxSampleCount::SampleCount1);

        if self.resource_type.contains(RafxResourceType::TEXTURE_CUBE) {
            assert_eq!(self.array_length % 6, 0);
        }

        // we support only one or the other
        assert!(
            !(self.resource_type.contains(
                RafxResourceType::RENDER_TARGET_ARRAY_SLICES
                    | RafxResourceType::RENDER_TARGET_DEPTH_SLICES
            ))
        );

        assert!(
            !(self.format.has_depth()
                && self
                    .resource_type
                    .intersects(RafxResourceType::TEXTURE_READ_WRITE)),
            "Cannot use depth stencil as UAV"
        );
    }
}

/// Used to create a `RafxCommandPool`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RafxCommandPoolDef {
    /// Set to true if the command buffers allocated from the pool are expected to have very short
    /// lifetimes
    pub transient: bool,
}

/// Used to create a `RafxCommandBuffer`
#[derive(Debug, Clone, PartialEq)]
pub struct RafxCommandBufferDef {
    /// Secondary command buffers are used to encode a single pass on multiple threads
    pub is_secondary: bool,
}

/// Used to create a `RafxSwapchain`
#[derive(Clone, Debug)]
pub struct RafxSwapchainDef {
    pub width: u32,
    pub height: u32,
    pub enable_vsync: bool,
    // image count?
}

/// Describes a single stage within a shader
#[derive(Clone, Debug)]
pub struct RafxShaderStageDef {
    pub shader_module: RafxShaderModule,
    pub reflection: RafxShaderStageReflection,
}

impl RafxShaderStageDef {
    pub fn hash_definition<HasherT: std::hash::Hasher, ShaderModuleHashT: Hash>(
        hasher: &mut HasherT,
        reflection_data: &[&RafxShaderStageReflection],
        shader_module_hashes: &[ShaderModuleHashT],
    ) {
        assert_eq!(reflection_data.len(), shader_module_hashes.len());
        fn hash_stage<HasherT: std::hash::Hasher, ShaderModuleHashT: Hash>(
            hasher: &mut HasherT,
            stage_flag: RafxShaderStageFlags,
            reflection_data: &[&RafxShaderStageReflection],
            shader_module_hashes: &[ShaderModuleHashT],
        ) {
            for (reflection, shader_module_hash) in reflection_data.iter().zip(shader_module_hashes)
            {
                if reflection.shader_stage.intersects(stage_flag) {
                    reflection.shader_stage.hash(hasher);
                    reflection.entry_point_name.hash(hasher);
                    reflection.resources.hash(hasher);
                    shader_module_hash.hash(hasher);
                    break;
                }
            }
        }

        // Hash stages in a deterministic order
        for stage_flag in &crate::ALL_SHADER_STAGE_FLAGS {
            hash_stage(hasher, *stage_flag, reflection_data, shader_module_hashes);
        }
    }
}

/// Indicates which immutable sampler is being set
#[derive(Clone, Hash, Debug)]
pub enum RafxImmutableSamplerKey<'a> {
    Name(&'a str),
    Binding(u32, u32),
}

impl<'a> RafxImmutableSamplerKey<'a> {
    pub fn from_name(name: &'a str) -> RafxImmutableSamplerKey<'a> {
        RafxImmutableSamplerKey::Name(name)
    }

    pub fn from_binding(
        set_index: u32,
        binding: u32,
    ) -> RafxImmutableSamplerKey<'a> {
        RafxImmutableSamplerKey::Binding(set_index, binding)
    }
}

/// Describes an immutable sampler key/value pair
#[derive(Debug)]
pub struct RafxImmutableSamplers<'a> {
    pub key: RafxImmutableSamplerKey<'a>,
    pub samplers: &'a [RafxSampler],
}

impl<'a> RafxImmutableSamplers<'a> {
    pub fn from_name(
        name: &'a str,
        samplers: &'a [RafxSampler],
    ) -> RafxImmutableSamplers<'a> {
        RafxImmutableSamplers {
            key: RafxImmutableSamplerKey::from_name(name),
            samplers,
        }
    }

    pub fn from_binding(
        set_index: u32,
        binding: u32,
        samplers: &'a [RafxSampler],
    ) -> RafxImmutableSamplers<'a> {
        RafxImmutableSamplers {
            key: RafxImmutableSamplerKey::from_binding(set_index, binding),
            samplers,
        }
    }
}

/// Used to create a `RafxRootSignature`
pub struct RafxRootSignatureDef<'a> {
    pub shaders: &'a [RafxShader],
    pub immutable_samplers: &'a [RafxImmutableSamplers<'a>],
}

impl<'a> RafxRootSignatureDef<'a> {
    // The current implementation here is minimal. It will produce different hash values for
    // shader orderings and immutable samplers.
    pub fn hash_definition<
        HasherT: std::hash::Hasher,
        ShaderHashT: Hash,
        ImmutableSamplerHashT: Hash,
    >(
        hasher: &mut HasherT,
        shader_hashes: &[ShaderHashT],
        immutable_sampler_keys: &[RafxImmutableSamplerKey],
        immutable_sampler_hashes: &[Vec<ImmutableSamplerHashT>],
    ) {
        // Hash all the shader hashes and xor them together, this keeps them order-independent
        let mut combined_shaders_hash = 0;
        for shader_hash in shader_hashes {
            let mut h = FnvHasher::default();
            shader_hash.hash(&mut h);
            combined_shaders_hash ^= h.finish();
        }

        // Hash all the sampler key/value pairs and xor them together, this keeps them
        // order-independent
        let mut combined_immutable_samplers_hash = 0;
        for (key, samplers) in immutable_sampler_keys.iter().zip(immutable_sampler_hashes) {
            let mut h = FnvHasher::default();
            key.hash(&mut h);
            samplers.hash(&mut h);
            combined_immutable_samplers_hash ^= h.finish();
        }

        // Hash both combined hashes to produce the final hash
        combined_shaders_hash.hash(hasher);
        combined_immutable_samplers_hash.hash(hasher);
    }
}

/// Used to create a `RafxSampler`
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxSamplerDef {
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub min_filter: RafxFilterType,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub mag_filter: RafxFilterType,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub mip_map_mode: RafxMipMapMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub address_mode_u: RafxAddressMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub address_mode_v: RafxAddressMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub address_mode_w: RafxAddressMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub mip_lod_bias: f32,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub max_anisotropy: f32,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub compare_op: RafxCompareOp,
    //NOTE: Custom hash impl, don't forget to add changes there too!
}

impl Eq for RafxSamplerDef {}

impl Hash for RafxSamplerDef {
    fn hash<H: Hasher>(
        &self,
        mut state: &mut H,
    ) {
        self.min_filter.hash(&mut state);
        self.mag_filter.hash(&mut state);
        self.mip_map_mode.hash(&mut state);
        self.address_mode_u.hash(&mut state);
        self.address_mode_v.hash(&mut state);
        self.address_mode_w.hash(&mut state);
        DecimalF32(self.mip_lod_bias).hash(&mut state);
        DecimalF32(self.max_anisotropy).hash(&mut state);
        self.compare_op.hash(&mut state);
    }
}

/// Describes an attribute within a RafxVertexLayout
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RafxVertexLayoutAttribute {
    /// Format of the attribute
    pub format: RafxFormat,
    /// Which buffer the attribute is contained in
    pub buffer_index: u32,
    /// Affects what input variable within the shader the attribute is assigned
    pub location: u32,
    /// The byte offset of the attribute within the buffer
    pub byte_offset: u32,

    /// name of the attribute in the shader, only required for GL
    pub gl_attribute_name: Option<String>,
}

/// Describes a buffer that provides vertex attribute data (See RafxVertexLayout)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RafxVertexLayoutBuffer {
    pub stride: u32,
    pub rate: RafxVertexAttributeRate,
}

/// Describes how vertex attributes are laid out within one or more buffers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RafxVertexLayout {
    pub attributes: Vec<RafxVertexLayoutAttribute>,
    pub buffers: Vec<RafxVertexLayoutBuffer>,
}

/// Affects depth testing and stencil usage. Commonly used to enable "Z-buffering".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxDepthState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: RafxCompareOp,
    pub stencil_test_enable: bool,
    pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
    pub front_depth_fail_op: RafxStencilOp,
    pub front_stencil_compare_op: RafxCompareOp,
    pub front_stencil_fail_op: RafxStencilOp,
    pub front_stencil_pass_op: RafxStencilOp,
    pub back_depth_fail_op: RafxStencilOp,
    pub back_stencil_compare_op: RafxCompareOp,
    pub back_stencil_fail_op: RafxStencilOp,
    pub back_stencil_pass_op: RafxStencilOp,
}

impl Default for RafxDepthState {
    fn default() -> Self {
        RafxDepthState {
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: RafxCompareOp::LessOrEqual,
            stencil_test_enable: false,
            stencil_read_mask: 0xFF,
            stencil_write_mask: 0xFF,
            front_depth_fail_op: Default::default(),
            front_stencil_compare_op: RafxCompareOp::Always,
            front_stencil_fail_op: Default::default(),
            front_stencil_pass_op: Default::default(),
            back_depth_fail_op: Default::default(),
            back_stencil_compare_op: RafxCompareOp::Always,
            back_stencil_fail_op: Default::default(),
            back_stencil_pass_op: Default::default(),
        }
    }
}

/// Affects rasterization, commonly used to enable backface culling or wireframe rendering
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxRasterizerState {
    pub cull_mode: RafxCullMode,
    pub front_face: RafxFrontFace,
    pub fill_mode: RafxFillMode,
    pub depth_bias: i32,
    pub depth_bias_slope_scaled: f32,
    pub depth_clamp_enable: bool,
    pub multisample: bool,
    pub scissor: bool,
    // Hash implemented manually below, don't forget to update it!
}

impl Eq for RafxRasterizerState {}

impl Hash for RafxRasterizerState {
    fn hash<H: Hasher>(
        &self,
        mut state: &mut H,
    ) {
        self.cull_mode.hash(&mut state);
        self.front_face.hash(&mut state);
        self.fill_mode.hash(&mut state);
        self.depth_bias.hash(&mut state);
        DecimalF32(self.depth_bias_slope_scaled).hash(&mut state);
        self.depth_clamp_enable.hash(&mut state);
        self.multisample.hash(&mut state);
        self.scissor.hash(&mut state);
    }
}

impl Default for RafxRasterizerState {
    fn default() -> Self {
        RafxRasterizerState {
            cull_mode: RafxCullMode::None,
            front_face: Default::default(),
            fill_mode: Default::default(),
            depth_bias: 0,
            depth_bias_slope_scaled: 0.0,
            depth_clamp_enable: false,
            multisample: false,
            scissor: false,
        }
    }
}

/// Configures blend state for a particular render target
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxBlendStateRenderTarget {
    pub src_factor: RafxBlendFactor,
    pub dst_factor: RafxBlendFactor,
    pub src_factor_alpha: RafxBlendFactor,
    pub dst_factor_alpha: RafxBlendFactor,
    pub blend_op: RafxBlendOp,
    pub blend_op_alpha: RafxBlendOp,
    pub masks: RafxColorFlags,
}

impl Default for RafxBlendStateRenderTarget {
    fn default() -> Self {
        RafxBlendStateRenderTarget {
            blend_op: RafxBlendOp::Add,
            blend_op_alpha: RafxBlendOp::Add,
            src_factor: RafxBlendFactor::One,
            src_factor_alpha: RafxBlendFactor::One,
            dst_factor: RafxBlendFactor::Zero,
            dst_factor_alpha: RafxBlendFactor::Zero,
            masks: RafxColorFlags::ALL,
        }
    }
}

impl RafxBlendStateRenderTarget {
    pub fn default_alpha_disabled() -> Self {
        Default::default()
    }

    pub fn default_alpha_enabled() -> Self {
        RafxBlendStateRenderTarget {
            src_factor: RafxBlendFactor::SrcAlpha,
            dst_factor: RafxBlendFactor::OneMinusSrcAlpha,
            src_factor_alpha: RafxBlendFactor::One,
            dst_factor_alpha: RafxBlendFactor::Zero,
            blend_op: RafxBlendOp::Add,
            blend_op_alpha: RafxBlendOp::Add,
            masks: RafxColorFlags::ALL,
        }
    }
}

impl RafxBlendStateRenderTarget {
    pub fn blend_enabled(&self) -> bool {
        self.src_factor != RafxBlendFactor::One
            || self.src_factor_alpha != RafxBlendFactor::One
            || self.dst_factor != RafxBlendFactor::Zero
            || self.dst_factor_alpha != RafxBlendFactor::Zero
    }
}

/// Affects the way the result of a pixel shader is blended with a value it will overwrite. Commonly
/// used to enable "alpha-blending".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxBlendState {
    /// Individual blend states for blend targets
    pub render_target_blend_states: Vec<RafxBlendStateRenderTarget>,

    /// Indicates which blend targets to affect. Blend targets with unset bits are left in default
    /// state.
    pub render_target_mask: RafxBlendStateTargets,

    /// If false, `render_target_blend_states[0]` will apply to all render targets indicated by
    /// `render_target_mask`. If true, we index into `render_target_blend_states` based on the
    /// render target's index.
    pub independent_blend: bool,
}

impl RafxBlendState {
    pub fn default_alpha_disabled() -> Self {
        RafxBlendState {
            render_target_blend_states: vec![RafxBlendStateRenderTarget::default_alpha_disabled()],
            render_target_mask: RafxBlendStateTargets::BLEND_STATE_TARGET_ALL,
            independent_blend: false,
        }
    }

    pub fn default_alpha_enabled() -> Self {
        RafxBlendState {
            render_target_blend_states: vec![RafxBlendStateRenderTarget::default_alpha_enabled()],
            render_target_mask: RafxBlendStateTargets::BLEND_STATE_TARGET_ALL,
            independent_blend: false,
        }
    }
}

impl Default for RafxBlendState {
    fn default() -> Self {
        Self::default_alpha_disabled()
    }
}

impl RafxBlendState {
    pub fn verify(
        &self,
        color_attachment_count: usize,
    ) {
        if !self.independent_blend {
            assert_eq!(self.render_target_blend_states.len(), 1, "If RafxBlendState::independent_blend is false, RafxBlendState::render_target_blend_states must be 1");
        } else {
            assert_eq!(self.render_target_blend_states.len(), color_attachment_count, "If RafxBlendState::independent_blend is true, RafxBlendState::render_target_blend_states length must match color attachment count");
        }
    }
}

/// Used to create a `RafxPipeline` for graphics operations
#[derive(Debug)]
pub struct RafxGraphicsPipelineDef<'a> {
    pub shader: &'a RafxShader,
    pub root_signature: &'a RafxRootSignature,
    pub vertex_layout: &'a RafxVertexLayout,
    pub blend_state: &'a RafxBlendState,
    pub depth_state: &'a RafxDepthState,
    pub rasterizer_state: &'a RafxRasterizerState,
    pub primitive_topology: RafxPrimitiveTopology,
    pub color_formats: &'a [RafxFormat],
    pub depth_stencil_format: Option<RafxFormat>,
    pub sample_count: RafxSampleCount,
    //indirect_commands_enable: bool
}

/// Used to create a `RafxPipeline` for compute operations
#[derive(Debug)]
pub struct RafxComputePipelineDef<'a> {
    pub shader: &'a RafxShader,
    pub root_signature: &'a RafxRootSignature,
}

/// Used to create a `RafxDescriptorSetArray`
pub struct RafxDescriptorSetArrayDef<'a> {
    /// The root signature the descriptor set will be based on
    pub root_signature: &'a RafxRootSignature,
    /// Which descriptor set to create the descriptor set array for
    pub set_index: u32,
    /// The number of descriptor sets in the array
    pub array_length: usize,
}
