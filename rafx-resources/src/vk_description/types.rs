use ash::vk;
use std::hash::Hasher;

use rafx_shell_vulkan::MsaaLevel;
use serde::{Deserialize, Serialize};

use bitflags::bitflags;
use enumflags2::BitFlags;

//TODO: Rename all this from description to definition

// This is an f32 that supports Hash and Eq. Generally this is dangerous, but here we're
// not doing any sort of fp-arithmetic and not expecting NaN. We should be deterministically
// parsing a string and creating a float from it. Representing as an f64 since this ensures
// all 32-bit whole numbers can be represented exactly. (anything <= 2^53)
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Decimal(pub f64);

impl Decimal {
    pub fn to_f32(&self) -> f32 {
        self.0 as f32
    }

    pub fn to_i32(&self) -> i32 {
        self.0 as i32
    }

    pub fn to_u32(&self) -> u32 {
        self.0 as u32
    }
}

impl PartialEq for Decimal {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.0 == other.0
    }
}

impl Eq for Decimal {}

impl std::hash::Hash for Decimal {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        let bits: u64 = self.0.to_bits();
        bits.hash(state);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageViewType {
    Type1D,
    Type2D,
    Type3D,
    Cube,
    Type1DArray,
    Type2DArray,
    CubeArray,
}

impl Default for ImageViewType {
    fn default() -> Self {
        ImageViewType::Type2D
    }
}

impl Into<vk::ImageViewType> for ImageViewType {
    fn into(self) -> vk::ImageViewType {
        match self {
            ImageViewType::Type1D => vk::ImageViewType::TYPE_1D,
            ImageViewType::Type2D => vk::ImageViewType::TYPE_2D,
            ImageViewType::Type3D => vk::ImageViewType::TYPE_3D,
            ImageViewType::Cube => vk::ImageViewType::CUBE,
            ImageViewType::Type1DArray => vk::ImageViewType::TYPE_1D_ARRAY,
            ImageViewType::Type2DArray => vk::ImageViewType::TYPE_2D_ARRAY,
            ImageViewType::CubeArray => vk::ImageViewType::CUBE_ARRAY,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentSwizzle {
    Identity,
    Zero,
    One,
    R,
    G,
    B,
    A,
}

impl Into<vk::ComponentSwizzle> for ComponentSwizzle {
    fn into(self) -> vk::ComponentSwizzle {
        match self {
            ComponentSwizzle::Identity => vk::ComponentSwizzle::IDENTITY,
            ComponentSwizzle::Zero => vk::ComponentSwizzle::ZERO,
            ComponentSwizzle::One => vk::ComponentSwizzle::ONE,
            ComponentSwizzle::R => vk::ComponentSwizzle::R,
            ComponentSwizzle::G => vk::ComponentSwizzle::G,
            ComponentSwizzle::B => vk::ComponentSwizzle::B,
            ComponentSwizzle::A => vk::ComponentSwizzle::A,
        }
    }
}

impl Default for ComponentSwizzle {
    fn default() -> Self {
        ComponentSwizzle::Identity
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ComponentMapping {
    pub r: ComponentSwizzle,
    pub g: ComponentSwizzle,
    pub b: ComponentSwizzle,
    pub a: ComponentSwizzle,
}

impl Into<vk::ComponentMapping> for ComponentMapping {
    fn into(self) -> vk::ComponentMapping {
        vk::ComponentMapping::builder()
            .r(self.r.into())
            .g(self.g.into())
            .b(self.b.into())
            .a(self.a.into())
            .build()
    }
}

#[derive(BitFlags, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum ImageAspectFlag {
    Color = 1,
    Depth = 2,
    Stencil = 4,
    Metadata = 8,
}

pub type ImageAspectFlags = BitFlags<ImageAspectFlag>;

impl ImageAspectFlag {
    pub fn from_vk_image_aspect_flags(vk_flags: vk::ImageAspectFlags) -> ImageAspectFlags {
        let mut flags = ImageAspectFlags::empty();
        if vk_flags.contains(vk::ImageAspectFlags::COLOR) {
            flags |= ImageAspectFlag::Color;
        }
        if vk_flags.contains(vk::ImageAspectFlags::DEPTH) {
            flags |= ImageAspectFlag::Depth;
        }
        if vk_flags.contains(vk::ImageAspectFlags::STENCIL) {
            flags |= ImageAspectFlag::Stencil;
        }
        if vk_flags.contains(vk::ImageAspectFlags::METADATA) {
            flags |= ImageAspectFlag::Metadata;
        }
        flags
    }
}

impl Default for ImageAspectFlag {
    fn default() -> Self {
        ImageAspectFlag::Color
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageSubresourceRange {
    pub aspect_mask: ImageAspectFlags,
    pub base_mip_level: u32,
    pub level_count: u32,
    pub base_array_layer: u32,
    pub layer_count: u32,
}

impl Into<vk::ImageSubresourceRange> for ImageSubresourceRange {
    fn into(self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::from_raw(self.aspect_mask.bits()))
            .base_mip_level(self.base_mip_level)
            .level_count(self.level_count)
            .base_array_layer(self.base_array_layer)
            .layer_count(self.layer_count)
            .build()
    }
}

impl ImageSubresourceRange {
    pub fn default_no_mips_no_layers(aspect_mask: ImageAspectFlags) -> Self {
        ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    pub fn default_no_mips_single_layer(
        aspect_mask: ImageAspectFlags,
        layer: u32,
    ) -> Self {
        ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: layer,
            layer_count: 1,
        }
    }

    pub fn default_all_mips_all_layers(
        aspect_mask: ImageAspectFlags,
        mip_count: u32,
        layer_count: u32,
    ) -> Self {
        ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: mip_count,
            base_array_layer: 0,
            layer_count: layer_count,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

impl Into<vk::CompareOp> for CompareOp {
    fn into(self) -> vk::CompareOp {
        match self {
            CompareOp::Never => vk::CompareOp::NEVER,
            CompareOp::Less => vk::CompareOp::LESS,
            CompareOp::Equal => vk::CompareOp::EQUAL,
            CompareOp::LessOrEqual => vk::CompareOp::LESS_OR_EQUAL,
            CompareOp::Greater => vk::CompareOp::GREATER,
            CompareOp::NotEqual => vk::CompareOp::NOT_EQUAL,
            CompareOp::GreaterOrEqual => vk::CompareOp::GREATER_OR_EQUAL,
            CompareOp::Always => vk::CompareOp::ALWAYS,
        }
    }
}

impl Default for CompareOp {
    fn default() -> Self {
        CompareOp::Never
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BorderColor {
    FloatTransparentBlack,
    IntTransparentBlack,
    FloatOpaqueBlack,
    IntOpaqueBlack,
    FloatOpaqueWhite,
    IntOpaqueWhite,
}

impl Into<vk::BorderColor> for BorderColor {
    fn into(self) -> vk::BorderColor {
        match self {
            BorderColor::FloatTransparentBlack => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            BorderColor::IntTransparentBlack => vk::BorderColor::INT_TRANSPARENT_BLACK,
            BorderColor::FloatOpaqueBlack => vk::BorderColor::FLOAT_OPAQUE_BLACK,
            BorderColor::IntOpaqueBlack => vk::BorderColor::INT_OPAQUE_BLACK,
            BorderColor::FloatOpaqueWhite => vk::BorderColor::FLOAT_OPAQUE_WHITE,
            BorderColor::IntOpaqueWhite => vk::BorderColor::INT_OPAQUE_WHITE,
        }
    }
}

impl Default for BorderColor {
    fn default() -> Self {
        BorderColor::FloatTransparentBlack
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamplerAddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

impl Into<vk::SamplerAddressMode> for SamplerAddressMode {
    fn into(self) -> vk::SamplerAddressMode {
        match self {
            SamplerAddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            SamplerAddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            SamplerAddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            SamplerAddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }
}

impl Default for SamplerAddressMode {
    fn default() -> Self {
        SamplerAddressMode::Repeat
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamplerMipmapMode {
    Nearest,
    Linear,
}

impl Into<vk::SamplerMipmapMode> for SamplerMipmapMode {
    fn into(self) -> vk::SamplerMipmapMode {
        match self {
            SamplerMipmapMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            SamplerMipmapMode::Linear => vk::SamplerMipmapMode::LINEAR,
        }
    }
}

impl Default for SamplerMipmapMode {
    fn default() -> Self {
        SamplerMipmapMode::Nearest
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Filter {
    Nearest,
    Linear,
}

impl Into<vk::Filter> for Filter {
    fn into(self) -> vk::Filter {
        match self {
            Filter::Nearest => vk::Filter::NEAREST,
            Filter::Linear => vk::Filter::LINEAR,
        }
    }
}

impl Default for Filter {
    fn default() -> Self {
        Filter::Nearest
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sampler {
    pub mag_filter: Filter,
    pub min_filter: Filter,
    pub mipmap_mode: SamplerMipmapMode,
    pub address_mode_u: SamplerAddressMode,
    pub address_mode_v: SamplerAddressMode,
    pub address_mode_w: SamplerAddressMode,
    pub mip_lod_bias: Decimal,
    pub anisotropy_enable: bool,
    pub max_anisotropy: Decimal,
    pub compare_enable: bool,
    pub compare_op: CompareOp,
    pub min_lod: Decimal,
    pub max_lod: Decimal,
    pub border_color: BorderColor,
    pub unnormalized_coordinates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageViewMeta {
    // Actual image excluded from meta
    //pub image: Image,
    pub view_type: ImageViewType,
    pub format: Format,
    pub components: ComponentMapping,
    pub subresource_range: ImageSubresourceRange,
}

impl ImageViewMeta {
    pub fn as_builder(
        &self,
        image: vk::Image,
    ) -> vk::ImageViewCreateInfoBuilder {
        vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(self.view_type.into())
            .format(self.format.into())
            .components(self.components.clone().into())
            .subresource_range(self.subresource_range.clone().into())
    }

    pub fn default_2d_no_mips_or_layers(
        format: Format,
        image_aspect_flags: ImageAspectFlags,
    ) -> Self {
        ImageViewMeta {
            view_type: ImageViewType::Type2D,
            format,
            components: Default::default(),
            subresource_range: ImageSubresourceRange::default_no_mips_no_layers(image_aspect_flags),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    InputAttachment,
}

impl DescriptorType {
    pub fn count() -> usize {
        vk::DescriptorType::INPUT_ATTACHMENT.as_raw() as usize + 1
    }
}

impl Into<vk::DescriptorType> for DescriptorType {
    fn into(self) -> vk::DescriptorType {
        match self {
            DescriptorType::Sampler => vk::DescriptorType::SAMPLER,
            DescriptorType::CombinedImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            DescriptorType::SampledImage => vk::DescriptorType::SAMPLED_IMAGE,
            DescriptorType::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
            DescriptorType::UniformTexelBuffer => vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
            DescriptorType::StorageTexelBuffer => vk::DescriptorType::STORAGE_TEXEL_BUFFER,
            DescriptorType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
            DescriptorType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
            DescriptorType::UniformBufferDynamic => vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
            DescriptorType::StorageBufferDynamic => vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
            DescriptorType::InputAttachment => vk::DescriptorType::INPUT_ATTACHMENT,
        }
    }
}

impl Default for DescriptorType {
    fn default() -> Self {
        DescriptorType::Sampler
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderStage {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    Compute,
    AllGraphics,
    All,
}

impl Into<vk::ShaderStageFlags> for ShaderStage {
    fn into(self) -> vk::ShaderStageFlags {
        match self {
            ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
            ShaderStage::TessellationControl => vk::ShaderStageFlags::TESSELLATION_CONTROL,
            ShaderStage::TessellationEvaluation => vk::ShaderStageFlags::TESSELLATION_EVALUATION,
            ShaderStage::Geometry => vk::ShaderStageFlags::GEOMETRY,
            ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
            ShaderStage::Compute => vk::ShaderStageFlags::COMPUTE,
            ShaderStage::AllGraphics => vk::ShaderStageFlags::ALL_GRAPHICS,
            ShaderStage::All => vk::ShaderStageFlags::ALL,
        }
    }
}

impl Into<ShaderStageFlags> for ShaderStage {
    fn into(self) -> ShaderStageFlags {
        match self {
            ShaderStage::Vertex => ShaderStageFlags::VERTEX,
            ShaderStage::TessellationControl => ShaderStageFlags::TESSELLATION_CONTROL,
            ShaderStage::TessellationEvaluation => ShaderStageFlags::TESSELLATION_EVALUATION,
            ShaderStage::Geometry => ShaderStageFlags::GEOMETRY,
            ShaderStage::Fragment => ShaderStageFlags::FRAGMENT,
            ShaderStage::Compute => ShaderStageFlags::COMPUTE,
            ShaderStage::AllGraphics => ShaderStageFlags::ALL_GRAPHICS,
            ShaderStage::All => ShaderStageFlags::ALL,
        }
    }
}

impl Default for ShaderStage {
    fn default() -> Self {
        ShaderStage::Vertex
    }
}

crate::option_set! {
    pub struct ShaderStageFlags : u32 {
        const VERTEX = vk::ShaderStageFlags::VERTEX.as_raw();
        const TESSELLATION_CONTROL = vk::ShaderStageFlags::TESSELLATION_CONTROL.as_raw();
        const TESSELLATION_EVALUATION = vk::ShaderStageFlags::TESSELLATION_EVALUATION.as_raw();
        const GEOMETRY = vk::ShaderStageFlags::GEOMETRY.as_raw();
        const FRAGMENT = vk::ShaderStageFlags::FRAGMENT.as_raw();
        const COMPUTE = vk::ShaderStageFlags::COMPUTE.as_raw();
        const ALL_GRAPHICS = vk::ShaderStageFlags::ALL_GRAPHICS.as_raw();
        const ALL = vk::ShaderStageFlags::ALL.as_raw();
    }
}

impl Into<vk::ShaderStageFlags> for ShaderStageFlags {
    fn into(self) -> vk::ShaderStageFlags {
        vk::ShaderStageFlags::from_raw(self.bits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: ShaderStageFlags,
    pub immutable_samplers: Option<Vec<Sampler>>,

    // Used for descriptor sets, if this is non-zero we will allocate a buffer owned by the
    // descriptor set pool chunk, allowing materials to be used directly without worrying about
    // buffers.
    pub internal_buffer_per_descriptor_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayout {
    pub descriptor_set_layout_bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayout {
    pub fn new() -> Self {
        DescriptorSetLayout {
            descriptor_set_layout_bindings: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PushConstantRange {
    pub stage_flags: ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
}

impl PushConstantRange {
    pub fn as_builder(&self) -> vk::PushConstantRangeBuilder {
        vk::PushConstantRange::builder()
            .stage_flags(self.stage_flags.into())
            .offset(self.offset)
            .size(self.size)
    }
}

impl Into<vk::PushConstantRange> for PushConstantRange {
    fn into(self) -> vk::PushConstantRange {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineLayout {
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>,
    pub push_constant_ranges: Vec<PushConstantRange>,
}

impl PipelineLayout {
    pub fn new() -> Self {
        PipelineLayout {
            descriptor_set_layouts: Default::default(),
            push_constant_ranges: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentDescriptionFlags {
    None,
    MayAlias,
}

impl Into<vk::AttachmentDescriptionFlags> for AttachmentDescriptionFlags {
    fn into(self) -> vk::AttachmentDescriptionFlags {
        match self {
            AttachmentDescriptionFlags::None => vk::AttachmentDescriptionFlags::empty(),
            AttachmentDescriptionFlags::MayAlias => vk::AttachmentDescriptionFlags::MAY_ALIAS,
        }
    }
}

impl Default for AttachmentDescriptionFlags {
    fn default() -> Self {
        AttachmentDescriptionFlags::MayAlias
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SwapchainSurfaceInfo {
    pub extents: vk::Extent2D,
    pub msaa_level: MsaaLevel,
    pub surface_format: vk::SurfaceFormatKHR,
    pub color_format: vk::Format,
    pub depth_format: vk::Format,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SubpassInfo {
    pub surface_info: SwapchainSurfaceInfo,
    pub subpass_sample_count_flags: vk::SampleCountFlags,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SampleCountFlags {
    MatchSwapchain,
    SampleCount1,
    SampleCount2,
    SampleCount4,
    SampleCount8,
    SampleCount16,
    SampleCount32,
    SampleCount64,
}

impl SampleCountFlags {
    pub fn as_vk_sample_count_flags(
        &self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> vk::SampleCountFlags {
        match self {
            SampleCountFlags::MatchSwapchain => swapchain_surface_info.msaa_level.into(),
            SampleCountFlags::SampleCount1 => vk::SampleCountFlags::TYPE_1,
            SampleCountFlags::SampleCount2 => vk::SampleCountFlags::TYPE_2,
            SampleCountFlags::SampleCount4 => vk::SampleCountFlags::TYPE_4,
            SampleCountFlags::SampleCount8 => vk::SampleCountFlags::TYPE_8,
            SampleCountFlags::SampleCount16 => vk::SampleCountFlags::TYPE_16,
            SampleCountFlags::SampleCount32 => vk::SampleCountFlags::TYPE_32,
            SampleCountFlags::SampleCount64 => vk::SampleCountFlags::TYPE_64,
        }
    }

    pub fn from_vk_sample_count_flags(sample_count: vk::SampleCountFlags) -> Option<Self> {
        match sample_count {
            vk::SampleCountFlags::TYPE_1 => Some(SampleCountFlags::SampleCount1),
            vk::SampleCountFlags::TYPE_2 => Some(SampleCountFlags::SampleCount2),
            vk::SampleCountFlags::TYPE_4 => Some(SampleCountFlags::SampleCount4),
            vk::SampleCountFlags::TYPE_8 => Some(SampleCountFlags::SampleCount8),
            vk::SampleCountFlags::TYPE_16 => Some(SampleCountFlags::SampleCount16),
            vk::SampleCountFlags::TYPE_32 => Some(SampleCountFlags::SampleCount32),
            vk::SampleCountFlags::TYPE_64 => Some(SampleCountFlags::SampleCount64),
            _ => None,
        }
    }
}

impl Default for SampleCountFlags {
    fn default() -> Self {
        SampleCountFlags::SampleCount1
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineSampleCountFlags {
    MatchSwapchain,
    MatchSubpass,
    SampleCount1,
    SampleCount2,
    SampleCount4,
    SampleCount8,
    SampleCount16,
    SampleCount32,
    SampleCount64,
}

impl PipelineSampleCountFlags {
    pub fn as_vk_sample_count_flags(
        &self,
        subpass_info: &SubpassInfo,
    ) -> vk::SampleCountFlags {
        match self {
            PipelineSampleCountFlags::MatchSwapchain => subpass_info.surface_info.msaa_level.into(),
            PipelineSampleCountFlags::MatchSubpass => subpass_info.subpass_sample_count_flags,
            PipelineSampleCountFlags::SampleCount1 => vk::SampleCountFlags::TYPE_1,
            PipelineSampleCountFlags::SampleCount2 => vk::SampleCountFlags::TYPE_2,
            PipelineSampleCountFlags::SampleCount4 => vk::SampleCountFlags::TYPE_4,
            PipelineSampleCountFlags::SampleCount8 => vk::SampleCountFlags::TYPE_8,
            PipelineSampleCountFlags::SampleCount16 => vk::SampleCountFlags::TYPE_16,
            PipelineSampleCountFlags::SampleCount32 => vk::SampleCountFlags::TYPE_32,
            PipelineSampleCountFlags::SampleCount64 => vk::SampleCountFlags::TYPE_64,
        }
    }

    pub fn from_vk_sample_count_flags(sample_count: vk::SampleCountFlags) -> Option<Self> {
        match sample_count {
            vk::SampleCountFlags::TYPE_1 => Some(PipelineSampleCountFlags::SampleCount1),
            vk::SampleCountFlags::TYPE_2 => Some(PipelineSampleCountFlags::SampleCount2),
            vk::SampleCountFlags::TYPE_4 => Some(PipelineSampleCountFlags::SampleCount4),
            vk::SampleCountFlags::TYPE_8 => Some(PipelineSampleCountFlags::SampleCount8),
            vk::SampleCountFlags::TYPE_16 => Some(PipelineSampleCountFlags::SampleCount16),
            vk::SampleCountFlags::TYPE_32 => Some(PipelineSampleCountFlags::SampleCount32),
            vk::SampleCountFlags::TYPE_64 => Some(PipelineSampleCountFlags::SampleCount64),
            _ => None,
        }
    }
}

impl Default for PipelineSampleCountFlags {
    fn default() -> Self {
        PipelineSampleCountFlags::MatchSubpass
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentLoadOp {
    Load,
    Clear,
    DontCare,
}

impl Into<vk::AttachmentLoadOp> for AttachmentLoadOp {
    fn into(self) -> vk::AttachmentLoadOp {
        match self {
            AttachmentLoadOp::Load => vk::AttachmentLoadOp::LOAD,
            AttachmentLoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
            AttachmentLoadOp::DontCare => vk::AttachmentLoadOp::DONT_CARE,
        }
    }
}

impl From<vk::AttachmentLoadOp> for AttachmentLoadOp {
    fn from(other: vk::AttachmentLoadOp) -> AttachmentLoadOp {
        match other {
            vk::AttachmentLoadOp::LOAD => AttachmentLoadOp::Load,
            vk::AttachmentLoadOp::CLEAR => AttachmentLoadOp::Clear,
            vk::AttachmentLoadOp::DONT_CARE => AttachmentLoadOp::DontCare,
            _ => unimplemented!(),
        }
    }
}

impl Default for AttachmentLoadOp {
    fn default() -> Self {
        AttachmentLoadOp::Load
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

impl Into<vk::AttachmentStoreOp> for AttachmentStoreOp {
    fn into(self) -> vk::AttachmentStoreOp {
        match self {
            AttachmentStoreOp::Store => vk::AttachmentStoreOp::STORE,
            AttachmentStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
        }
    }
}

impl From<vk::AttachmentStoreOp> for AttachmentStoreOp {
    fn from(other: vk::AttachmentStoreOp) -> AttachmentStoreOp {
        match other {
            vk::AttachmentStoreOp::STORE => AttachmentStoreOp::Store,
            vk::AttachmentStoreOp::DONT_CARE => AttachmentStoreOp::DontCare,
            _ => unimplemented!(),
        }
    }
}

impl Default for AttachmentStoreOp {
    fn default() -> Self {
        AttachmentStoreOp::Store
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineBindPoint {
    Compute,
    Graphics,
}

impl Into<vk::PipelineBindPoint> for PipelineBindPoint {
    fn into(self) -> vk::PipelineBindPoint {
        match self {
            PipelineBindPoint::Compute => vk::PipelineBindPoint::COMPUTE,
            PipelineBindPoint::Graphics => vk::PipelineBindPoint::GRAPHICS,
        }
    }
}

impl Default for PipelineBindPoint {
    fn default() -> Self {
        PipelineBindPoint::Graphics
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageLayout {
    Undefined,
    General,
    ColorAttachmentOptimal,
    DepthStencilAttachmentOptimal,
    DepthStencilReadOnlyOptimal,
    ShaderReadOnlyOptimal,
    TransferSrcOptimal,
    TransferDstOptimal,
    Preinitialized,
    PresentSrcKhr,
    SharedPresentKhr,
    ShadingRateOptimal,
    FragmentDensityMapOptimalExt,
    DepthReadOnlyStencilAttachmentOptimal,
    DepthAttachmentStencilReadOnlyOptimal,
    DepthAttachmentOptimal,
    DepthReadOnlyOptimal,
    StencilAttachmentOptimal,
    StencilReadOnlyOptimal,
}

impl Into<vk::ImageLayout> for ImageLayout {
    fn into(self) -> vk::ImageLayout {
        match self {
            ImageLayout::Undefined => vk::ImageLayout::UNDEFINED,
            ImageLayout::General => vk::ImageLayout::GENERAL,
            ImageLayout::ColorAttachmentOptimal => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ImageLayout::DepthStencilAttachmentOptimal => {
                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
            }
            ImageLayout::DepthStencilReadOnlyOptimal => {
                vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL
            }
            ImageLayout::ShaderReadOnlyOptimal => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ImageLayout::TransferSrcOptimal => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            ImageLayout::TransferDstOptimal => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::Preinitialized => vk::ImageLayout::PREINITIALIZED,
            ImageLayout::PresentSrcKhr => vk::ImageLayout::PRESENT_SRC_KHR,
            ImageLayout::SharedPresentKhr => vk::ImageLayout::SHARED_PRESENT_KHR,
            ImageLayout::ShadingRateOptimal => vk::ImageLayout::SHADING_RATE_OPTIMAL_NV,
            ImageLayout::FragmentDensityMapOptimalExt => {
                vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT
            }
            ImageLayout::DepthReadOnlyStencilAttachmentOptimal => {
                vk::ImageLayout::DEPTH_READ_ONLY_STENCIL_ATTACHMENT_OPTIMAL
            }
            ImageLayout::DepthAttachmentStencilReadOnlyOptimal => {
                vk::ImageLayout::DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL
            }
            ImageLayout::DepthAttachmentOptimal => vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            ImageLayout::DepthReadOnlyOptimal => vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL,
            ImageLayout::StencilAttachmentOptimal => vk::ImageLayout::STENCIL_ATTACHMENT_OPTIMAL,
            ImageLayout::StencilReadOnlyOptimal => vk::ImageLayout::STENCIL_READ_ONLY_OPTIMAL,
        }
    }
}

impl From<vk::ImageLayout> for ImageLayout {
    fn from(layout: vk::ImageLayout) -> Self {
        match layout {
            vk::ImageLayout::UNDEFINED => ImageLayout::Undefined,
            vk::ImageLayout::GENERAL => ImageLayout::General,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => ImageLayout::ColorAttachmentOptimal,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                ImageLayout::DepthStencilAttachmentOptimal
            }
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL => {
                ImageLayout::DepthStencilReadOnlyOptimal
            }
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => ImageLayout::ShaderReadOnlyOptimal,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => ImageLayout::TransferSrcOptimal,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => ImageLayout::TransferDstOptimal,
            vk::ImageLayout::PREINITIALIZED => ImageLayout::Preinitialized,
            vk::ImageLayout::PRESENT_SRC_KHR => ImageLayout::PresentSrcKhr,
            vk::ImageLayout::SHARED_PRESENT_KHR => ImageLayout::SharedPresentKhr,
            vk::ImageLayout::SHADING_RATE_OPTIMAL_NV => ImageLayout::ShadingRateOptimal,
            vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT => {
                ImageLayout::FragmentDensityMapOptimalExt
            }
            vk::ImageLayout::DEPTH_READ_ONLY_STENCIL_ATTACHMENT_OPTIMAL => {
                ImageLayout::DepthReadOnlyStencilAttachmentOptimal
            }
            vk::ImageLayout::DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL => {
                ImageLayout::DepthAttachmentStencilReadOnlyOptimal
            }
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL => ImageLayout::DepthAttachmentOptimal,
            vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL => ImageLayout::DepthReadOnlyOptimal,
            vk::ImageLayout::STENCIL_ATTACHMENT_OPTIMAL => ImageLayout::StencilAttachmentOptimal,
            vk::ImageLayout::STENCIL_READ_ONLY_OPTIMAL => ImageLayout::StencilReadOnlyOptimal,
            _ => unimplemented!(),
        }
    }
}

impl Default for ImageLayout {
    fn default() -> Self {
        ImageLayout::Undefined
    }
}

crate::option_set! {
    pub struct PipelineStageFlags : u32 {
        const TOP_OF_PIPE = vk::PipelineStageFlags::TOP_OF_PIPE.as_raw();
        const DRAW_INDIRECT = vk::PipelineStageFlags::DRAW_INDIRECT.as_raw();
        const VERTEX_INPUT = vk::PipelineStageFlags::VERTEX_INPUT.as_raw();
        const VERTEX_SHADER = vk::PipelineStageFlags::VERTEX_SHADER.as_raw();
        const TESSELLATION_CONTROL_SHADER = vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER.as_raw();
        const TESSELLATION_EVALUATION_SHADER = vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER.as_raw();
        const GEOMETRY_SHADER = vk::PipelineStageFlags::GEOMETRY_SHADER.as_raw();
        const FRAGMENT_SHADER = vk::PipelineStageFlags::FRAGMENT_SHADER.as_raw();
        const EARLY_FRAGMENT_TESTS = vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS.as_raw();
        const LATE_FRAGMENT_TESTS = vk::PipelineStageFlags::LATE_FRAGMENT_TESTS.as_raw();
        const COLOR_ATTACHMENT_OUTPUT = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT.as_raw();
        const COMPUTE_SHADER = vk::PipelineStageFlags::COMPUTE_SHADER.as_raw();
        const TRANSFER = vk::PipelineStageFlags::TRANSFER.as_raw();
        const BOTTOM_OF_PIPE = vk::PipelineStageFlags::BOTTOM_OF_PIPE.as_raw();
        const HOST = vk::PipelineStageFlags::HOST.as_raw();
        const ALL_GRAPHICS = vk::PipelineStageFlags::ALL_GRAPHICS.as_raw();
        const ALL_COMMANDS = vk::PipelineStageFlags::ALL_COMMANDS.as_raw();
    }
}

// #[derive(BitFlags, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
// #[repr(u32)]
// pub enum PipelineStageFlag {
//     TopOfPipe = 0b1,
//     DrawIndirect = 0b10,
//     VertexInput = 0b100,
//     VertexShader = 0b1000,
//     TesselationControlShader = 0b1_0000,
//     TesselationEvaluationShader = 0b10_0000,
//     GeometryShader = 0b100_0000,
//     FragmentShader = 0b1000_0000,
//     EarlyFragmentTests = 0b1_0000_0000,
//     LateFragmentTests = 0b10_0000_0000,
//     ColorAttachmentOutput = 0b100_0000_0000,
//     ComputeShader = 0b1000_0000_0000,
//     Transfer = 0b1_0000_0000_0000,
//     BottomOfPipe = 0b10_0000_0000_0000,
//     Host = 0b100_0000_0000_0000,
//     AllGraphics = 0b1000_0000_0000_0000,
//     AllCommands = 0b1_0000_0000_0000_0000,
// }
//
// pub type PipelineStageFlags = BitFlags<PipelineStageFlag>;
/*
impl PipelineStageFlags {
    pub fn from_pipeline_stage_mask(flag_mask: vk::PipelineStageFlags) -> Vec<PipelineStageFlags> {
        let mut flags = Vec::default();
        if flag_mask.intersects(vk::PipelineStageFlags::TOP_OF_PIPE) {
            flags.push(PipelineStageFlags::TopOfPipe);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::DRAW_INDIRECT) {
            flags.push(PipelineStageFlags::DrawIndirect);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::VERTEX_INPUT) {
            flags.push(PipelineStageFlags::VertexInput);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::VERTEX_SHADER) {
            flags.push(PipelineStageFlags::VertexShader);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER) {
            flags.push(PipelineStageFlags::TesselationControlShader);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER) {
            flags.push(PipelineStageFlags::TesselationEvaluationShader);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::GEOMETRY_SHADER) {
            flags.push(PipelineStageFlags::GeometryShader);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::FRAGMENT_SHADER) {
            flags.push(PipelineStageFlags::FragmentShader);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS) {
            flags.push(PipelineStageFlags::EarlyFragmentTests);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::LATE_FRAGMENT_TESTS) {
            flags.push(PipelineStageFlags::LateFragmentTests);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT) {
            flags.push(PipelineStageFlags::ColorAttachmentOutput);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::COMPUTE_SHADER) {
            flags.push(PipelineStageFlags::ComputeShader);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::TRANSFER) {
            flags.push(PipelineStageFlags::Transfer);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::BOTTOM_OF_PIPE) {
            flags.push(PipelineStageFlags::BottomOfPipe);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::HOST) {
            flags.push(PipelineStageFlags::Host);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::ALL_GRAPHICS) {
            flags.push(PipelineStageFlags::AllGraphics);
        }
        if flag_mask.intersects(vk::PipelineStageFlags::ALL_COMMANDS) {
            flags.push(PipelineStageFlags::AllCommands);
        }
        flags
    }

    fn to_pipeline_stage_mask(flags: &[PipelineStageFlags]) -> vk::PipelineStageFlags {
        let mut flag_mask = vk::PipelineStageFlags::empty();
        for flag in flags {
            flag_mask |= (*flag).into();
        }
        flag_mask
    }
}

impl Into<vk::PipelineStageFlags> for PipelineStageFlags {
    fn into(self) -> vk::PipelineStageFlags {
        match self {
            PipelineStageFlags::TopOfPipe => vk::PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::DrawIndirect => vk::PipelineStageFlags::DRAW_INDIRECT,
            PipelineStageFlags::VertexInput => vk::PipelineStageFlags::VERTEX_INPUT,
            PipelineStageFlags::VertexShader => vk::PipelineStageFlags::VERTEX_SHADER,
            PipelineStageFlags::TesselationControlShader => {
                vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER
            }
            PipelineStageFlags::TesselationEvaluationShader => {
                vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER
            }
            PipelineStageFlags::GeometryShader => vk::PipelineStageFlags::GEOMETRY_SHADER,
            PipelineStageFlags::FragmentShader => vk::PipelineStageFlags::FRAGMENT_SHADER,
            PipelineStageFlags::EarlyFragmentTests => vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            PipelineStageFlags::LateFragmentTests => vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            PipelineStageFlags::ColorAttachmentOutput => {
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            }
            PipelineStageFlags::ComputeShader => vk::PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::Transfer => vk::PipelineStageFlags::TRANSFER,
            PipelineStageFlags::BottomOfPipe => vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            PipelineStageFlags::Host => vk::PipelineStageFlags::HOST,
            PipelineStageFlags::AllGraphics => vk::PipelineStageFlags::ALL_GRAPHICS,
            PipelineStageFlags::AllCommands => vk::PipelineStageFlags::ALL_COMMANDS,
        }
    }
}
*/
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessFlags {
    Empty,
    IndirectCommandRead,
    IndexRead,
    VertexAttributeRead,
    UniformRead,
    InputAttachmentRead,
    ShaderRead,
    ShaderWrite,
    ColorAttachmentRead,
    ColorAttachmentWrite,
    DepthStencilAttachmentRead,
    DepthStencilAttachmentWrite,
    TransferRead,
    TransferWrite,
    HostRead,
    HostWrite,
    MemoryRead,
    MemoryWrite,
}

impl AccessFlags {
    pub fn from_access_flag_mask(flag_mask: vk::AccessFlags) -> Vec<AccessFlags> {
        let mut flags = Vec::default();
        if flag_mask.intersects(vk::AccessFlags::INDIRECT_COMMAND_READ) {
            flags.push(AccessFlags::IndirectCommandRead);
        }
        if flag_mask.intersects(vk::AccessFlags::INDEX_READ) {
            flags.push(AccessFlags::IndexRead);
        }
        if flag_mask.intersects(vk::AccessFlags::VERTEX_ATTRIBUTE_READ) {
            flags.push(AccessFlags::VertexAttributeRead);
        }
        if flag_mask.intersects(vk::AccessFlags::UNIFORM_READ) {
            flags.push(AccessFlags::UniformRead);
        }
        if flag_mask.intersects(vk::AccessFlags::INPUT_ATTACHMENT_READ) {
            flags.push(AccessFlags::InputAttachmentRead);
        }
        if flag_mask.intersects(vk::AccessFlags::SHADER_READ) {
            flags.push(AccessFlags::ShaderRead);
        }
        if flag_mask.intersects(vk::AccessFlags::SHADER_WRITE) {
            flags.push(AccessFlags::ShaderWrite);
        }
        if flag_mask.intersects(vk::AccessFlags::COLOR_ATTACHMENT_READ) {
            flags.push(AccessFlags::ColorAttachmentRead);
        }
        if flag_mask.intersects(vk::AccessFlags::COLOR_ATTACHMENT_WRITE) {
            flags.push(AccessFlags::ColorAttachmentWrite);
        }
        if flag_mask.intersects(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ) {
            flags.push(AccessFlags::DepthStencilAttachmentRead);
        }
        if flag_mask.intersects(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE) {
            flags.push(AccessFlags::DepthStencilAttachmentWrite);
        }
        if flag_mask.intersects(vk::AccessFlags::TRANSFER_READ) {
            flags.push(AccessFlags::TransferRead);
        }
        if flag_mask.intersects(vk::AccessFlags::TRANSFER_WRITE) {
            flags.push(AccessFlags::TransferWrite);
        }
        if flag_mask.intersects(vk::AccessFlags::HOST_READ) {
            flags.push(AccessFlags::HostRead);
        }
        if flag_mask.intersects(vk::AccessFlags::HOST_WRITE) {
            flags.push(AccessFlags::HostWrite);
        }
        if flag_mask.intersects(vk::AccessFlags::MEMORY_READ) {
            flags.push(AccessFlags::MemoryRead);
        }
        if flag_mask.intersects(vk::AccessFlags::MEMORY_WRITE) {
            flags.push(AccessFlags::MemoryWrite);
        }
        flags
    }

    fn create_vk_access_flag_mask_from_list(flags: &[AccessFlags]) -> vk::AccessFlags {
        let mut flag_mask = vk::AccessFlags::empty();
        for flag in flags {
            flag_mask |= (*flag).into();
        }
        flag_mask
    }
}

impl Into<vk::AccessFlags> for AccessFlags {
    fn into(self) -> vk::AccessFlags {
        match self {
            AccessFlags::Empty => vk::AccessFlags::empty(),
            AccessFlags::IndirectCommandRead => vk::AccessFlags::INDIRECT_COMMAND_READ,
            AccessFlags::IndexRead => vk::AccessFlags::INDEX_READ,
            AccessFlags::VertexAttributeRead => vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            AccessFlags::UniformRead => vk::AccessFlags::UNIFORM_READ,
            AccessFlags::InputAttachmentRead => vk::AccessFlags::INPUT_ATTACHMENT_READ,
            AccessFlags::ShaderRead => vk::AccessFlags::SHADER_READ,
            AccessFlags::ShaderWrite => vk::AccessFlags::SHADER_WRITE,
            AccessFlags::ColorAttachmentRead => vk::AccessFlags::COLOR_ATTACHMENT_READ,
            AccessFlags::ColorAttachmentWrite => vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::DepthStencilAttachmentRead => {
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
            }
            AccessFlags::DepthStencilAttachmentWrite => {
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
            }
            AccessFlags::TransferRead => vk::AccessFlags::TRANSFER_READ,
            AccessFlags::TransferWrite => vk::AccessFlags::TRANSFER_WRITE,
            AccessFlags::HostRead => vk::AccessFlags::HOST_READ,
            AccessFlags::HostWrite => vk::AccessFlags::HOST_WRITE,
            AccessFlags::MemoryRead => vk::AccessFlags::MEMORY_READ,
            AccessFlags::MemoryWrite => vk::AccessFlags::MEMORY_WRITE,
        }
    }
}

impl Default for AccessFlags {
    fn default() -> Self {
        AccessFlags::Empty
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyFlags {
    Empty,
    ByRegion,
}

impl Into<vk::DependencyFlags> for DependencyFlags {
    fn into(self) -> vk::DependencyFlags {
        match self {
            DependencyFlags::Empty => vk::DependencyFlags::empty(),
            DependencyFlags::ByRegion => vk::DependencyFlags::BY_REGION,
        }
    }
}

impl Default for DependencyFlags {
    fn default() -> Self {
        DependencyFlags::Empty
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentFormat {
    MatchSurface,
    MatchColorAttachment,
    MatchDepthAttachment,
    Format(Format),
}

impl AttachmentFormat {
    fn as_vk_format(
        &self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> vk::Format {
        match self {
            AttachmentFormat::MatchSurface => swapchain_surface_info.surface_format.format,
            AttachmentFormat::MatchColorAttachment => swapchain_surface_info.color_format,
            AttachmentFormat::MatchDepthAttachment => swapchain_surface_info.depth_format,
            AttachmentFormat::Format(format) => (*format).into(),
        }
    }
}

impl Default for AttachmentFormat {
    fn default() -> Self {
        AttachmentFormat::MatchColorAttachment
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum Format {
    UNDEFINED,
    R4G4_UNORM_PACK8,
    R4G4B4A4_UNORM_PACK16,
    B4G4R4A4_UNORM_PACK16,
    R5G6B5_UNORM_PACK16,
    B5G6R5_UNORM_PACK16,
    R5G5B5A1_UNORM_PACK16,
    B5G5R5A1_UNORM_PACK16,
    A1R5G5B5_UNORM_PACK16,
    R8_UNORM,
    R8_SNORM,
    R8_USCALED,
    R8_SSCALED,
    R8_UINT,
    R8_SINT,
    R8_SRGB,
    R8G8_UNORM,
    R8G8_SNORM,
    R8G8_USCALED,
    R8G8_SSCALED,
    R8G8_UINT,
    R8G8_SINT,
    R8G8_SRGB,
    R8G8B8_UNORM,
    R8G8B8_SNORM,
    R8G8B8_USCALED,
    R8G8B8_SSCALED,
    R8G8B8_UINT,
    R8G8B8_SINT,
    R8G8B8_SRGB,
    B8G8R8_UNORM,
    B8G8R8_SNORM,
    B8G8R8_USCALED,
    B8G8R8_SSCALED,
    B8G8R8_UINT,
    B8G8R8_SINT,
    B8G8R8_SRGB,
    R8G8B8A8_UNORM,
    R8G8B8A8_SNORM,
    R8G8B8A8_USCALED,
    R8G8B8A8_SSCALED,
    R8G8B8A8_UINT,
    R8G8B8A8_SINT,
    R8G8B8A8_SRGB,
    B8G8R8A8_UNORM,
    B8G8R8A8_SNORM,
    B8G8R8A8_USCALED,
    B8G8R8A8_SSCALED,
    B8G8R8A8_UINT,
    B8G8R8A8_SINT,
    B8G8R8A8_SRGB,
    A8B8G8R8_UNORM_PACK32,
    A8B8G8R8_SNORM_PACK32,
    A8B8G8R8_USCALED_PACK32,
    A8B8G8R8_SSCALED_PACK32,
    A8B8G8R8_UINT_PACK32,
    A8B8G8R8_SINT_PACK32,
    A8B8G8R8_SRGB_PACK32,
    A2R10G10B10_UNORM_PACK32,
    A2R10G10B10_SNORM_PACK32,
    A2R10G10B10_USCALED_PACK32,
    A2R10G10B10_SSCALED_PACK32,
    A2R10G10B10_UINT_PACK32,
    A2R10G10B10_SINT_PACK32,
    A2B10G10R10_UNORM_PACK32,
    A2B10G10R10_SNORM_PACK32,
    A2B10G10R10_USCALED_PACK32,
    A2B10G10R10_SSCALED_PACK32,
    A2B10G10R10_UINT_PACK32,
    A2B10G10R10_SINT_PACK32,
    R16_UNORM,
    R16_SNORM,
    R16_USCALED,
    R16_SSCALED,
    R16_UINT,
    R16_SINT,
    R16_SFLOAT,
    R16G16_UNORM,
    R16G16_SNORM,
    R16G16_USCALED,
    R16G16_SSCALED,
    R16G16_UINT,
    R16G16_SINT,
    R16G16_SFLOAT,
    R16G16B16_UNORM,
    R16G16B16_SNORM,
    R16G16B16_USCALED,
    R16G16B16_SSCALED,
    R16G16B16_UINT,
    R16G16B16_SINT,
    R16G16B16_SFLOAT,
    R16G16B16A16_UNORM,
    R16G16B16A16_SNORM,
    R16G16B16A16_USCALED,
    R16G16B16A16_SSCALED,
    R16G16B16A16_UINT,
    R16G16B16A16_SINT,
    R16G16B16A16_SFLOAT,
    R32_UINT,
    R32_SINT,
    R32_SFLOAT,
    R32G32_UINT,
    R32G32_SINT,
    R32G32_SFLOAT,
    R32G32B32_UINT,
    R32G32B32_SINT,
    R32G32B32_SFLOAT,
    R32G32B32A32_UINT,
    R32G32B32A32_SINT,
    R32G32B32A32_SFLOAT,
    R64_UINT,
    R64_SINT,
    R64_SFLOAT,
    R64G64_UINT,
    R64G64_SINT,
    R64G64_SFLOAT,
    R64G64B64_UINT,
    R64G64B64_SINT,
    R64G64B64_SFLOAT,
    R64G64B64A64_UINT,
    R64G64B64A64_SINT,
    R64G64B64A64_SFLOAT,
    B10G11R11_UFLOAT_PACK32,
    E5B9G9R9_UFLOAT_PACK32,
    D16_UNORM,
    X8_D24_UNORM_PACK32,
    D32_SFLOAT,
    S8_UINT,
    D16_UNORM_S8_UINT,
    D24_UNORM_S8_UINT,
    D32_SFLOAT_S8_UINT,
    BC1_RGB_UNORM_BLOCK,
    BC1_RGB_SRGB_BLOCK,
    BC1_RGBA_UNORM_BLOCK,
    BC1_RGBA_SRGB_BLOCK,
    BC2_UNORM_BLOCK,
    BC2_SRGB_BLOCK,
    BC3_UNORM_BLOCK,
    BC3_SRGB_BLOCK,
    BC4_UNORM_BLOCK,
    BC4_SNORM_BLOCK,
    BC5_UNORM_BLOCK,
    BC5_SNORM_BLOCK,
    BC6H_UFLOAT_BLOCK,
    BC6H_SFLOAT_BLOCK,
    BC7_UNORM_BLOCK,
    BC7_SRGB_BLOCK,
    ETC2_R8G8B8_UNORM_BLOCK,
    ETC2_R8G8B8_SRGB_BLOCK,
    ETC2_R8G8B8A1_UNORM_BLOCK,
    ETC2_R8G8B8A1_SRGB_BLOCK,
    ETC2_R8G8B8A8_UNORM_BLOCK,
    ETC2_R8G8B8A8_SRGB_BLOCK,
    EAC_R11_UNORM_BLOCK,
    EAC_R11_SNORM_BLOCK,
    EAC_R11G11_UNORM_BLOCK,
    EAC_R11G11_SNORM_BLOCK,
    ASTC_4X4_UNORM_BLOCK,
    ASTC_4X4_SRGB_BLOCK,
    ASTC_5X4_UNORM_BLOCK,
    ASTC_5X4_SRGB_BLOCK,
    ASTC_5X5_UNORM_BLOCK,
    ASTC_5X5_SRGB_BLOCK,
    ASTC_6X5_UNORM_BLOCK,
    ASTC_6X5_SRGB_BLOCK,
    ASTC_6X6_UNORM_BLOCK,
    ASTC_6X6_SRGB_BLOCK,
    ASTC_8X5_UNORM_BLOCK,
    ASTC_8X5_SRGB_BLOCK,
    ASTC_8X6_UNORM_BLOCK,
    ASTC_8X6_SRGB_BLOCK,
    ASTC_8X8_UNORM_BLOCK,
    ASTC_8X8_SRGB_BLOCK,
    ASTC_10X5_UNORM_BLOCK,
    ASTC_10X5_SRGB_BLOCK,
    ASTC_10X6_UNORM_BLOCK,
    ASTC_10X6_SRGB_BLOCK,
    ASTC_10X8_UNORM_BLOCK,
    ASTC_10X8_SRGB_BLOCK,
    ASTC_10X10_UNORM_BLOCK,
    ASTC_10X10_SRGB_BLOCK,
    ASTC_12X10_UNORM_BLOCK,
    ASTC_12X10_SRGB_BLOCK,
    ASTC_12X12_UNORM_BLOCK,
    ASTC_12X12_SRGB_BLOCK,
}

impl Into<vk::Format> for Format {
    fn into(self) -> vk::Format {
        match self {
            Format::UNDEFINED => vk::Format::UNDEFINED,
            Format::R4G4_UNORM_PACK8 => vk::Format::R4G4_UNORM_PACK8,
            Format::R4G4B4A4_UNORM_PACK16 => vk::Format::R4G4B4A4_UNORM_PACK16,
            Format::B4G4R4A4_UNORM_PACK16 => vk::Format::B4G4R4A4_UNORM_PACK16,
            Format::R5G6B5_UNORM_PACK16 => vk::Format::R5G6B5_UNORM_PACK16,
            Format::B5G6R5_UNORM_PACK16 => vk::Format::B5G6R5_UNORM_PACK16,
            Format::R5G5B5A1_UNORM_PACK16 => vk::Format::R5G5B5A1_UNORM_PACK16,
            Format::B5G5R5A1_UNORM_PACK16 => vk::Format::B5G5R5A1_UNORM_PACK16,
            Format::A1R5G5B5_UNORM_PACK16 => vk::Format::A1R5G5B5_UNORM_PACK16,
            Format::R8_UNORM => vk::Format::R8_UNORM,
            Format::R8_SNORM => vk::Format::R8_SNORM,
            Format::R8_USCALED => vk::Format::R8_USCALED,
            Format::R8_SSCALED => vk::Format::R8_SSCALED,
            Format::R8_UINT => vk::Format::R8_UINT,
            Format::R8_SINT => vk::Format::R8_SINT,
            Format::R8_SRGB => vk::Format::R8_SRGB,
            Format::R8G8_UNORM => vk::Format::R8G8_UNORM,
            Format::R8G8_SNORM => vk::Format::R8G8_SNORM,
            Format::R8G8_USCALED => vk::Format::R8G8_USCALED,
            Format::R8G8_SSCALED => vk::Format::R8G8_SSCALED,
            Format::R8G8_UINT => vk::Format::R8G8_UINT,
            Format::R8G8_SINT => vk::Format::R8G8_SINT,
            Format::R8G8_SRGB => vk::Format::R8G8_SRGB,
            Format::R8G8B8_UNORM => vk::Format::R8G8B8_UNORM,
            Format::R8G8B8_SNORM => vk::Format::R8G8B8_SNORM,
            Format::R8G8B8_USCALED => vk::Format::R8G8B8_USCALED,
            Format::R8G8B8_SSCALED => vk::Format::R8G8B8_SSCALED,
            Format::R8G8B8_UINT => vk::Format::R8G8B8_UINT,
            Format::R8G8B8_SINT => vk::Format::R8G8B8_SINT,
            Format::R8G8B8_SRGB => vk::Format::R8G8B8_SRGB,
            Format::B8G8R8_UNORM => vk::Format::B8G8R8_UNORM,
            Format::B8G8R8_SNORM => vk::Format::B8G8R8_SNORM,
            Format::B8G8R8_USCALED => vk::Format::B8G8R8_USCALED,
            Format::B8G8R8_SSCALED => vk::Format::B8G8R8_SSCALED,
            Format::B8G8R8_UINT => vk::Format::B8G8R8_UINT,
            Format::B8G8R8_SINT => vk::Format::B8G8R8_SINT,
            Format::B8G8R8_SRGB => vk::Format::B8G8R8_SRGB,
            Format::R8G8B8A8_UNORM => vk::Format::R8G8B8A8_UNORM,
            Format::R8G8B8A8_SNORM => vk::Format::R8G8B8A8_SNORM,
            Format::R8G8B8A8_USCALED => vk::Format::R8G8B8A8_USCALED,
            Format::R8G8B8A8_SSCALED => vk::Format::R8G8B8A8_SSCALED,
            Format::R8G8B8A8_UINT => vk::Format::R8G8B8A8_UINT,
            Format::R8G8B8A8_SINT => vk::Format::R8G8B8A8_SINT,
            Format::R8G8B8A8_SRGB => vk::Format::R8G8B8A8_SRGB,
            Format::B8G8R8A8_UNORM => vk::Format::B8G8R8A8_UNORM,
            Format::B8G8R8A8_SNORM => vk::Format::B8G8R8A8_SNORM,
            Format::B8G8R8A8_USCALED => vk::Format::B8G8R8A8_USCALED,
            Format::B8G8R8A8_SSCALED => vk::Format::B8G8R8A8_SSCALED,
            Format::B8G8R8A8_UINT => vk::Format::B8G8R8A8_UINT,
            Format::B8G8R8A8_SINT => vk::Format::B8G8R8A8_SINT,
            Format::B8G8R8A8_SRGB => vk::Format::B8G8R8A8_SRGB,
            Format::A8B8G8R8_UNORM_PACK32 => vk::Format::A8B8G8R8_UNORM_PACK32,
            Format::A8B8G8R8_SNORM_PACK32 => vk::Format::A8B8G8R8_SNORM_PACK32,
            Format::A8B8G8R8_USCALED_PACK32 => vk::Format::A8B8G8R8_USCALED_PACK32,
            Format::A8B8G8R8_SSCALED_PACK32 => vk::Format::A8B8G8R8_SSCALED_PACK32,
            Format::A8B8G8R8_UINT_PACK32 => vk::Format::A8B8G8R8_UINT_PACK32,
            Format::A8B8G8R8_SINT_PACK32 => vk::Format::A8B8G8R8_SINT_PACK32,
            Format::A8B8G8R8_SRGB_PACK32 => vk::Format::A8B8G8R8_SRGB_PACK32,
            Format::A2R10G10B10_UNORM_PACK32 => vk::Format::A2R10G10B10_UNORM_PACK32,
            Format::A2R10G10B10_SNORM_PACK32 => vk::Format::A2R10G10B10_SNORM_PACK32,
            Format::A2R10G10B10_USCALED_PACK32 => vk::Format::A2R10G10B10_USCALED_PACK32,
            Format::A2R10G10B10_SSCALED_PACK32 => vk::Format::A2R10G10B10_SSCALED_PACK32,
            Format::A2R10G10B10_UINT_PACK32 => vk::Format::A2R10G10B10_UINT_PACK32,
            Format::A2R10G10B10_SINT_PACK32 => vk::Format::A2R10G10B10_SINT_PACK32,
            Format::A2B10G10R10_UNORM_PACK32 => vk::Format::A2B10G10R10_UNORM_PACK32,
            Format::A2B10G10R10_SNORM_PACK32 => vk::Format::A2B10G10R10_SNORM_PACK32,
            Format::A2B10G10R10_USCALED_PACK32 => vk::Format::A2B10G10R10_USCALED_PACK32,
            Format::A2B10G10R10_SSCALED_PACK32 => vk::Format::A2B10G10R10_SSCALED_PACK32,
            Format::A2B10G10R10_UINT_PACK32 => vk::Format::A2B10G10R10_UINT_PACK32,
            Format::A2B10G10R10_SINT_PACK32 => vk::Format::A2B10G10R10_SINT_PACK32,
            Format::R16_UNORM => vk::Format::R16_UNORM,
            Format::R16_SNORM => vk::Format::R16_SNORM,
            Format::R16_USCALED => vk::Format::R16_USCALED,
            Format::R16_SSCALED => vk::Format::R16_SSCALED,
            Format::R16_UINT => vk::Format::R16_UINT,
            Format::R16_SINT => vk::Format::R16_SINT,
            Format::R16_SFLOAT => vk::Format::R16_SFLOAT,
            Format::R16G16_UNORM => vk::Format::R16G16_UNORM,
            Format::R16G16_SNORM => vk::Format::R16G16_SNORM,
            Format::R16G16_USCALED => vk::Format::R16G16_USCALED,
            Format::R16G16_SSCALED => vk::Format::R16G16_SSCALED,
            Format::R16G16_UINT => vk::Format::R16G16_UINT,
            Format::R16G16_SINT => vk::Format::R16G16_SINT,
            Format::R16G16_SFLOAT => vk::Format::R16G16_SFLOAT,
            Format::R16G16B16_UNORM => vk::Format::R16G16B16_UNORM,
            Format::R16G16B16_SNORM => vk::Format::R16G16B16_SNORM,
            Format::R16G16B16_USCALED => vk::Format::R16G16B16_USCALED,
            Format::R16G16B16_SSCALED => vk::Format::R16G16B16_SSCALED,
            Format::R16G16B16_UINT => vk::Format::R16G16B16_UINT,
            Format::R16G16B16_SINT => vk::Format::R16G16B16_SINT,
            Format::R16G16B16_SFLOAT => vk::Format::R16G16B16_SFLOAT,
            Format::R16G16B16A16_UNORM => vk::Format::R16G16B16A16_UNORM,
            Format::R16G16B16A16_SNORM => vk::Format::R16G16B16A16_SNORM,
            Format::R16G16B16A16_USCALED => vk::Format::R16G16B16A16_USCALED,
            Format::R16G16B16A16_SSCALED => vk::Format::R16G16B16A16_SSCALED,
            Format::R16G16B16A16_UINT => vk::Format::R16G16B16A16_UINT,
            Format::R16G16B16A16_SINT => vk::Format::R16G16B16A16_SINT,
            Format::R16G16B16A16_SFLOAT => vk::Format::R16G16B16A16_SFLOAT,
            Format::R32_UINT => vk::Format::R32_UINT,
            Format::R32_SINT => vk::Format::R32_SINT,
            Format::R32_SFLOAT => vk::Format::R32_SFLOAT,
            Format::R32G32_UINT => vk::Format::R32G32_UINT,
            Format::R32G32_SINT => vk::Format::R32G32_SINT,
            Format::R32G32_SFLOAT => vk::Format::R32G32_SFLOAT,
            Format::R32G32B32_UINT => vk::Format::R32G32B32_UINT,
            Format::R32G32B32_SINT => vk::Format::R32G32B32_SINT,
            Format::R32G32B32_SFLOAT => vk::Format::R32G32B32_SFLOAT,
            Format::R32G32B32A32_UINT => vk::Format::R32G32B32A32_UINT,
            Format::R32G32B32A32_SINT => vk::Format::R32G32B32A32_SINT,
            Format::R32G32B32A32_SFLOAT => vk::Format::R32G32B32A32_SFLOAT,
            Format::R64_UINT => vk::Format::R64_UINT,
            Format::R64_SINT => vk::Format::R64_SINT,
            Format::R64_SFLOAT => vk::Format::R64_SFLOAT,
            Format::R64G64_UINT => vk::Format::R64G64_UINT,
            Format::R64G64_SINT => vk::Format::R64G64_SINT,
            Format::R64G64_SFLOAT => vk::Format::R64G64_SFLOAT,
            Format::R64G64B64_UINT => vk::Format::R64G64B64_UINT,
            Format::R64G64B64_SINT => vk::Format::R64G64B64_SINT,
            Format::R64G64B64_SFLOAT => vk::Format::R64G64B64_SFLOAT,
            Format::R64G64B64A64_UINT => vk::Format::R64G64B64A64_UINT,
            Format::R64G64B64A64_SINT => vk::Format::R64G64B64A64_SINT,
            Format::R64G64B64A64_SFLOAT => vk::Format::R64G64B64A64_SFLOAT,
            Format::B10G11R11_UFLOAT_PACK32 => vk::Format::B10G11R11_UFLOAT_PACK32,
            Format::E5B9G9R9_UFLOAT_PACK32 => vk::Format::E5B9G9R9_UFLOAT_PACK32,
            Format::D16_UNORM => vk::Format::D16_UNORM,
            Format::X8_D24_UNORM_PACK32 => vk::Format::X8_D24_UNORM_PACK32,
            Format::D32_SFLOAT => vk::Format::D32_SFLOAT,
            Format::S8_UINT => vk::Format::S8_UINT,
            Format::D16_UNORM_S8_UINT => vk::Format::D16_UNORM_S8_UINT,
            Format::D24_UNORM_S8_UINT => vk::Format::D24_UNORM_S8_UINT,
            Format::D32_SFLOAT_S8_UINT => vk::Format::D32_SFLOAT_S8_UINT,
            Format::BC1_RGB_UNORM_BLOCK => vk::Format::BC1_RGB_UNORM_BLOCK,
            Format::BC1_RGB_SRGB_BLOCK => vk::Format::BC1_RGB_SRGB_BLOCK,
            Format::BC1_RGBA_UNORM_BLOCK => vk::Format::BC1_RGBA_UNORM_BLOCK,
            Format::BC1_RGBA_SRGB_BLOCK => vk::Format::BC1_RGBA_SRGB_BLOCK,
            Format::BC2_UNORM_BLOCK => vk::Format::BC2_UNORM_BLOCK,
            Format::BC2_SRGB_BLOCK => vk::Format::BC2_SRGB_BLOCK,
            Format::BC3_UNORM_BLOCK => vk::Format::BC3_UNORM_BLOCK,
            Format::BC3_SRGB_BLOCK => vk::Format::BC3_SRGB_BLOCK,
            Format::BC4_UNORM_BLOCK => vk::Format::BC4_UNORM_BLOCK,
            Format::BC4_SNORM_BLOCK => vk::Format::BC4_SNORM_BLOCK,
            Format::BC5_UNORM_BLOCK => vk::Format::BC5_UNORM_BLOCK,
            Format::BC5_SNORM_BLOCK => vk::Format::BC5_SNORM_BLOCK,
            Format::BC6H_UFLOAT_BLOCK => vk::Format::BC6H_UFLOAT_BLOCK,
            Format::BC6H_SFLOAT_BLOCK => vk::Format::BC6H_SFLOAT_BLOCK,
            Format::BC7_UNORM_BLOCK => vk::Format::BC7_UNORM_BLOCK,
            Format::BC7_SRGB_BLOCK => vk::Format::BC7_SRGB_BLOCK,
            Format::ETC2_R8G8B8_UNORM_BLOCK => vk::Format::ETC2_R8G8B8_UNORM_BLOCK,
            Format::ETC2_R8G8B8_SRGB_BLOCK => vk::Format::ETC2_R8G8B8_SRGB_BLOCK,
            Format::ETC2_R8G8B8A1_UNORM_BLOCK => vk::Format::ETC2_R8G8B8A1_UNORM_BLOCK,
            Format::ETC2_R8G8B8A1_SRGB_BLOCK => vk::Format::ETC2_R8G8B8A1_SRGB_BLOCK,
            Format::ETC2_R8G8B8A8_UNORM_BLOCK => vk::Format::ETC2_R8G8B8A8_UNORM_BLOCK,
            Format::ETC2_R8G8B8A8_SRGB_BLOCK => vk::Format::ETC2_R8G8B8A8_SRGB_BLOCK,
            Format::EAC_R11_UNORM_BLOCK => vk::Format::EAC_R11_UNORM_BLOCK,
            Format::EAC_R11_SNORM_BLOCK => vk::Format::EAC_R11_SNORM_BLOCK,
            Format::EAC_R11G11_UNORM_BLOCK => vk::Format::EAC_R11G11_UNORM_BLOCK,
            Format::EAC_R11G11_SNORM_BLOCK => vk::Format::EAC_R11G11_SNORM_BLOCK,
            Format::ASTC_4X4_UNORM_BLOCK => vk::Format::ASTC_4X4_UNORM_BLOCK,
            Format::ASTC_4X4_SRGB_BLOCK => vk::Format::ASTC_4X4_SRGB_BLOCK,
            Format::ASTC_5X4_UNORM_BLOCK => vk::Format::ASTC_5X4_UNORM_BLOCK,
            Format::ASTC_5X4_SRGB_BLOCK => vk::Format::ASTC_5X4_SRGB_BLOCK,
            Format::ASTC_5X5_UNORM_BLOCK => vk::Format::ASTC_5X5_UNORM_BLOCK,
            Format::ASTC_5X5_SRGB_BLOCK => vk::Format::ASTC_5X5_SRGB_BLOCK,
            Format::ASTC_6X5_UNORM_BLOCK => vk::Format::ASTC_6X5_UNORM_BLOCK,
            Format::ASTC_6X5_SRGB_BLOCK => vk::Format::ASTC_6X5_SRGB_BLOCK,
            Format::ASTC_6X6_UNORM_BLOCK => vk::Format::ASTC_6X6_UNORM_BLOCK,
            Format::ASTC_6X6_SRGB_BLOCK => vk::Format::ASTC_6X6_SRGB_BLOCK,
            Format::ASTC_8X5_UNORM_BLOCK => vk::Format::ASTC_8X5_UNORM_BLOCK,
            Format::ASTC_8X5_SRGB_BLOCK => vk::Format::ASTC_8X5_SRGB_BLOCK,
            Format::ASTC_8X6_UNORM_BLOCK => vk::Format::ASTC_8X6_UNORM_BLOCK,
            Format::ASTC_8X6_SRGB_BLOCK => vk::Format::ASTC_8X6_SRGB_BLOCK,
            Format::ASTC_8X8_UNORM_BLOCK => vk::Format::ASTC_8X8_UNORM_BLOCK,
            Format::ASTC_8X8_SRGB_BLOCK => vk::Format::ASTC_8X8_SRGB_BLOCK,
            Format::ASTC_10X5_UNORM_BLOCK => vk::Format::ASTC_10X5_UNORM_BLOCK,
            Format::ASTC_10X5_SRGB_BLOCK => vk::Format::ASTC_10X5_SRGB_BLOCK,
            Format::ASTC_10X6_UNORM_BLOCK => vk::Format::ASTC_10X6_UNORM_BLOCK,
            Format::ASTC_10X6_SRGB_BLOCK => vk::Format::ASTC_10X6_SRGB_BLOCK,
            Format::ASTC_10X8_UNORM_BLOCK => vk::Format::ASTC_10X8_UNORM_BLOCK,
            Format::ASTC_10X8_SRGB_BLOCK => vk::Format::ASTC_10X8_SRGB_BLOCK,
            Format::ASTC_10X10_UNORM_BLOCK => vk::Format::ASTC_10X10_UNORM_BLOCK,
            Format::ASTC_10X10_SRGB_BLOCK => vk::Format::ASTC_10X10_SRGB_BLOCK,
            Format::ASTC_12X10_UNORM_BLOCK => vk::Format::ASTC_12X10_UNORM_BLOCK,
            Format::ASTC_12X10_SRGB_BLOCK => vk::Format::ASTC_12X10_SRGB_BLOCK,
            Format::ASTC_12X12_UNORM_BLOCK => vk::Format::ASTC_12X12_UNORM_BLOCK,
            Format::ASTC_12X12_SRGB_BLOCK => vk::Format::ASTC_12X12_SRGB_BLOCK,
        }
    }
}
impl From<vk::Format> for Format {
    fn from(format: vk::Format) -> Format {
        match format {
            vk::Format::UNDEFINED => Format::UNDEFINED,
            vk::Format::R4G4_UNORM_PACK8 => Format::R4G4_UNORM_PACK8,
            vk::Format::R4G4B4A4_UNORM_PACK16 => Format::R4G4B4A4_UNORM_PACK16,
            vk::Format::B4G4R4A4_UNORM_PACK16 => Format::B4G4R4A4_UNORM_PACK16,
            vk::Format::R5G6B5_UNORM_PACK16 => Format::R5G6B5_UNORM_PACK16,
            vk::Format::B5G6R5_UNORM_PACK16 => Format::B5G6R5_UNORM_PACK16,
            vk::Format::R5G5B5A1_UNORM_PACK16 => Format::R5G5B5A1_UNORM_PACK16,
            vk::Format::B5G5R5A1_UNORM_PACK16 => Format::B5G5R5A1_UNORM_PACK16,
            vk::Format::A1R5G5B5_UNORM_PACK16 => Format::A1R5G5B5_UNORM_PACK16,
            vk::Format::R8_UNORM => Format::R8_UNORM,
            vk::Format::R8_SNORM => Format::R8_SNORM,
            vk::Format::R8_USCALED => Format::R8_USCALED,
            vk::Format::R8_SSCALED => Format::R8_SSCALED,
            vk::Format::R8_UINT => Format::R8_UINT,
            vk::Format::R8_SINT => Format::R8_SINT,
            vk::Format::R8_SRGB => Format::R8_SRGB,
            vk::Format::R8G8_UNORM => Format::R8G8_UNORM,
            vk::Format::R8G8_SNORM => Format::R8G8_SNORM,
            vk::Format::R8G8_USCALED => Format::R8G8_USCALED,
            vk::Format::R8G8_SSCALED => Format::R8G8_SSCALED,
            vk::Format::R8G8_UINT => Format::R8G8_UINT,
            vk::Format::R8G8_SINT => Format::R8G8_SINT,
            vk::Format::R8G8_SRGB => Format::R8G8_SRGB,
            vk::Format::R8G8B8_UNORM => Format::R8G8B8_UNORM,
            vk::Format::R8G8B8_SNORM => Format::R8G8B8_SNORM,
            vk::Format::R8G8B8_USCALED => Format::R8G8B8_USCALED,
            vk::Format::R8G8B8_SSCALED => Format::R8G8B8_SSCALED,
            vk::Format::R8G8B8_UINT => Format::R8G8B8_UINT,
            vk::Format::R8G8B8_SINT => Format::R8G8B8_SINT,
            vk::Format::R8G8B8_SRGB => Format::R8G8B8_SRGB,
            vk::Format::B8G8R8_UNORM => Format::B8G8R8_UNORM,
            vk::Format::B8G8R8_SNORM => Format::B8G8R8_SNORM,
            vk::Format::B8G8R8_USCALED => Format::B8G8R8_USCALED,
            vk::Format::B8G8R8_SSCALED => Format::B8G8R8_SSCALED,
            vk::Format::B8G8R8_UINT => Format::B8G8R8_UINT,
            vk::Format::B8G8R8_SINT => Format::B8G8R8_SINT,
            vk::Format::B8G8R8_SRGB => Format::B8G8R8_SRGB,
            vk::Format::R8G8B8A8_UNORM => Format::R8G8B8A8_UNORM,
            vk::Format::R8G8B8A8_SNORM => Format::R8G8B8A8_SNORM,
            vk::Format::R8G8B8A8_USCALED => Format::R8G8B8A8_USCALED,
            vk::Format::R8G8B8A8_SSCALED => Format::R8G8B8A8_SSCALED,
            vk::Format::R8G8B8A8_UINT => Format::R8G8B8A8_UINT,
            vk::Format::R8G8B8A8_SINT => Format::R8G8B8A8_SINT,
            vk::Format::R8G8B8A8_SRGB => Format::R8G8B8A8_SRGB,
            vk::Format::B8G8R8A8_UNORM => Format::B8G8R8A8_UNORM,
            vk::Format::B8G8R8A8_SNORM => Format::B8G8R8A8_SNORM,
            vk::Format::B8G8R8A8_USCALED => Format::B8G8R8A8_USCALED,
            vk::Format::B8G8R8A8_SSCALED => Format::B8G8R8A8_SSCALED,
            vk::Format::B8G8R8A8_UINT => Format::B8G8R8A8_UINT,
            vk::Format::B8G8R8A8_SINT => Format::B8G8R8A8_SINT,
            vk::Format::B8G8R8A8_SRGB => Format::B8G8R8A8_SRGB,
            vk::Format::A8B8G8R8_UNORM_PACK32 => Format::A8B8G8R8_UNORM_PACK32,
            vk::Format::A8B8G8R8_SNORM_PACK32 => Format::A8B8G8R8_SNORM_PACK32,
            vk::Format::A8B8G8R8_USCALED_PACK32 => Format::A8B8G8R8_USCALED_PACK32,
            vk::Format::A8B8G8R8_SSCALED_PACK32 => Format::A8B8G8R8_SSCALED_PACK32,
            vk::Format::A8B8G8R8_UINT_PACK32 => Format::A8B8G8R8_UINT_PACK32,
            vk::Format::A8B8G8R8_SINT_PACK32 => Format::A8B8G8R8_SINT_PACK32,
            vk::Format::A8B8G8R8_SRGB_PACK32 => Format::A8B8G8R8_SRGB_PACK32,
            vk::Format::A2R10G10B10_UNORM_PACK32 => Format::A2R10G10B10_UNORM_PACK32,
            vk::Format::A2R10G10B10_SNORM_PACK32 => Format::A2R10G10B10_SNORM_PACK32,
            vk::Format::A2R10G10B10_USCALED_PACK32 => Format::A2R10G10B10_USCALED_PACK32,
            vk::Format::A2R10G10B10_SSCALED_PACK32 => Format::A2R10G10B10_SSCALED_PACK32,
            vk::Format::A2R10G10B10_UINT_PACK32 => Format::A2R10G10B10_UINT_PACK32,
            vk::Format::A2R10G10B10_SINT_PACK32 => Format::A2R10G10B10_SINT_PACK32,
            vk::Format::A2B10G10R10_UNORM_PACK32 => Format::A2B10G10R10_UNORM_PACK32,
            vk::Format::A2B10G10R10_SNORM_PACK32 => Format::A2B10G10R10_SNORM_PACK32,
            vk::Format::A2B10G10R10_USCALED_PACK32 => Format::A2B10G10R10_USCALED_PACK32,
            vk::Format::A2B10G10R10_SSCALED_PACK32 => Format::A2B10G10R10_SSCALED_PACK32,
            vk::Format::A2B10G10R10_UINT_PACK32 => Format::A2B10G10R10_UINT_PACK32,
            vk::Format::A2B10G10R10_SINT_PACK32 => Format::A2B10G10R10_SINT_PACK32,
            vk::Format::R16_UNORM => Format::R16_UNORM,
            vk::Format::R16_SNORM => Format::R16_SNORM,
            vk::Format::R16_USCALED => Format::R16_USCALED,
            vk::Format::R16_SSCALED => Format::R16_SSCALED,
            vk::Format::R16_UINT => Format::R16_UINT,
            vk::Format::R16_SINT => Format::R16_SINT,
            vk::Format::R16_SFLOAT => Format::R16_SFLOAT,
            vk::Format::R16G16_UNORM => Format::R16G16_UNORM,
            vk::Format::R16G16_SNORM => Format::R16G16_SNORM,
            vk::Format::R16G16_USCALED => Format::R16G16_USCALED,
            vk::Format::R16G16_SSCALED => Format::R16G16_SSCALED,
            vk::Format::R16G16_UINT => Format::R16G16_UINT,
            vk::Format::R16G16_SINT => Format::R16G16_SINT,
            vk::Format::R16G16_SFLOAT => Format::R16G16_SFLOAT,
            vk::Format::R16G16B16_UNORM => Format::R16G16B16_UNORM,
            vk::Format::R16G16B16_SNORM => Format::R16G16B16_SNORM,
            vk::Format::R16G16B16_USCALED => Format::R16G16B16_USCALED,
            vk::Format::R16G16B16_SSCALED => Format::R16G16B16_SSCALED,
            vk::Format::R16G16B16_UINT => Format::R16G16B16_UINT,
            vk::Format::R16G16B16_SINT => Format::R16G16B16_SINT,
            vk::Format::R16G16B16_SFLOAT => Format::R16G16B16_SFLOAT,
            vk::Format::R16G16B16A16_UNORM => Format::R16G16B16A16_UNORM,
            vk::Format::R16G16B16A16_SNORM => Format::R16G16B16A16_SNORM,
            vk::Format::R16G16B16A16_USCALED => Format::R16G16B16A16_USCALED,
            vk::Format::R16G16B16A16_SSCALED => Format::R16G16B16A16_SSCALED,
            vk::Format::R16G16B16A16_UINT => Format::R16G16B16A16_UINT,
            vk::Format::R16G16B16A16_SINT => Format::R16G16B16A16_SINT,
            vk::Format::R16G16B16A16_SFLOAT => Format::R16G16B16A16_SFLOAT,
            vk::Format::R32_UINT => Format::R32_UINT,
            vk::Format::R32_SINT => Format::R32_SINT,
            vk::Format::R32_SFLOAT => Format::R32_SFLOAT,
            vk::Format::R32G32_UINT => Format::R32G32_UINT,
            vk::Format::R32G32_SINT => Format::R32G32_SINT,
            vk::Format::R32G32_SFLOAT => Format::R32G32_SFLOAT,
            vk::Format::R32G32B32_UINT => Format::R32G32B32_UINT,
            vk::Format::R32G32B32_SINT => Format::R32G32B32_SINT,
            vk::Format::R32G32B32_SFLOAT => Format::R32G32B32_SFLOAT,
            vk::Format::R32G32B32A32_UINT => Format::R32G32B32A32_UINT,
            vk::Format::R32G32B32A32_SINT => Format::R32G32B32A32_SINT,
            vk::Format::R32G32B32A32_SFLOAT => Format::R32G32B32A32_SFLOAT,
            vk::Format::R64_UINT => Format::R64_UINT,
            vk::Format::R64_SINT => Format::R64_SINT,
            vk::Format::R64_SFLOAT => Format::R64_SFLOAT,
            vk::Format::R64G64_UINT => Format::R64G64_UINT,
            vk::Format::R64G64_SINT => Format::R64G64_SINT,
            vk::Format::R64G64_SFLOAT => Format::R64G64_SFLOAT,
            vk::Format::R64G64B64_UINT => Format::R64G64B64_UINT,
            vk::Format::R64G64B64_SINT => Format::R64G64B64_SINT,
            vk::Format::R64G64B64_SFLOAT => Format::R64G64B64_SFLOAT,
            vk::Format::R64G64B64A64_UINT => Format::R64G64B64A64_UINT,
            vk::Format::R64G64B64A64_SINT => Format::R64G64B64A64_SINT,
            vk::Format::R64G64B64A64_SFLOAT => Format::R64G64B64A64_SFLOAT,
            vk::Format::B10G11R11_UFLOAT_PACK32 => Format::B10G11R11_UFLOAT_PACK32,
            vk::Format::E5B9G9R9_UFLOAT_PACK32 => Format::E5B9G9R9_UFLOAT_PACK32,
            vk::Format::D16_UNORM => Format::D16_UNORM,
            vk::Format::X8_D24_UNORM_PACK32 => Format::X8_D24_UNORM_PACK32,
            vk::Format::D32_SFLOAT => Format::D32_SFLOAT,
            vk::Format::S8_UINT => Format::S8_UINT,
            vk::Format::D16_UNORM_S8_UINT => Format::D16_UNORM_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT => Format::D24_UNORM_S8_UINT,
            vk::Format::D32_SFLOAT_S8_UINT => Format::D32_SFLOAT_S8_UINT,
            vk::Format::BC1_RGB_UNORM_BLOCK => Format::BC1_RGB_UNORM_BLOCK,
            vk::Format::BC1_RGB_SRGB_BLOCK => Format::BC1_RGB_SRGB_BLOCK,
            vk::Format::BC1_RGBA_UNORM_BLOCK => Format::BC1_RGBA_UNORM_BLOCK,
            vk::Format::BC1_RGBA_SRGB_BLOCK => Format::BC1_RGBA_SRGB_BLOCK,
            vk::Format::BC2_UNORM_BLOCK => Format::BC2_UNORM_BLOCK,
            vk::Format::BC2_SRGB_BLOCK => Format::BC2_SRGB_BLOCK,
            vk::Format::BC3_UNORM_BLOCK => Format::BC3_UNORM_BLOCK,
            vk::Format::BC3_SRGB_BLOCK => Format::BC3_SRGB_BLOCK,
            vk::Format::BC4_UNORM_BLOCK => Format::BC4_UNORM_BLOCK,
            vk::Format::BC4_SNORM_BLOCK => Format::BC4_SNORM_BLOCK,
            vk::Format::BC5_UNORM_BLOCK => Format::BC5_UNORM_BLOCK,
            vk::Format::BC5_SNORM_BLOCK => Format::BC5_SNORM_BLOCK,
            vk::Format::BC6H_UFLOAT_BLOCK => Format::BC6H_UFLOAT_BLOCK,
            vk::Format::BC6H_SFLOAT_BLOCK => Format::BC6H_SFLOAT_BLOCK,
            vk::Format::BC7_UNORM_BLOCK => Format::BC7_UNORM_BLOCK,
            vk::Format::BC7_SRGB_BLOCK => Format::BC7_SRGB_BLOCK,
            vk::Format::ETC2_R8G8B8_UNORM_BLOCK => Format::ETC2_R8G8B8_UNORM_BLOCK,
            vk::Format::ETC2_R8G8B8_SRGB_BLOCK => Format::ETC2_R8G8B8_SRGB_BLOCK,
            vk::Format::ETC2_R8G8B8A1_UNORM_BLOCK => Format::ETC2_R8G8B8A1_UNORM_BLOCK,
            vk::Format::ETC2_R8G8B8A1_SRGB_BLOCK => Format::ETC2_R8G8B8A1_SRGB_BLOCK,
            vk::Format::ETC2_R8G8B8A8_UNORM_BLOCK => Format::ETC2_R8G8B8A8_UNORM_BLOCK,
            vk::Format::ETC2_R8G8B8A8_SRGB_BLOCK => Format::ETC2_R8G8B8A8_SRGB_BLOCK,
            vk::Format::EAC_R11_UNORM_BLOCK => Format::EAC_R11_UNORM_BLOCK,
            vk::Format::EAC_R11_SNORM_BLOCK => Format::EAC_R11_SNORM_BLOCK,
            vk::Format::EAC_R11G11_UNORM_BLOCK => Format::EAC_R11G11_UNORM_BLOCK,
            vk::Format::EAC_R11G11_SNORM_BLOCK => Format::EAC_R11G11_SNORM_BLOCK,
            vk::Format::ASTC_4X4_UNORM_BLOCK => Format::ASTC_4X4_UNORM_BLOCK,
            vk::Format::ASTC_4X4_SRGB_BLOCK => Format::ASTC_4X4_SRGB_BLOCK,
            vk::Format::ASTC_5X4_UNORM_BLOCK => Format::ASTC_5X4_UNORM_BLOCK,
            vk::Format::ASTC_5X4_SRGB_BLOCK => Format::ASTC_5X4_SRGB_BLOCK,
            vk::Format::ASTC_5X5_UNORM_BLOCK => Format::ASTC_5X5_UNORM_BLOCK,
            vk::Format::ASTC_5X5_SRGB_BLOCK => Format::ASTC_5X5_SRGB_BLOCK,
            vk::Format::ASTC_6X5_UNORM_BLOCK => Format::ASTC_6X5_UNORM_BLOCK,
            vk::Format::ASTC_6X5_SRGB_BLOCK => Format::ASTC_6X5_SRGB_BLOCK,
            vk::Format::ASTC_6X6_UNORM_BLOCK => Format::ASTC_6X6_UNORM_BLOCK,
            vk::Format::ASTC_6X6_SRGB_BLOCK => Format::ASTC_6X6_SRGB_BLOCK,
            vk::Format::ASTC_8X5_UNORM_BLOCK => Format::ASTC_8X5_UNORM_BLOCK,
            vk::Format::ASTC_8X5_SRGB_BLOCK => Format::ASTC_8X5_SRGB_BLOCK,
            vk::Format::ASTC_8X6_UNORM_BLOCK => Format::ASTC_8X6_UNORM_BLOCK,
            vk::Format::ASTC_8X6_SRGB_BLOCK => Format::ASTC_8X6_SRGB_BLOCK,
            vk::Format::ASTC_8X8_UNORM_BLOCK => Format::ASTC_8X8_UNORM_BLOCK,
            vk::Format::ASTC_8X8_SRGB_BLOCK => Format::ASTC_8X8_SRGB_BLOCK,
            vk::Format::ASTC_10X5_UNORM_BLOCK => Format::ASTC_10X5_UNORM_BLOCK,
            vk::Format::ASTC_10X5_SRGB_BLOCK => Format::ASTC_10X5_SRGB_BLOCK,
            vk::Format::ASTC_10X6_UNORM_BLOCK => Format::ASTC_10X6_UNORM_BLOCK,
            vk::Format::ASTC_10X6_SRGB_BLOCK => Format::ASTC_10X6_SRGB_BLOCK,
            vk::Format::ASTC_10X8_UNORM_BLOCK => Format::ASTC_10X8_UNORM_BLOCK,
            vk::Format::ASTC_10X8_SRGB_BLOCK => Format::ASTC_10X8_SRGB_BLOCK,
            vk::Format::ASTC_10X10_UNORM_BLOCK => Format::ASTC_10X10_UNORM_BLOCK,
            vk::Format::ASTC_10X10_SRGB_BLOCK => Format::ASTC_10X10_SRGB_BLOCK,
            vk::Format::ASTC_12X10_UNORM_BLOCK => Format::ASTC_12X10_UNORM_BLOCK,
            vk::Format::ASTC_12X10_SRGB_BLOCK => Format::ASTC_12X10_SRGB_BLOCK,
            vk::Format::ASTC_12X12_UNORM_BLOCK => Format::ASTC_12X12_UNORM_BLOCK,
            vk::Format::ASTC_12X12_SRGB_BLOCK => Format::ASTC_12X12_SRGB_BLOCK,
            _ => unimplemented!(),
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Format::UNDEFINED
    }
}

// Returns None for formats unlikely to be used for vertices (like ATSC blocks) or undefined
pub fn size_of_vertex_format(format: Format) -> Option<usize> {
    match format {
        Format::R4G4_UNORM_PACK8 => Some(1),
        Format::R4G4B4A4_UNORM_PACK16 => Some(2),
        Format::B4G4R4A4_UNORM_PACK16 => Some(2),
        Format::R5G6B5_UNORM_PACK16 => Some(2),
        Format::B5G6R5_UNORM_PACK16 => Some(2),
        Format::R5G5B5A1_UNORM_PACK16 => Some(2),
        Format::B5G5R5A1_UNORM_PACK16 => Some(2),
        Format::A1R5G5B5_UNORM_PACK16 => Some(2),
        Format::R8_UNORM => Some(1),
        Format::R8_SNORM => Some(1),
        Format::R8_USCALED => Some(1),
        Format::R8_SSCALED => Some(1),
        Format::R8_UINT => Some(1),
        Format::R8_SINT => Some(1),
        Format::R8_SRGB => Some(1),
        Format::R8G8_UNORM => Some(2),
        Format::R8G8_SNORM => Some(2),
        Format::R8G8_USCALED => Some(2),
        Format::R8G8_SSCALED => Some(2),
        Format::R8G8_UINT => Some(2),
        Format::R8G8_SINT => Some(2),
        Format::R8G8_SRGB => Some(2),
        Format::R8G8B8_UNORM => Some(3),
        Format::R8G8B8_SNORM => Some(3),
        Format::R8G8B8_USCALED => Some(3),
        Format::R8G8B8_SSCALED => Some(3),
        Format::R8G8B8_UINT => Some(3),
        Format::R8G8B8_SINT => Some(3),
        Format::R8G8B8_SRGB => Some(3),
        Format::B8G8R8_UNORM => Some(3),
        Format::B8G8R8_SNORM => Some(3),
        Format::B8G8R8_USCALED => Some(3),
        Format::B8G8R8_SSCALED => Some(3),
        Format::B8G8R8_UINT => Some(3),
        Format::B8G8R8_SINT => Some(3),
        Format::B8G8R8_SRGB => Some(3),
        Format::R8G8B8A8_UNORM => Some(4),
        Format::R8G8B8A8_SNORM => Some(4),
        Format::R8G8B8A8_USCALED => Some(4),
        Format::R8G8B8A8_SSCALED => Some(4),
        Format::R8G8B8A8_UINT => Some(4),
        Format::R8G8B8A8_SINT => Some(4),
        Format::R8G8B8A8_SRGB => Some(4),
        Format::B8G8R8A8_UNORM => Some(4),
        Format::B8G8R8A8_SNORM => Some(4),
        Format::B8G8R8A8_USCALED => Some(4),
        Format::B8G8R8A8_SSCALED => Some(4),
        Format::B8G8R8A8_UINT => Some(4),
        Format::B8G8R8A8_SINT => Some(4),
        Format::B8G8R8A8_SRGB => Some(4),
        Format::A8B8G8R8_UNORM_PACK32 => Some(4),
        Format::A8B8G8R8_SNORM_PACK32 => Some(4),
        Format::A8B8G8R8_USCALED_PACK32 => Some(4),
        Format::A8B8G8R8_SSCALED_PACK32 => Some(4),
        Format::A8B8G8R8_UINT_PACK32 => Some(4),
        Format::A8B8G8R8_SINT_PACK32 => Some(4),
        Format::A8B8G8R8_SRGB_PACK32 => Some(4),
        Format::A2R10G10B10_UNORM_PACK32 => Some(4),
        Format::A2R10G10B10_SNORM_PACK32 => Some(4),
        Format::A2R10G10B10_USCALED_PACK32 => Some(4),
        Format::A2R10G10B10_SSCALED_PACK32 => Some(4),
        Format::A2R10G10B10_UINT_PACK32 => Some(4),
        Format::A2R10G10B10_SINT_PACK32 => Some(4),
        Format::A2B10G10R10_UNORM_PACK32 => Some(4),
        Format::A2B10G10R10_SNORM_PACK32 => Some(4),
        Format::A2B10G10R10_USCALED_PACK32 => Some(4),
        Format::A2B10G10R10_SSCALED_PACK32 => Some(4),
        Format::A2B10G10R10_UINT_PACK32 => Some(4),
        Format::A2B10G10R10_SINT_PACK32 => Some(4),
        Format::R16_UNORM => Some(2),
        Format::R16_SNORM => Some(2),
        Format::R16_USCALED => Some(2),
        Format::R16_SSCALED => Some(2),
        Format::R16_UINT => Some(2),
        Format::R16_SINT => Some(2),
        Format::R16_SFLOAT => Some(2),
        Format::R16G16_UNORM => Some(4),
        Format::R16G16_SNORM => Some(4),
        Format::R16G16_USCALED => Some(4),
        Format::R16G16_SSCALED => Some(4),
        Format::R16G16_UINT => Some(4),
        Format::R16G16_SINT => Some(4),
        Format::R16G16_SFLOAT => Some(4),
        Format::R16G16B16_UNORM => Some(6),
        Format::R16G16B16_SNORM => Some(6),
        Format::R16G16B16_USCALED => Some(6),
        Format::R16G16B16_SSCALED => Some(6),
        Format::R16G16B16_UINT => Some(6),
        Format::R16G16B16_SINT => Some(6),
        Format::R16G16B16_SFLOAT => Some(6),
        Format::R16G16B16A16_UNORM => Some(8),
        Format::R16G16B16A16_SNORM => Some(8),
        Format::R16G16B16A16_USCALED => Some(8),
        Format::R16G16B16A16_SSCALED => Some(8),
        Format::R16G16B16A16_UINT => Some(8),
        Format::R16G16B16A16_SINT => Some(8),
        Format::R16G16B16A16_SFLOAT => Some(8),
        Format::R32_UINT => Some(4),
        Format::R32_SINT => Some(4),
        Format::R32_SFLOAT => Some(4),
        Format::R32G32_UINT => Some(8),
        Format::R32G32_SINT => Some(8),
        Format::R32G32_SFLOAT => Some(8),
        Format::R32G32B32_UINT => Some(12),
        Format::R32G32B32_SINT => Some(12),
        Format::R32G32B32_SFLOAT => Some(12),
        Format::R32G32B32A32_UINT => Some(16),
        Format::R32G32B32A32_SINT => Some(16),
        Format::R32G32B32A32_SFLOAT => Some(16),
        Format::R64_UINT => Some(8),
        Format::R64_SINT => Some(8),
        Format::R64_SFLOAT => Some(8),
        Format::R64G64_UINT => Some(16),
        Format::R64G64_SINT => Some(16),
        Format::R64G64_SFLOAT => Some(16),
        Format::R64G64B64_UINT => Some(24),
        Format::R64G64B64_SINT => Some(24),
        Format::R64G64B64_SFLOAT => Some(24),
        Format::R64G64B64A64_UINT => Some(32),
        Format::R64G64B64A64_SINT => Some(32),
        Format::R64G64B64A64_SFLOAT => Some(32),
        Format::B10G11R11_UFLOAT_PACK32 => Some(4),
        Format::E5B9G9R9_UFLOAT_PACK32 => Some(4),
        Format::D16_UNORM => Some(2),
        Format::X8_D24_UNORM_PACK32 => Some(4),
        Format::D32_SFLOAT => Some(4),
        Format::S8_UINT => Some(1),
        Format::D16_UNORM_S8_UINT => Some(3),
        Format::D24_UNORM_S8_UINT => Some(4),
        Format::D32_SFLOAT_S8_UINT => Some(5),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AttachmentDescription {
    pub flags: AttachmentDescriptionFlags,
    pub format: AttachmentFormat,
    pub samples: SampleCountFlags,
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub stencil_load_op: AttachmentLoadOp,
    pub stencil_store_op: AttachmentStoreOp,
    pub initial_layout: ImageLayout,
    pub final_layout: ImageLayout,
}

impl AttachmentDescription {
    pub fn as_builder(
        &self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> vk::AttachmentDescriptionBuilder {
        vk::AttachmentDescription::builder()
            .flags(self.flags.into())
            .format(self.format.as_vk_format(swapchain_surface_info))
            .samples(
                self.samples
                    .as_vk_sample_count_flags(swapchain_surface_info),
            )
            .load_op(self.load_op.into())
            .store_op(self.store_op.into())
            .stencil_load_op(self.stencil_load_op.into())
            .stencil_store_op(self.stencil_store_op.into())
            .initial_layout(self.initial_layout.into())
            .final_layout(self.final_layout.into())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentIndex {
    Index(u32),
    Unused,
}

impl Into<u32> for AttachmentIndex {
    fn into(self) -> u32 {
        match self {
            AttachmentIndex::Index(index) => index,
            AttachmentIndex::Unused => vk::ATTACHMENT_UNUSED,
        }
    }
}

impl Default for AttachmentIndex {
    fn default() -> Self {
        AttachmentIndex::Index(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AttachmentReference {
    pub attachment: AttachmentIndex,
    pub layout: ImageLayout,
}

impl AttachmentReference {
    pub fn as_builder(&self) -> vk::AttachmentReferenceBuilder {
        vk::AttachmentReference::builder()
            .attachment(self.attachment.into())
            .layout(self.layout.into())
    }
}

impl Into<vk::AttachmentReference> for AttachmentReference {
    fn into(self) -> vk::AttachmentReference {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SubpassDescription {
    pub pipeline_bind_point: PipelineBindPoint,
    pub input_attachments: Vec<AttachmentReference>,
    pub color_attachments: Vec<AttachmentReference>,
    pub resolve_attachments: Vec<AttachmentReference>,
    pub depth_stencil_attachment: Option<AttachmentReference>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubpassDependencyIndex {
    External,
    Index(u32),
}

impl Into<u32> for SubpassDependencyIndex {
    fn into(self) -> u32 {
        match self {
            SubpassDependencyIndex::External => vk::SUBPASS_EXTERNAL,
            SubpassDependencyIndex::Index(index) => index,
        }
    }
}

impl Default for SubpassDependencyIndex {
    fn default() -> Self {
        SubpassDependencyIndex::External
    }
}

//TODO: Change the Vec<T> for pipeline stages and accesses to be masks. This will require a custom
// bitfield type and serializer that supports printing them in a list of enum values
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SubpassDependency {
    pub src_subpass: SubpassDependencyIndex,
    pub dst_subpass: SubpassDependencyIndex,
    pub src_stage_mask: PipelineStageFlags,
    pub dst_stage_mask: PipelineStageFlags,
    pub src_access_mask: Vec<AccessFlags>,
    pub dst_access_mask: Vec<AccessFlags>,
    pub dependency_flags: DependencyFlags,
}

impl SubpassDependency {
    pub fn as_builder(&self) -> vk::SubpassDependencyBuilder {
        // fn access_flag_list_to_mask(access_flags: &[AccessFlags]) -> vk::AccessFlags {
        //     let mut access_flags_mask = vk::AccessFlags::empty();
        //     for access_flag in access_flags {
        //         let vk_access_flag: vk::AccessFlags = (*access_flag).into();
        //         access_flags_mask |= vk_access_flag;
        //     }
        //     access_flags_mask
        // }

        vk::SubpassDependency::builder()
            .src_subpass(self.src_subpass.into())
            .dst_subpass(self.dst_subpass.into())
            .src_stage_mask(vk::PipelineStageFlags::from_raw(self.src_stage_mask.bits()))
            .dst_stage_mask(vk::PipelineStageFlags::from_raw(self.dst_stage_mask.bits()))
            .src_access_mask(AccessFlags::create_vk_access_flag_mask_from_list(
                self.src_access_mask.as_slice(),
            ))
            .dst_access_mask(AccessFlags::create_vk_access_flag_mask_from_list(
                self.dst_access_mask.as_slice(),
            ))
            .dependency_flags(self.dependency_flags.into())
    }
}

impl Into<vk::SubpassDependency> for SubpassDependency {
    fn into(self) -> vk::SubpassDependency {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RenderPass {
    pub attachments: Vec<AttachmentDescription>,
    pub subpasses: Vec<SubpassDescription>,
    pub dependencies: Vec<SubpassDependency>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    TriangleFan,
    LineListWithAdjacency,
    LineStripWithAdjacency,
    TriangleListWithAdjacency,
    TriangleStripWithAdjacency,
    PatchList,
}

impl Into<vk::PrimitiveTopology> for PrimitiveTopology {
    fn into(self) -> vk::PrimitiveTopology {
        match self {
            PrimitiveTopology::PointList => vk::PrimitiveTopology::POINT_LIST,
            PrimitiveTopology::LineList => vk::PrimitiveTopology::LINE_LIST,
            PrimitiveTopology::LineStrip => vk::PrimitiveTopology::LINE_STRIP,
            PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
            PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
            PrimitiveTopology::TriangleFan => vk::PrimitiveTopology::TRIANGLE_FAN,
            PrimitiveTopology::LineListWithAdjacency => {
                vk::PrimitiveTopology::LINE_LIST_WITH_ADJACENCY
            }
            PrimitiveTopology::LineStripWithAdjacency => {
                vk::PrimitiveTopology::LINE_STRIP_WITH_ADJACENCY
            }
            PrimitiveTopology::TriangleListWithAdjacency => {
                vk::PrimitiveTopology::TRIANGLE_LIST_WITH_ADJACENCY
            }
            PrimitiveTopology::TriangleStripWithAdjacency => {
                vk::PrimitiveTopology::TRIANGLE_STRIP_WITH_ADJACENCY
            }
            PrimitiveTopology::PatchList => vk::PrimitiveTopology::PATCH_LIST,
        }
    }
}

impl Default for PrimitiveTopology {
    fn default() -> Self {
        PrimitiveTopology::PointList
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineInputAssemblyState {
    pub primitive_topology: PrimitiveTopology,
    pub primitive_restart_enable: bool,
}

impl PipelineInputAssemblyState {
    pub fn as_builder(&self) -> vk::PipelineInputAssemblyStateCreateInfoBuilder {
        vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(self.primitive_topology.into())
            .primitive_restart_enable(self.primitive_restart_enable)
    }
}

impl Into<vk::PipelineInputAssemblyStateCreateInfo> for PipelineInputAssemblyState {
    fn into(self) -> vk::PipelineInputAssemblyStateCreateInfo {
        self.as_builder().build()
    }
}

//impl Into<vk::PipelineInputAssemblyState>

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

impl Into<vk::VertexInputRate> for VertexInputRate {
    fn into(self) -> vk::VertexInputRate {
        match self {
            VertexInputRate::Vertex => vk::VertexInputRate::VERTEX,
            VertexInputRate::Instance => vk::VertexInputRate::INSTANCE,
        }
    }
}

impl Default for VertexInputRate {
    fn default() -> Self {
        VertexInputRate::Vertex
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct VertexInputBindingDescription {
    pub binding: u32,
    pub stride: u32,
    pub input_rate: VertexInputRate,
}

impl VertexInputBindingDescription {
    pub fn as_builder(&self) -> vk::VertexInputBindingDescriptionBuilder {
        vk::VertexInputBindingDescription::builder()
            .binding(self.binding)
            .stride(self.stride)
            .input_rate(self.input_rate.into())
    }
}

impl Into<vk::VertexInputBindingDescription> for VertexInputBindingDescription {
    fn into(self) -> vk::VertexInputBindingDescription {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct VertexInputAttributeDescription {
    pub location: u32,
    pub binding: u32,
    pub format: Format,
    pub offset: u32,
}

impl Into<vk::VertexInputAttributeDescription> for VertexInputAttributeDescription {
    fn into(self) -> vk::VertexInputAttributeDescription {
        vk::VertexInputAttributeDescription::builder()
            .location(self.location)
            .binding(self.binding)
            .format(self.format.into())
            .offset(self.offset)
            .build()
    }
}

// impl VertexInputAttributeDescription {
//     pub fn as_builder(
//         &self,
//     ) -> vk::VertexInputAttributeDescriptionBuilder {
//         vk::VertexInputAttributeDescription::builder()
//             .location(self.location)
//             .binding(self.binding)
//             .format(self.format.into())
//             .offset(self.offset)
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineVertexInputState {
    pub binding_descriptions: Vec<VertexInputBindingDescription>,
    pub attribute_descriptions: Vec<VertexInputAttributeDescription>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RectDecimal {
    pub x: Decimal,
    pub y: Decimal,
    pub width: Decimal,
    pub height: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RectF32 {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RectI32 {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dimensions {
    MatchSwapchain,
    MatchFramebuffer,
    Raw(RectDecimal),
}

impl Dimensions {
    fn as_rect_f32(
        &self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        framebuffer_meta: &FramebufferMeta,
    ) -> RectF32 {
        match self {
            Dimensions::MatchSwapchain => RectF32 {
                x: 0.0,
                y: 0.0,
                width: swapchain_surface_info.extents.width as f32,
                height: swapchain_surface_info.extents.height as f32,
            },
            Dimensions::MatchFramebuffer => RectF32 {
                x: 0.0,
                y: 0.0,
                width: framebuffer_meta.width as f32,
                height: framebuffer_meta.height as f32,
            },
            Dimensions::Raw(rect) => RectF32 {
                x: rect.x.to_f32(),
                y: rect.y.to_f32(),
                width: rect.width.to_f32(),
                height: rect.height.to_f32(),
            },
        }
    }

    fn as_rect_i32(
        &self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        framebuffer_meta: &FramebufferMeta,
    ) -> RectI32 {
        match self {
            Dimensions::MatchSwapchain => RectI32 {
                x: 0,
                y: 0,
                width: swapchain_surface_info.extents.width,
                height: swapchain_surface_info.extents.height,
            },
            Dimensions::MatchFramebuffer => RectI32 {
                x: 0,
                y: 0,
                width: framebuffer_meta.width,
                height: framebuffer_meta.height,
            },
            Dimensions::Raw(rect) => RectI32 {
                x: rect.x.to_i32(),
                y: rect.y.to_i32(),
                width: rect.width.to_u32(),
                height: rect.height.to_u32(),
            },
        }
    }
}

impl Default for Dimensions {
    fn default() -> Self {
        Dimensions::MatchSwapchain
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Viewport {
    pub dimensions: Dimensions,
    pub min_depth: Decimal,
    pub max_depth: Decimal,
}

impl Viewport {
    pub fn as_builder(
        &self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        framebuffer_meta: &FramebufferMeta,
    ) -> vk::ViewportBuilder {
        let rect_f32 = self
            .dimensions
            .as_rect_f32(swapchain_surface_info, framebuffer_meta);
        vk::Viewport::builder()
            .x(rect_f32.x)
            .y(rect_f32.y)
            .width(rect_f32.width)
            .height(rect_f32.height)
            .min_depth(self.min_depth.to_f32())
            .max_depth(self.max_depth.to_f32())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Scissors {
    dimensions: Dimensions,
}

impl Scissors {
    pub fn to_rect2d(
        &self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        framebuffer_meta: &FramebufferMeta,
    ) -> vk::Rect2D {
        let rect_i32 = self
            .dimensions
            .as_rect_i32(swapchain_surface_info, framebuffer_meta);
        vk::Rect2D {
            offset: vk::Offset2D {
                x: rect_i32.x,
                y: rect_i32.y,
            },
            extent: vk::Extent2D {
                width: rect_i32.width,
                height: rect_i32.height,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineViewportState {
    pub viewports: Vec<Viewport>,
    pub scissors: Vec<Scissors>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolygonMode {
    Fill,
    Line,
    Point,
}

impl Into<vk::PolygonMode> for PolygonMode {
    fn into(self) -> vk::PolygonMode {
        match self {
            PolygonMode::Fill => vk::PolygonMode::FILL,
            PolygonMode::Line => vk::PolygonMode::LINE,
            PolygonMode::Point => vk::PolygonMode::POINT,
        }
    }
}

impl Default for PolygonMode {
    fn default() -> Self {
        PolygonMode::Fill
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

impl Into<vk::FrontFace> for FrontFace {
    fn into(self) -> vk::FrontFace {
        match self {
            FrontFace::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
            FrontFace::Clockwise => vk::FrontFace::CLOCKWISE,
        }
    }
}

impl Default for FrontFace {
    fn default() -> Self {
        FrontFace::CounterClockwise
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CullModeFlags {
    None,
    Front,
    Back,
    FrontAndBack,
}

impl Into<vk::CullModeFlags> for CullModeFlags {
    fn into(self) -> vk::CullModeFlags {
        match self {
            CullModeFlags::None => vk::CullModeFlags::NONE,
            CullModeFlags::Front => vk::CullModeFlags::FRONT,
            CullModeFlags::Back => vk::CullModeFlags::BACK,
            CullModeFlags::FrontAndBack => vk::CullModeFlags::FRONT_AND_BACK,
        }
    }
}

impl Default for CullModeFlags {
    fn default() -> Self {
        CullModeFlags::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineRasterizationState {
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: PolygonMode,
    pub cull_mode: CullModeFlags,
    pub front_face: FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: Decimal,
    pub depth_bias_clamp: Decimal,
    pub depth_bias_slope_factor: Decimal,
    pub line_width: Decimal,
}

impl PipelineRasterizationState {
    pub fn as_builder(&self) -> vk::PipelineRasterizationStateCreateInfoBuilder {
        vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(self.depth_bias_enable)
            .rasterizer_discard_enable(self.rasterizer_discard_enable)
            .polygon_mode(self.polygon_mode.into())
            .cull_mode(self.cull_mode.into())
            .front_face(self.front_face.into())
            .depth_bias_enable(self.depth_bias_enable)
            .depth_bias_constant_factor(self.depth_bias_constant_factor.to_f32())
            .depth_bias_clamp(self.depth_bias_clamp.to_f32())
            .depth_bias_slope_factor(self.depth_bias_slope_factor.to_f32())
            .line_width(self.line_width.to_f32())
    }
}

impl Into<vk::PipelineRasterizationStateCreateInfo> for PipelineRasterizationState {
    fn into(self) -> vk::PipelineRasterizationStateCreateInfo {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineMultisampleState {
    pub rasterization_samples: PipelineSampleCountFlags,
    pub sample_shading_enable: bool,
    pub min_sample_shading: Decimal,
    pub sample_mask: Option<Vec<u32>>,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}

impl PipelineMultisampleState {
    pub fn as_builder(
        &self,
        subpass_info: &SubpassInfo,
    ) -> vk::PipelineMultisampleStateCreateInfoBuilder {
        let mut builder = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(
                self.rasterization_samples
                    .as_vk_sample_count_flags(subpass_info),
            )
            .sample_shading_enable(self.sample_shading_enable)
            .min_sample_shading(self.min_sample_shading.to_f32())
            .alpha_to_coverage_enable(self.alpha_to_coverage_enable)
            .alpha_to_one_enable(self.alpha_to_one_enable);

        if let Some(sample_mask) = &self.sample_mask {
            builder = builder.sample_mask(sample_mask.as_slice())
        }

        builder
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendFactor {
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
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
    Src1Color,
    OneMinusSrc1Color,
    Src1Alpha,
    OneMinusSrc1Alpha,
}

impl Into<vk::BlendFactor> for BlendFactor {
    fn into(self) -> vk::BlendFactor {
        match self {
            BlendFactor::Zero => vk::BlendFactor::ZERO,
            BlendFactor::One => vk::BlendFactor::ONE,
            BlendFactor::SrcColor => vk::BlendFactor::SRC_COLOR,
            BlendFactor::OneMinusSrcColor => vk::BlendFactor::ONE_MINUS_SRC_COLOR,
            BlendFactor::DstColor => vk::BlendFactor::DST_COLOR,
            BlendFactor::OneMinusDstColor => vk::BlendFactor::ONE_MINUS_DST_COLOR,
            BlendFactor::SrcAlpha => vk::BlendFactor::SRC_ALPHA,
            BlendFactor::OneMinusSrcAlpha => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            BlendFactor::DstAlpha => vk::BlendFactor::DST_ALPHA,
            BlendFactor::OneMinusDstAlpha => vk::BlendFactor::ONE_MINUS_DST_ALPHA,
            BlendFactor::ConstantColor => vk::BlendFactor::CONSTANT_COLOR,
            BlendFactor::OneMinusConstantColor => vk::BlendFactor::ONE_MINUS_CONSTANT_COLOR,
            BlendFactor::ConstantAlpha => vk::BlendFactor::CONSTANT_ALPHA,
            BlendFactor::OneMinusConstantAlpha => vk::BlendFactor::ONE_MINUS_CONSTANT_ALPHA,
            BlendFactor::SrcAlphaSaturate => vk::BlendFactor::SRC_ALPHA_SATURATE,
            BlendFactor::Src1Color => vk::BlendFactor::SRC1_COLOR,
            BlendFactor::OneMinusSrc1Color => vk::BlendFactor::ONE_MINUS_SRC1_COLOR,
            BlendFactor::Src1Alpha => vk::BlendFactor::SRC1_ALPHA,
            BlendFactor::OneMinusSrc1Alpha => vk::BlendFactor::ONE_MINUS_SRC1_ALPHA,
        }
    }
}

impl Default for BlendFactor {
    fn default() -> Self {
        BlendFactor::Zero
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

impl Into<vk::BlendOp> for BlendOp {
    fn into(self) -> vk::BlendOp {
        match self {
            BlendOp::Add => vk::BlendOp::ADD,
            BlendOp::Subtract => vk::BlendOp::SUBTRACT,
            BlendOp::ReverseSubtract => vk::BlendOp::REVERSE_SUBTRACT,
            BlendOp::Min => vk::BlendOp::MIN,
            BlendOp::Max => vk::BlendOp::MAX,
        }
    }
}

impl Default for BlendOp {
    fn default() -> Self {
        BlendOp::Add
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ColorComponentFlags {
    pub red: bool,
    pub green: bool,
    pub blue: bool,
    pub alpha: bool,
}

impl Into<vk::ColorComponentFlags> for ColorComponentFlags {
    fn into(self) -> vk::ColorComponentFlags {
        let mut color_component_flags = vk::ColorComponentFlags::empty();

        if self.red {
            color_component_flags |= vk::ColorComponentFlags::R;
        }

        if self.green {
            color_component_flags |= vk::ColorComponentFlags::G;
        }

        if self.blue {
            color_component_flags |= vk::ColorComponentFlags::B;
        }

        if self.alpha {
            color_component_flags |= vk::ColorComponentFlags::A;
        }

        color_component_flags
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineColorBlendAttachmentState {
    pub blend_enable: bool,
    pub src_color_blend_factor: BlendFactor,
    pub dst_color_blend_factor: BlendFactor,
    pub color_blend_op: BlendOp,
    pub src_alpha_blend_factor: BlendFactor,
    pub dst_alpha_blend_factor: BlendFactor,
    pub alpha_blend_op: BlendOp,
    pub color_write_mask: ColorComponentFlags,
}

impl PipelineColorBlendAttachmentState {
    pub fn as_builder(&self) -> vk::PipelineColorBlendAttachmentStateBuilder {
        vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(self.blend_enable)
            .src_color_blend_factor(self.src_color_blend_factor.into())
            .dst_color_blend_factor(self.dst_color_blend_factor.into())
            .color_blend_op(self.color_blend_op.into())
            .src_alpha_blend_factor(self.src_alpha_blend_factor.into())
            .dst_alpha_blend_factor(self.dst_alpha_blend_factor.into())
            .alpha_blend_op(self.alpha_blend_op.into())
            .color_write_mask(self.color_write_mask.into())
    }
}

impl Into<vk::PipelineColorBlendAttachmentState> for PipelineColorBlendAttachmentState {
    fn into(self) -> vk::PipelineColorBlendAttachmentState {
        self.as_builder().build()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogicOp {
    Clear,
    And,
    AndReverse,
    Copy,
    AndInverted,
    NoOp,
    Xor,
    Or,
    Nor,
    Equivalent,
    Invert,
    OrReverse,
    CopyInverted,
    OrInverted,
    Nand,
    Set,
}

impl Into<vk::LogicOp> for LogicOp {
    fn into(self) -> vk::LogicOp {
        match self {
            LogicOp::Clear => vk::LogicOp::CLEAR,
            LogicOp::And => vk::LogicOp::AND,
            LogicOp::AndReverse => vk::LogicOp::AND_REVERSE,
            LogicOp::Copy => vk::LogicOp::COPY,
            LogicOp::AndInverted => vk::LogicOp::AND_INVERTED,
            LogicOp::NoOp => vk::LogicOp::NO_OP,
            LogicOp::Xor => vk::LogicOp::XOR,
            LogicOp::Or => vk::LogicOp::OR,
            LogicOp::Nor => vk::LogicOp::NOR,
            LogicOp::Equivalent => vk::LogicOp::EQUIVALENT,
            LogicOp::Invert => vk::LogicOp::INVERT,
            LogicOp::OrReverse => vk::LogicOp::OR_REVERSE,
            LogicOp::CopyInverted => vk::LogicOp::COPY_INVERTED,
            LogicOp::OrInverted => vk::LogicOp::OR_INVERTED,
            LogicOp::Nand => vk::LogicOp::NAND,
            LogicOp::Set => vk::LogicOp::SET,
        }
    }
}

impl Default for LogicOp {
    fn default() -> LogicOp {
        LogicOp::Clear
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineColorBlendState {
    pub logic_op_enable: bool,
    pub logic_op: LogicOp,
    pub attachments: Vec<PipelineColorBlendAttachmentState>,
    pub blend_constants: [Decimal; 4],
}

impl PipelineColorBlendState {
    pub fn blend_constants_as_f32(&self) -> [f32; 4] {
        [
            self.blend_constants[0].to_f32(),
            self.blend_constants[1].to_f32(),
            self.blend_constants[2].to_f32(),
            self.blend_constants[3].to_f32(),
        ]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DynamicState {
    Viewport,
    Scissor,
    LineWidth,
    DepthBias,
    BlendConstants,
    DepthBounds,
    StencilCompareMask,
    StencilWriteMask,
    StencilReference,
}

impl Into<vk::DynamicState> for DynamicState {
    fn into(self) -> vk::DynamicState {
        match self {
            DynamicState::Viewport => vk::DynamicState::VIEWPORT,
            DynamicState::Scissor => vk::DynamicState::SCISSOR,
            DynamicState::LineWidth => vk::DynamicState::LINE_WIDTH,
            DynamicState::DepthBias => vk::DynamicState::DEPTH_BIAS,
            DynamicState::BlendConstants => vk::DynamicState::BLEND_CONSTANTS,
            DynamicState::DepthBounds => vk::DynamicState::DEPTH_BOUNDS,
            DynamicState::StencilCompareMask => vk::DynamicState::STENCIL_COMPARE_MASK,
            DynamicState::StencilWriteMask => vk::DynamicState::STENCIL_WRITE_MASK,
            DynamicState::StencilReference => vk::DynamicState::STENCIL_REFERENCE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineDynamicState {
    pub dynamic_states: Vec<DynamicState>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StencilOp {
    Keep,
    Zero,
    Replace,
    IncrementAndClamp,
    DecrementAndClamp,
    Invert,
    IncrementAndWrap,
    DecrementAndWrap,
}

impl Into<vk::StencilOp> for StencilOp {
    fn into(self) -> vk::StencilOp {
        match self {
            StencilOp::Keep => vk::StencilOp::KEEP,
            StencilOp::Zero => vk::StencilOp::ZERO,
            StencilOp::Replace => vk::StencilOp::REPLACE,
            StencilOp::IncrementAndClamp => vk::StencilOp::INCREMENT_AND_CLAMP,
            StencilOp::DecrementAndClamp => vk::StencilOp::DECREMENT_AND_CLAMP,
            StencilOp::Invert => vk::StencilOp::INVERT,
            StencilOp::IncrementAndWrap => vk::StencilOp::INCREMENT_AND_WRAP,
            StencilOp::DecrementAndWrap => vk::StencilOp::DECREMENT_AND_WRAP,
        }
    }
}

impl Default for StencilOp {
    fn default() -> StencilOp {
        StencilOp::Keep
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct StencilOpState {
    pub fail_op: StencilOp,
    pub pass_op: StencilOp,
    pub depth_fail_op: StencilOp,
    pub compare_op: CompareOp,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

impl StencilOpState {
    pub fn as_builder(&self) -> vk::StencilOpStateBuilder {
        vk::StencilOpState::builder()
            .fail_op(self.fail_op.into())
            .pass_op(self.pass_op.into())
            .depth_fail_op(self.depth_fail_op.into())
            .compare_op(self.compare_op.into())
            .compare_mask(self.compare_mask)
            .write_mask(self.write_mask)
            .reference(self.reference)
    }
}

impl Into<vk::StencilOpState> for StencilOpState {
    fn into(self) -> vk::StencilOpState {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineDepthStencilState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
    pub depth_bounds_test_enable: bool,
    pub min_depth_bounds: Decimal,
    pub max_depth_bounds: Decimal,
    pub stencil_test_enable: bool,
    pub front: StencilOpState,
    pub back: StencilOpState,
}

impl PipelineDepthStencilState {
    pub fn as_builder(&self) -> vk::PipelineDepthStencilStateCreateInfoBuilder {
        vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(self.depth_test_enable)
            .depth_write_enable(self.depth_write_enable)
            .depth_compare_op(self.depth_compare_op.into())
            .depth_bounds_test_enable(self.depth_bounds_test_enable)
            .min_depth_bounds(self.min_depth_bounds.to_f32())
            .max_depth_bounds(self.max_depth_bounds.to_f32())
            .stencil_test_enable(self.stencil_test_enable)
            .front(self.front.clone().into())
            .back(self.back.clone().into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FixedFunctionState {
    pub input_assembly_state: PipelineInputAssemblyState,
    pub viewport_state: PipelineViewportState,
    pub rasterization_state: PipelineRasterizationState,
    pub multisample_state: PipelineMultisampleState,
    pub color_blend_state: PipelineColorBlendState,
    pub dynamic_state: PipelineDynamicState,
    pub depth_stencil_state: PipelineDepthStencilState,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShaderModuleMeta {
    pub stage: ShaderStage,
    pub entry_name: String,
    // Reference to shader is excluded
}

// These structs are candidates for removal because in practice you probably wouldn't want to
// embed a shader module's full data into a struct and pass it around
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShaderModuleCodeHash(u64);
impl ShaderModuleCodeHash {
    pub fn hash_shader_code(code: &Vec<u32>) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        let mut hasher = DefaultHasher::new();
        code.hash(&mut hasher);
        let code_hash = hasher.finish();
        ShaderModuleCodeHash(code_hash)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShaderModule {
    // Precalculate a hash so we can avoid hashing this blob of bytes at runtime
    pub code_hash: ShaderModuleCodeHash,
    pub code: Vec<u32>,
}

impl ShaderModule {
    pub fn new(code: Vec<u32>) -> Self {
        let code_hash = ShaderModuleCodeHash::hash_shader_code(&code);
        ShaderModule { code_hash, code }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FramebufferMeta {
    // Renderpass
    // Attachments (image view keys)
    pub width: u32,
    pub height: u32,
    pub layers: u32,
}
