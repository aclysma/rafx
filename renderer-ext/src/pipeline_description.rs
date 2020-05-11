
use ash::vk;
use crate::asset_storage::ResourceHandle;
use image2::Hash;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use fnv::FnvHashMap;
use std::collections::hash_map::Entry::Occupied;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::ffi::CString;
use serde::{Serialize, Deserialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DescriptorType {
    SAMPLER,
    COMBINED_IMAGE_SAMPLER,
    SAMPLED_IMAGE,
    STORAGE_IMAGE,
    UNIFORM_TEXEL_BUFFER,
    STORAGE_TEXEL_BUFFER,
    UNIFORM_BUFFER,
    STORAGE_BUFFER,
    UNIFORM_BUFFER_DYNAMIC,
    STORAGE_BUFFER_DYNAMIC,
    INPUT_ATTACHMENT,
}

impl Into<vk::DescriptorType> for DescriptorType {
    fn into(self) -> vk::DescriptorType {
        match self {
            SAMPLER => vk::DescriptorType::SAMPLER,
            COMBINED_IMAGE_SAMPLER => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            SAMPLED_IMAGE => vk::DescriptorType::SAMPLED_IMAGE,
            STORAGE_IMAGE => vk::DescriptorType::STORAGE_IMAGE,
            UNIFORM_TEXEL_BUFFER => vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
            STORAGE_TEXEL_BUFFER => vk::DescriptorType::STORAGE_TEXEL_BUFFER,
            UNIFORM_BUFFER => vk::DescriptorType::UNIFORM_BUFFER,
            STORAGE_BUFFER => vk::DescriptorType::STORAGE_BUFFER,
            UNIFORM_BUFFER_DYNAMIC => vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
            STORAGE_BUFFER_DYNAMIC => vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
            INPUT_ATTACHMENT => vk::DescriptorType::INPUT_ATTACHMENT,
        }
    }
}

impl Default for DescriptorType {
    fn default() -> Self {
        DescriptorType::SAMPLER
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderStageFlags {
    VERTEX,
    TESSELLATION_CONTROL,
    TESSELLATION_EVALUATION,
    GEOMETRY,
    FRAGMENT,
    COMPUTE,
    ALL_GRAPHICS,
    ALL,
}

impl Into<vk::ShaderStageFlags> for ShaderStageFlags {
    fn into(self) -> vk::ShaderStageFlags {
        match self {
            VERTEX => vk::ShaderStageFlags::VERTEX,
            TESSELLATION_CONTROL => vk::ShaderStageFlags::TESSELLATION_CONTROL,
            TESSELLATION_EVALUATION => vk::ShaderStageFlags::TESSELLATION_EVALUATION,
            GEOMETRY => vk::ShaderStageFlags::GEOMETRY,
            FRAGMENT => vk::ShaderStageFlags::FRAGMENT,
            COMPUTE => vk::ShaderStageFlags::COMPUTE,
            ALL_GRAPHICS => vk::ShaderStageFlags::ALL_GRAPHICS,
            ALL => vk::ShaderStageFlags::ALL,
        }
    }
}

impl Default for ShaderStageFlags {
    fn default() -> Self {
        ShaderStageFlags::VERTEX
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: ShaderStageFlags,
    //samplers: Vec<Sampler>,
}

impl DescriptorSetLayoutBinding {
    pub fn as_builder(&self) -> vk::DescriptorSetLayoutBindingBuilder {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(self.binding)
            .descriptor_type(self.descriptor_type.into())
            .descriptor_count(self.descriptor_count)
            .stage_flags(self.stage_flags.into())
    }
}

impl Into<vk::DescriptorSetLayoutBinding> for DescriptorSetLayoutBinding {
    fn into(self) -> vk::DescriptorSetLayoutBinding {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayout {
    pub descriptor_set_layout_bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayout {
    pub fn new() -> Self {
        DescriptorSetLayout {
            descriptor_set_layout_bindings: Default::default()
        }
    }
}















#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    MayAlias
}

impl Into<vk::AttachmentDescriptionFlags> for AttachmentDescriptionFlags {
    fn into(self) -> vk::AttachmentDescriptionFlags {
        match self {
            AttachmentDescriptionFlags::None => vk::AttachmentDescriptionFlags::empty(),
            AttachmentDescriptionFlags::MayAlias => vk::AttachmentDescriptionFlags::MAY_ALIAS,
        }
    }
}

pub struct SwapchainSurfaceInfo {
    pub surface_format: vk::SurfaceFormatKHR,
    pub extents: vk::Extent2D,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SampleCountFlags {
    SampleCount1,
    SampleCount2,
    SampleCount4,
    SampleCount8,
    SampleCount16,
    SampleCount32,
    SampleCount64
}

impl Into<vk::SampleCountFlags> for SampleCountFlags {
    fn into(self) -> vk::SampleCountFlags {
        match self {
            SampleCountFlags::SampleCount1 => vk::SampleCountFlags::TYPE_1,
            SampleCountFlags::SampleCount2 => vk::SampleCountFlags::TYPE_2,
            SampleCountFlags::SampleCount4 => vk::SampleCountFlags::TYPE_4,
            SampleCountFlags::SampleCount8 => vk::SampleCountFlags::TYPE_8,
            SampleCountFlags::SampleCount16 => vk::SampleCountFlags::TYPE_16,
            SampleCountFlags::SampleCount32 => vk::SampleCountFlags::TYPE_32,
            SampleCountFlags::SampleCount64 => vk::SampleCountFlags::TYPE_64,
        }
    }
}

impl Default for SampleCountFlags {
    fn default() -> Self {
        SampleCountFlags::SampleCount1
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentLoadOp {
    Load,
    Clear,
    DontCare
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentStoreOp {
    Store,
    DontCare
}

impl Into<vk::AttachmentStoreOp> for AttachmentStoreOp {
    fn into(self) -> vk::AttachmentStoreOp {
        match self {
            AttachmentStoreOp::Store => vk::AttachmentStoreOp::STORE,
            AttachmentStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineBindPoint {
    Compute,
    Graphics
}

impl Into<vk::PipelineBindPoint> for PipelineBindPoint {
    fn into(self) -> vk::PipelineBindPoint {
        match self {
            PipelineBindPoint::Compute => vk::PipelineBindPoint::COMPUTE,
            PipelineBindPoint::Graphics => vk::PipelineBindPoint::GRAPHICS,
        }
    }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageLayout {
    UNDEFINED,
    GENERAL,
    COLOR_ATTACHMENT_OPTIMAL,
    DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    DEPTH_STENCIL_READ_ONLY_OPTIMAL,
    SHADER_READ_ONLY_OPTIMAL,
    TRANSFER_SRC_OPTIMAL,
    TRANSFER_DST_OPTIMAL,
    PREINITIALIZED,
}

impl Into<vk::ImageLayout> for ImageLayout {
    fn into(self) -> vk::ImageLayout {
        match self {
            ImageLayout::UNDEFINED => vk::ImageLayout::UNDEFINED,
            ImageLayout::GENERAL => vk::ImageLayout::GENERAL,
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL => vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ImageLayout::TRANSFER_SRC_OPTIMAL => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            ImageLayout::TRANSFER_DST_OPTIMAL => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::PREINITIALIZED => vk::ImageLayout::PREINITIALIZED,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStageFlags {
    Empty,
    TOP_OF_PIPE,
    DRAW_INDIRECT,
    VERTEX_INPUT,
    VERTEX_SHADER,
    TESSELLATION_CONTROL_SHADER,
    TESSELLATION_EVALUATION_SHADER,
    GEOMETRY_SHADER,
    FRAGMENT_SHADER,
    EARLY_FRAGMENT_TESTS,
    LATE_FRAGMENT_TESTS,
    COLOR_ATTACHMENT_OUTPUT,
    COMPUTE_SHADER,
    TRANSFER,
    BOTTOM_OF_PIPE,
    HOST,
    ALL_GRAPHICS,
    ALL_COMMANDS,
}

impl Into<vk::PipelineStageFlags> for PipelineStageFlags {
    fn into(self) -> vk::PipelineStageFlags {
        match self {
            PipelineStageFlags::Empty => vk::PipelineStageFlags::empty(),
            PipelineStageFlags::TOP_OF_PIPE => vk::PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::DRAW_INDIRECT => vk::PipelineStageFlags::DRAW_INDIRECT,
            PipelineStageFlags::VERTEX_INPUT => vk::PipelineStageFlags::VERTEX_INPUT,
            PipelineStageFlags::VERTEX_SHADER => vk::PipelineStageFlags::VERTEX_SHADER,
            PipelineStageFlags::TESSELLATION_CONTROL_SHADER => vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER,
            PipelineStageFlags::TESSELLATION_EVALUATION_SHADER => vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER,
            PipelineStageFlags::GEOMETRY_SHADER => vk::PipelineStageFlags::GEOMETRY_SHADER,
            PipelineStageFlags::FRAGMENT_SHADER => vk::PipelineStageFlags::FRAGMENT_SHADER,
            PipelineStageFlags::EARLY_FRAGMENT_TESTS => vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            PipelineStageFlags::LATE_FRAGMENT_TESTS => vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT => vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::COMPUTE_SHADER => vk::PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::TRANSFER => vk::PipelineStageFlags::TRANSFER,
            PipelineStageFlags::BOTTOM_OF_PIPE => vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            PipelineStageFlags::HOST => vk::PipelineStageFlags::HOST,
            PipelineStageFlags::ALL_GRAPHICS => vk::PipelineStageFlags::ALL_GRAPHICS,
            PipelineStageFlags::ALL_COMMANDS => vk::PipelineStageFlags::ALL_COMMANDS,
        }
    }
}



#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessFlags {
    Empty,
    INDIRECT_COMMAND_READ,
    INDEX_READ,
    VERTEX_ATTRIBUTE_READ,
    UNIFORM_READ,
    INPUT_ATTACHMENT_READ,
    SHADER_READ,
    SHADER_WRITE,
    COLOR_ATTACHMENT_READ,
    COLOR_ATTACHMENT_WRITE,
    DEPTH_STENCIL_ATTACHMENT_READ,
    DEPTH_STENCIL_ATTACHMENT_WRITE,
    TRANSFER_READ,
    TRANSFER_WRITE,
    HOST_READ,
    HOST_WRITE,
    MEMORY_READ,
    MEMORY_WRITE,
}

impl Into<vk::AccessFlags> for AccessFlags {
    fn into(self) -> vk::AccessFlags {
        match self {
            AccessFlags::Empty => vk::AccessFlags::empty(),
            AccessFlags::INDIRECT_COMMAND_READ => vk::AccessFlags::INDIRECT_COMMAND_READ,
            AccessFlags::INDEX_READ => vk::AccessFlags::INDEX_READ,
            AccessFlags::VERTEX_ATTRIBUTE_READ => vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            AccessFlags::UNIFORM_READ => vk::AccessFlags::UNIFORM_READ,
            AccessFlags::INPUT_ATTACHMENT_READ => vk::AccessFlags::INPUT_ATTACHMENT_READ,
            AccessFlags::SHADER_READ => vk::AccessFlags::SHADER_READ,
            AccessFlags::SHADER_WRITE => vk::AccessFlags::SHADER_WRITE,
            AccessFlags::COLOR_ATTACHMENT_READ => vk::AccessFlags::COLOR_ATTACHMENT_READ,
            AccessFlags::COLOR_ATTACHMENT_WRITE => vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ => vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE => vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            AccessFlags::TRANSFER_READ => vk::AccessFlags::TRANSFER_READ,
            AccessFlags::TRANSFER_WRITE => vk::AccessFlags::TRANSFER_WRITE,
            AccessFlags::HOST_READ => vk::AccessFlags::HOST_READ,
            AccessFlags::HOST_WRITE => vk::AccessFlags::HOST_WRITE,
            AccessFlags::MEMORY_READ => vk::AccessFlags::MEMORY_READ,
            AccessFlags::MEMORY_WRITE => vk::AccessFlags::MEMORY_WRITE,
        }
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


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Format {
    MatchSwapchain,
    Raw(i32),
}

impl Format {
    fn as_vk_format(&self, swapchain_surface_info: &SwapchainSurfaceInfo) -> vk::Format {
        match self {
            Format::MatchSwapchain => swapchain_surface_info.surface_format.format,
            Format::Raw(raw) => vk::Format::from_raw(*raw)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachmentDescription {
    pub flags: AttachmentDescriptionFlags,
    pub format: Format,
    pub samples: SampleCountFlags,
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub stencil_load_op: AttachmentLoadOp,
    pub stencil_store_op: AttachmentStoreOp,
    pub initial_layout: ImageLayout,
    pub final_layout: ImageLayout
}

impl AttachmentDescription {
    pub fn as_builder(&self, swapchain_surface_info: &SwapchainSurfaceInfo) -> vk::AttachmentDescriptionBuilder {
        vk::AttachmentDescription::builder()
            .flags(self.flags.into())
            .format(self.format.as_vk_format(swapchain_surface_info))
            .samples(self.samples.into())
            .load_op(self.load_op.into())
            .store_op(self.store_op.into())
            .stencil_load_op(self.stencil_load_op.into())
            .stencil_store_op(self.stencil_store_op.into())
            .initial_layout(self.initial_layout.into())
            .final_layout(self.final_layout.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachmentReference {
    pub attachment: u32,
    pub layout: ImageLayout
}

impl AttachmentReference {
    pub fn as_builder(&self) -> vk::AttachmentReferenceBuilder {
        vk::AttachmentReference::builder()
            .attachment(self.attachment)
            .layout(self.layout.into())
    }
}

impl Into<vk::AttachmentReference> for AttachmentReference {
    fn into(self) -> vk::AttachmentReference {
        self.as_builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubpassDescription {
    pub pipeline_bind_point: PipelineBindPoint,
    pub input_attachments: Vec<AttachmentReference>,
    pub color_attachments: Vec<AttachmentReference>,
    pub resolve_attachments: Vec<AttachmentReference>,
    pub depth_stencil_attachment: AttachmentReference,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubpassDependencyIndex {
    External,
    Index(u32)
}

impl Into<u32> for SubpassDependencyIndex {
    fn into(self) -> u32 {
        match self {
            SubpassDependencyIndex::External => vk::SUBPASS_EXTERNAL,
            SubpassDependencyIndex::Index(index) => index,
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubpassDependency {
    pub src_subpass: SubpassDependencyIndex,
    pub dst_subpass: SubpassDependencyIndex,
    pub src_stage_mask: PipelineStageFlags,
    pub dst_stage_mask: PipelineStageFlags,
    pub src_access_mask: AccessFlags,
    pub dst_access_mask: AccessFlags,
    pub dependency_flags: DependencyFlags,
}

impl SubpassDependency {
    pub fn as_builder(&self) -> vk::SubpassDependencyBuilder {
        vk::SubpassDependency::builder()
            .src_subpass(self.src_subpass.into())
            .dst_subpass(self.dst_subpass.into())
            .src_stage_mask(self.src_stage_mask.into())
            .dst_stage_mask(self.dst_stage_mask.into())
            .src_access_mask(self.src_access_mask.into())
            .dst_access_mask(self.dst_access_mask.into())
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
    pub dependencies: Vec<SubpassDependency>
}








#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimitiveTopology {
    POINT_LIST,
    LINE_LIST,
    LINE_STRIP,
    TRIANGLE_LIST,
    TRIANGLE_STRIP,
    TRIANGLE_FAN,
    LINE_LIST_WITH_ADJACENCY,
    LINE_STRIP_WITH_ADJACENCY,
    TRIANGLE_LIST_WITH_ADJACENCY,
    TRIANGLE_STRIP_WITH_ADJACENCY,
    PATCH_LIST,
}

impl Into<vk::PrimitiveTopology> for PrimitiveTopology {
    fn into(self) -> vk::PrimitiveTopology {
        match self {
            PrimitiveTopology::POINT_LIST => vk::PrimitiveTopology::POINT_LIST,
            PrimitiveTopology::LINE_LIST => vk::PrimitiveTopology::LINE_LIST,
            PrimitiveTopology::LINE_STRIP => vk::PrimitiveTopology::LINE_STRIP,
            PrimitiveTopology::TRIANGLE_LIST => vk::PrimitiveTopology::TRIANGLE_LIST,
            PrimitiveTopology::TRIANGLE_STRIP => vk::PrimitiveTopology::TRIANGLE_STRIP,
            PrimitiveTopology::TRIANGLE_FAN => vk::PrimitiveTopology::TRIANGLE_FAN,
            PrimitiveTopology::LINE_LIST_WITH_ADJACENCY => vk::PrimitiveTopology::LINE_LIST_WITH_ADJACENCY,
            PrimitiveTopology::LINE_STRIP_WITH_ADJACENCY => vk::PrimitiveTopology::LINE_STRIP_WITH_ADJACENCY,
            PrimitiveTopology::TRIANGLE_LIST_WITH_ADJACENCY => vk::PrimitiveTopology::TRIANGLE_LIST_WITH_ADJACENCY,
            PrimitiveTopology::TRIANGLE_STRIP_WITH_ADJACENCY => vk::PrimitiveTopology::TRIANGLE_STRIP_WITH_ADJACENCY,
            PrimitiveTopology::PATCH_LIST => vk::PrimitiveTopology::PATCH_LIST,
        }
    }
}

impl Default for PrimitiveTopology {
    fn default() -> Self {
        PrimitiveTopology::POINT_LIST
    }
}



#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineInputAssemblyState {
    pub primitive_topology: PrimitiveTopology,
    pub primitive_restart_enable: bool
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
    Instance
}

impl Into<vk::VertexInputRate> for VertexInputRate {
    fn into(self) -> vk::VertexInputRate {
        match self {
            VertexInputRate::Vertex => vk::VertexInputRate::VERTEX,
            VertexInputRate::Instance => vk::VertexInputRate::INSTANCE
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VertexInputAttributeDescription {
    pub location: u32,
    pub binding: u32,
    pub format: Format,
    pub offset: u32,
}

impl VertexInputAttributeDescription {
    pub fn as_builder(&self, swapchain_surface_info: &SwapchainSurfaceInfo) -> vk::VertexInputAttributeDescriptionBuilder {
        vk::VertexInputAttributeDescription::builder()
            .location(self.location)
            .binding(self.binding)
            .format(self.format.as_vk_format(swapchain_surface_info))
            .offset(self.offset)
    }
}

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
    Raw(RectDecimal),
}

impl Dimensions {
    fn as_rect_f32(&self, swapchain_surface_info: &SwapchainSurfaceInfo) -> RectF32 {
        match self {
            Dimensions::MatchSwapchain => RectF32 {
                x: 0.0,
                y: 0.0,
                width: swapchain_surface_info.extents.width as f32,
                height: swapchain_surface_info.extents.height as f32,
            },
            Dimensions::Raw(rect) => RectF32 {
                x: rect.x.to_f32().unwrap(),
                y: rect.y.to_f32().unwrap(),
                width: rect.width.to_f32().unwrap(),
                height: rect.height.to_f32().unwrap(),
            }
        }
    }

    fn as_rect_i32(&self, swapchain_surface_info: &SwapchainSurfaceInfo) -> RectI32 {
        match self {
            Dimensions::MatchSwapchain => RectI32 {
                x: 0,
                y: 0,
                width: swapchain_surface_info.extents.width,
                height: swapchain_surface_info.extents.height,
            },
            Dimensions::Raw(rect) => RectI32 {
                x: rect.x.to_i32().unwrap(),
                y: rect.y.to_i32().unwrap(),
                width: rect.width.to_u32().unwrap(),
                height: rect.height.to_u32().unwrap(),
            }
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Viewport {
    pub dimensions: Dimensions,
    pub min_depth: Decimal,
    pub max_depth: Decimal,
}

impl Viewport {
    pub fn as_builder(&self, swapchain_surface_info: &SwapchainSurfaceInfo) -> vk::ViewportBuilder {
        let rect_f32 = self.dimensions.as_rect_f32(swapchain_surface_info);
        vk::Viewport::builder()
            .x(rect_f32.x)
            .y(rect_f32.y)
            .width(rect_f32.width)
            .height(rect_f32.height)
            .min_depth(self.min_depth.to_f32().unwrap())
            .max_depth(self.max_depth.to_f32().unwrap())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scissors {
    dimensions: Dimensions
}

impl Scissors {
    pub fn to_rect2d(&self, swapchain_surface_info: &SwapchainSurfaceInfo) -> vk::Rect2D {
        let rect_i32 = self.dimensions.as_rect_i32(swapchain_surface_info);
        vk::Rect2D {
            offset: vk::Offset2D { x: rect_i32.x, y: rect_i32.y },
            extent: vk::Extent2D { width: rect_i32.width, height: rect_i32.height },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineViewportState {
    pub viewports: Vec<Viewport>,
    pub scissors: Vec<Scissors>
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolygonMode {
    Fill,
    Line,
    Point
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
    FrontAndBack
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
            .depth_bias_constant_factor(self.depth_bias_constant_factor.to_f32().unwrap())
            .depth_bias_clamp(self.depth_bias_clamp.to_f32().unwrap())
            .depth_bias_slope_factor(self.depth_bias_slope_factor.to_f32().unwrap())
            .line_width(self.line_width.to_f32().unwrap())
    }
}

impl Into<vk::PipelineRasterizationStateCreateInfo> for PipelineRasterizationState {
    fn into(self) -> vk::PipelineRasterizationStateCreateInfo {
        self.as_builder().build()
    }
}





#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineMultisampleState {
    pub rasterization_samples: SampleCountFlags,
    pub sample_shading_enable: bool,
    pub min_sample_shading: Decimal,
    pub sample_mask: Vec<u32>,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}


impl PipelineMultisampleState {
    pub fn as_builder(&self) -> vk::PipelineMultisampleStateCreateInfoBuilder {
        vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(self.rasterization_samples.into())
            .sample_shading_enable(self.sample_shading_enable)
            .min_sample_shading(self.min_sample_shading.to_f32().unwrap())
            .sample_mask(&self.sample_mask)
            .alpha_to_coverage_enable(self.alpha_to_coverage_enable)
            .alpha_to_one_enable(self.alpha_to_one_enable)
    }
}







#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendFactor {
    ZERO,
    ONE,
    SRC_COLOR,
    ONE_MINUS_SRC_COLOR,
    DST_COLOR,
    ONE_MINUS_DST_COLOR,
    SRC_ALPHA,
    ONE_MINUS_SRC_ALPHA,
    DST_ALPHA,
    ONE_MINUS_DST_ALPHA,
    CONSTANT_COLOR,
    ONE_MINUS_CONSTANT_COLOR,
    CONSTANT_ALPHA,
    ONE_MINUS_CONSTANT_ALPHA,
    SRC_ALPHA_SATURATE,
    SRC1_COLOR,
    ONE_MINUS_SRC1_COLOR,
    SRC1_ALPHA,
    ONE_MINUS_SRC1_ALPHA,
}

impl Into<vk::BlendFactor> for BlendFactor {
    fn into(self) -> vk::BlendFactor {
        match self {
            BlendFactor::ZERO => vk::BlendFactor::ZERO,
            BlendFactor::ONE => vk::BlendFactor::ONE,
            BlendFactor::SRC_COLOR => vk::BlendFactor::SRC_COLOR,
            BlendFactor::ONE_MINUS_SRC_COLOR => vk::BlendFactor::ONE_MINUS_SRC_COLOR,
            BlendFactor::DST_COLOR => vk::BlendFactor::DST_COLOR,
            BlendFactor::ONE_MINUS_DST_COLOR => vk::BlendFactor::ONE_MINUS_DST_COLOR,
            BlendFactor::SRC_ALPHA => vk::BlendFactor::SRC_ALPHA,
            BlendFactor::ONE_MINUS_SRC_ALPHA => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            BlendFactor::DST_ALPHA => vk::BlendFactor::DST_ALPHA,
            BlendFactor::ONE_MINUS_DST_ALPHA => vk::BlendFactor::ONE_MINUS_DST_ALPHA,
            BlendFactor::CONSTANT_COLOR => vk::BlendFactor::CONSTANT_COLOR,
            BlendFactor::ONE_MINUS_CONSTANT_COLOR => vk::BlendFactor::ONE_MINUS_CONSTANT_COLOR,
            BlendFactor::CONSTANT_ALPHA => vk::BlendFactor::CONSTANT_ALPHA,
            BlendFactor::ONE_MINUS_CONSTANT_ALPHA => vk::BlendFactor::ONE_MINUS_CONSTANT_ALPHA,
            BlendFactor::SRC_ALPHA_SATURATE => vk::BlendFactor::SRC_ALPHA_SATURATE,
            BlendFactor::SRC1_COLOR => vk::BlendFactor::SRC1_COLOR,
            BlendFactor::ONE_MINUS_SRC1_COLOR => vk::BlendFactor::ONE_MINUS_SRC1_COLOR,
            BlendFactor::SRC1_ALPHA => vk::BlendFactor::SRC1_ALPHA,
            BlendFactor::ONE_MINUS_SRC1_ALPHA => vk::BlendFactor::ONE_MINUS_SRC1_ALPHA,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendOp {
    ADD,
    SUBTRACT,
    REVERSE_SUBTRACT,
    MIN,
    MAX,
}

impl Into<vk::BlendOp> for BlendOp {
    fn into(self) -> vk::BlendOp {
        match self {
            BlendOp::ADD => vk::BlendOp::ADD,
            BlendOp::SUBTRACT => vk::BlendOp::SUBTRACT,
            BlendOp::REVERSE_SUBTRACT => vk::BlendOp::REVERSE_SUBTRACT,
            BlendOp::MIN => vk::BlendOp::MIN,
            BlendOp::MAX => vk::BlendOp::MAX,
        }
    }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColorComponentFlags {
    red: bool,
    green: bool,
    blue: bool,
    alpha: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    CLEAR,
    AND,
    AND_REVERSE,
    COPY,
    AND_INVERTED,
    NO_OP,
    XOR,
    OR,
    NOR,
    EQUIVALENT,
    INVERT,
    OR_REVERSE,
    COPY_INVERTED,
    OR_INVERTED,
    NAND,
    SET,
}

impl Into<vk::LogicOp> for LogicOp {
    fn into(self) -> vk::LogicOp {
        match self {
            LogicOp::CLEAR => vk::LogicOp::CLEAR,
            LogicOp::AND => vk::LogicOp::AND,
            LogicOp::AND_REVERSE => vk::LogicOp::AND_REVERSE,
            LogicOp::COPY => vk::LogicOp::COPY,
            LogicOp::AND_INVERTED => vk::LogicOp::AND_INVERTED,
            LogicOp::NO_OP => vk::LogicOp::NO_OP,
            LogicOp::XOR => vk::LogicOp::XOR,
            LogicOp::OR => vk::LogicOp::OR,
            LogicOp::NOR => vk::LogicOp::NOR,
            LogicOp::EQUIVALENT => vk::LogicOp::EQUIVALENT,
            LogicOp::INVERT => vk::LogicOp::INVERT,
            LogicOp::OR_REVERSE => vk::LogicOp::OR_REVERSE,
            LogicOp::COPY_INVERTED => vk::LogicOp::COPY_INVERTED,
            LogicOp::OR_INVERTED => vk::LogicOp::OR_INVERTED,
            LogicOp::NAND => vk::LogicOp::NAND,
            LogicOp::SET => vk::LogicOp::SET,
        }
    }
}

impl Default for LogicOp {
    fn default() -> LogicOp {
        LogicOp::CLEAR
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
    pub fn blend_constants_as_f32(&self) -> [f32;4] {
        [
            self.blend_constants[0].to_f32().unwrap(),
            self.blend_constants[1].to_f32().unwrap(),
            self.blend_constants[2].to_f32().unwrap(),
            self.blend_constants[3].to_f32().unwrap(),
        ]
    }
}


// impl PipelineColorBlendState {
//     pub fn as_builder(&self) -> vk::PipelineColorBlendStateCreateInfoBuilder {
//         let blend_constants = [
//             self.blend_constants[0].to_f32().unwrap(),
//             self.blend_constants[1].to_f32().unwrap(),
//             self.blend_constants[2].to_f32().unwrap(),
//             self.blend_constants[3].to_f32().unwrap(),
//         ];
//
//         vk::PipelineColorBlendStateCreateInfo::builder()
//             .logic_op_enable(self.logic_op_enable)
//             .logic_op(self.logic_op.into())
//             .attachments(self.attachments)
//             .blend_constants(blend_constants)
//     }
// }

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DynamicState {
    VIEWPORT,
    SCISSOR,
    LINE_WIDTH,
    DEPTH_BIAS,
    BLEND_CONSTANTS,
    DEPTH_BOUNDS,
    STENCIL_COMPARE_MASK,
    STENCIL_WRITE_MASK,
    STENCIL_REFERENCE,
}

impl Into<vk::DynamicState> for DynamicState {
    fn into(self) -> vk::DynamicState {
        match self {
            DynamicState::VIEWPORT => vk::DynamicState::VIEWPORT,
            DynamicState::SCISSOR => vk::DynamicState::SCISSOR,
            DynamicState::LINE_WIDTH => vk::DynamicState::LINE_WIDTH,
            DynamicState::DEPTH_BIAS => vk::DynamicState::DEPTH_BIAS,
            DynamicState::BLEND_CONSTANTS => vk::DynamicState::BLEND_CONSTANTS,
            DynamicState::DEPTH_BOUNDS => vk::DynamicState::DEPTH_BOUNDS,
            DynamicState::STENCIL_COMPARE_MASK => vk::DynamicState::STENCIL_COMPARE_MASK,
            DynamicState::STENCIL_WRITE_MASK => vk::DynamicState::STENCIL_WRITE_MASK,
            DynamicState::STENCIL_REFERENCE => vk::DynamicState::STENCIL_REFERENCE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineDynamicState {
    pub dynamic_states: Vec<DynamicState>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FixedFunctionState {
    pub vertex_input_state: PipelineVertexInputState,
    pub input_assembly_state: PipelineInputAssemblyState,
    pub viewport_state: PipelineViewportState,
    pub rasterization_state: PipelineRasterizationState,
    pub multisample_state: PipelineMultisampleState,
    pub color_blend_state: PipelineColorBlendState,
    pub dynamic_state: PipelineDynamicState,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShaderModule {
    pub code: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PipelineShaderStage {
    pub stage: ShaderStageFlags,
    pub shader_module: ShaderModule,
    pub entry_name: CString
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PipelineShaderStages {
    pub stages: Vec<PipelineShaderStage>
}






#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct GraphicsPipeline {
    pub pipeline_layout: PipelineLayout,
    pub renderpass: RenderPass,
    pub fixed_function_state: FixedFunctionState,
    pub pipeline_shader_stages: PipelineShaderStages,
}



// // Applies globally
// let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo::builder()
// .attachments(&color_blend_attachment_states);
//
// let dynamic_state = vec![/*vk::DynamicState::SCISSOR*/];
// let dynamic_state_info =
// vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);


/*

    fn create_fixed_function_state<F: FnMut(&FixedFunctionState) -> VkResult<()>>(
        swapchain_info: &SwapchainInfo,
        mut f: F,
    ) -> VkResult<()> {
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<SpriteVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(SpriteVertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(SpriteVertex, tex_coord) as u32,
            },
            // vk::VertexInputAttributeDescription {
            //     binding: 0,
            //     location: 2,
            //     format: vk::Format::R8G8B8A8_UNORM,
            //     offset: offset_of!(Vertex, color) as u32,
            // },
        ];

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: swapchain_info.extents.width as f32,
            height: swapchain_info.extents.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain_info.extents.clone(),
        }];

        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0)
            .polygon_mode(vk::PolygonMode::FILL);

        // Skip depth/stencil testing

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Applies to the current framebuffer
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build()];

        // Applies globally
        let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&color_blend_attachment_states);

        let dynamic_state = vec![/*vk::DynamicState::SCISSOR*/];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

        let fixed_function_state = FixedFunctionState {
            vertex_input_assembly_state_info,
            vertex_input_state_info,
            viewport_state_info,
            rasterization_info,
            multisample_state_info,
            color_blend_state_info,
            dynamic_state_info,
        };

        f(&fixed_function_state)
    }
*/