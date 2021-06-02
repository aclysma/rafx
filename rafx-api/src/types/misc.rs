#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

use crate::{RafxBuffer, RafxSampler, RafxTexture};
use rafx_base::DecimalF32;
use std::hash::{Hash, Hasher};

/// Controls if validation is enabled or not. The requirements/behaviors of validation is
/// API-specific.
#[derive(Copy, Clone, Debug)]
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
        #[cfg(debug_assertions)]
        let validation_mode = RafxValidationMode::EnabledIfAvailable;
        #[cfg(not(debug_assertions))]
        let validation_mode = RafxValidationMode::Disabled;

        validation_mode
    }
}

/// Information about the device, mostly limits, requirements (like memory alignment), and flags to
/// indicate whether certain features are supported
pub struct RafxDeviceInfo {
    pub supports_multithreaded_usage: bool,

    pub min_uniform_buffer_offset_alignment: u32,
    pub min_storage_buffer_offset_alignment: u32,
    pub upload_buffer_texture_alignment: u32,
    pub upload_buffer_texture_row_alignment: u32,

    // Requires iOS 14.0, macOS 10.12
    pub supports_clamp_to_border_color: bool,

    pub max_vertex_attribute_count: u32,
    //max_vertex_input_binding_count: u32,
    // max_root_signature_dwords: u32,
    // wave_lane_count: u32,
    // wave_ops_support_flags: u32,
    // gpu_vendor_preset: u32,
    // metal_argument_buffer_max_textures: u32,
    // metal_heaps: u32,
    // metal_placement_heaps: u32,
    // metal_draw_index_vertex_offset_supported: bool,
}

/// Used to indicate which type of queue to use. Some operations require certain types of queues.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RafxQueueType {
    /// Graphics queues generally supports all operations and are a safe default choice
    Graphics,

    /// Compute queues can be used for compute-based work.
    Compute,

    /// Transfer queues are generally limited to basic operations like copying data from buffers
    /// to images.
    Transfer,
}

/// The color space an image data is in. The correct color space often varies between texture types
/// (like normal maps vs. albedo maps).
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
    /// The current state of a resource. When an operation is performed that references a resource,
    /// it must be in the correct state. Resources are moved between state using barriers.
    pub struct RafxResourceState: u32 {
        const UNDEFINED = 0;
        const VERTEX_AND_CONSTANT_BUFFER = 0x1;
        const INDEX_BUFFER = 0x2;
        /// Similar to vulkan's COLOR_ATTACHMENT_OPTIMAL image layout
        const RENDER_TARGET = 0x4;
        const UNORDERED_ACCESS = 0x8;
        /// Similar to vulkan's DEPTH_STENCIL_ATTACHMENT_OPTIMAL image layout
        const DEPTH_WRITE = 0x10;
        const DEPTH_READ = 0x20;
        const NON_PIXEL_SHADER_RESOURCE = 0x40;
        const PIXEL_SHADER_RESOURCE = 0x80;
        /// Similar to vulkan's SHADER_READ_ONLY_OPTIMAL image layout
        const SHADER_RESOURCE = 0x40 | 0x80;
        const STREAM_OUT = 0x100;
        const INDIRECT_ARGUMENT = 0x200;
        /// Similar to vulkan's TRANSFER_DST_OPTIMAL image layout
        const COPY_DST = 0x400;
        /// Similar to vulkan's TRANSFER_SRC_OPTIMAL image layout
        const COPY_SRC = 0x800;
        const GENERIC_READ = (((((0x1 | 0x2) | 0x40) | 0x80) | 0x200) | 0x800);
        /// Similar to vulkan's PRESENT_SRC_KHR image layout
        const PRESENT = 0x1000;
        /// Similar to vulkan's COMMON image layout
        const COMMON = 0x2000;
    }
}

/// A 2d size for windows, textures, etc.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RafxExtents2D {
    pub width: u32,
    pub height: u32,
}

/// A 3d size for windows, textures, etc.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RafxExtents3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl RafxExtents3D {
    pub fn to_2d(self) -> RafxExtents2D {
        RafxExtents2D {
            width: self.width,
            height: self.height,
        }
    }
}

/// Number of MSAA samples to use. 1xMSAA and 4xMSAA are most broadly supported
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

bitflags::bitflags! {
    /// Indicates how a resource will be used. In some cases, multiple flags are allowed.
    #[derive(Default)]
    #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
    pub struct RafxResourceType: u32 {
        const UNDEFINED = 0;
        const SAMPLER = 1<<0;
        /// Similar to DX12 SRV and vulkan SAMPLED image usage flag and SAMPLED_IMAGE descriptor type
        const TEXTURE = 1<<1;
        /// Similar to DX12 UAV and vulkan STORAGE image usage flag and STORAGE_IMAGE descriptor type
        const TEXTURE_READ_WRITE = 1<<2;
        /// Similar to DX12 SRV and vulkan STORAGE_BUFFER descriptor type
        const BUFFER = 1<<3;
        /// Similar to DX12 UAV and vulkan STORAGE_BUFFER descriptor type
        const BUFFER_READ_WRITE = 1<<5;
        /// Similar to vulkan UNIFORM_BUFFER descriptor type
        const UNIFORM_BUFFER = 1<<7;
        // Push constant / Root constant
        /// Similar to DX12 root constants and vulkan push constants
        const ROOT_CONSTANT = 1<<8;
        // Input assembler
        /// Similar to vulkan VERTEX_BUFFER buffer usage flag
        const VERTEX_BUFFER = 1<<9;
        /// Similar to vulkan INDEX_BUFFER buffer usage flag
        const INDEX_BUFFER = 1<<10;
        /// Similar to vulkan INDIRECT_BUFFER buffer usage flag
        const INDIRECT_BUFFER = 1<<11;
        // Cubemap SRV
        /// Similar to vulkan's CUBE_COMPATIBLE image create flag and metal's Cube texture type
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
        // Render target types
        /// A color attachment in a renderpass
        const RENDER_TARGET_COLOR = 1<<23;
        /// A depth/stencil attachment in a renderpass
        const RENDER_TARGET_DEPTH_STENCIL = 1<<24;
    }
}

impl RafxResourceType {
    pub fn is_uniform_buffer(self) -> bool {
        self.intersects(RafxResourceType::UNIFORM_BUFFER)
    }

    pub fn is_storage_buffer(self) -> bool {
        self.intersects(RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE)
    }

    pub fn is_render_target(self) -> bool {
        self.intersects(
            RafxResourceType::RENDER_TARGET_COLOR | RafxResourceType::RENDER_TARGET_DEPTH_STENCIL,
        )
    }

    pub fn is_texture(self) -> bool {
        self.intersects(RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE)
    }
}

bitflags::bitflags! {
    /// Flags for enabling/disabling color channels, used with `RafxBlendState`
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

/// Indicates how the memory will be accessed and affects where in memory it needs to be allocated.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RafxMemoryUsage {
    Unknown,

    /// The memory is only accessed by the GPU
    GpuOnly,

    /// The memory is only accessed by the CPU
    CpuOnly,

    /// The memory is written by the CPU and read by the GPU
    CpuToGpu,

    /// The memory is written by the GPU and read by the CPU
    GpuToCpu,
}

/// Indicates the result of presenting a swapchain image
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RafxPresentSuccessResult {
    /// The image was shown and the swapchain can continue to be used.
    Success,

    /// The image was shown and the swapchain can continue to be used. However, this result also
    /// hints that there is a more optimal configuration for the swapchain to be in. This is vague
    /// because the precise meaning varies between platform. For example, windows may return this
    /// when the application is minimized.
    SuccessSuboptimal,

    // While this is an "error" being returned as success, it is expected and recoverable while
    // other errors usually aren't. This way the ? operator can still be used to bail out the
    // unrecoverable errors and the different flavors of "success" should be explicitly handled
    // in a match
    /// Indicates that the swapchain can no longer be used
    DeviceReset,
}

/// Indicates the current state of a fence.
#[derive(Clone, Copy, PartialEq, Debug)]
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
    /// Indicates what render targets are affected by a blend state
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
    /// Indicates a particular stage of a shader, or set of stages in a shader. Similar to
    /// VkShaderStageFlagBits
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

/// Contains all the individual stages
pub const ALL_SHADER_STAGE_FLAGS: [RafxShaderStageFlags; 6] = [
    RafxShaderStageFlags::VERTEX,
    RafxShaderStageFlags::TESSELLATION_CONTROL,
    RafxShaderStageFlags::TESSELLATION_EVALUATION,
    RafxShaderStageFlags::GEOMETRY,
    RafxShaderStageFlags::FRAGMENT,
    RafxShaderStageFlags::COMPUTE,
];

/// Indicates the type of pipeline, roughly corresponds with RafxQueueType
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RafxPipelineType {
    Graphics = 0,
    Compute = 1,
}

/// Affects how quickly vertex attributes are consumed from buffers, similar to VkVertexInputRate
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

/// Determines if the contents of an image attachment in a renderpass begins with its previous
/// contents, a clear value, or undefined data. Similar to VkAttachmentLoadOp
#[derive(Copy, Clone, Debug, Hash, PartialEq)]
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

/// Determines if the contents of an image attachment in a rander pass will store the resulting
/// state for use after the render pass
#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub enum RafxStoreOp {
    /// Do not store the image, leaving the contents of it undefined
    DontCare,

    /// Persist the image's content after a render pass completes
    Store,
}

impl Default for RafxStoreOp {
    fn default() -> Self {
        RafxStoreOp::Store
    }
}

/// How to intepret vertex data into a form of geometry. Similar to VkPrimitiveTopology
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

/// The size of index buffer elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RafxIndexType {
    Uint32,
    Uint16,
}

impl Default for RafxIndexType {
    fn default() -> Self {
        RafxIndexType::Uint32
    }
}

/// Affects blending. Similar to VkBlendFactor
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

/// Affects blending. Similar to VkBlendOp
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

/// Affects depth testing and sampling. Similar to VkCompareOp
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

/// Similar to VkStencilOp
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

/// Determines if we cull polygons that are front-facing or back-facing. Facing direction is
/// determined by RafxFrontFace, sometimes called "winding order". Similar to VkCullModeFlags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxCullMode {
    None,
    Back,
    Front,
}

impl Default for RafxCullMode {
    fn default() -> Self {
        RafxCullMode::None
    }
}

/// Determines what winding order is considerered the front face of a polygon. Similar to
/// VkFrontFace
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

/// Whether to fill in polygons or not. Similar to VkPolygonMode
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

/// Filtering method when sampling. Similar to VkFilter
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxFilterType {
    /// Finds the closest value in the texture and uses it. Commonly used for "pixel-perfect"
    /// assets.
    Nearest,

    /// "Averages" color values of the texture. A common choice for most cases but may make some
    /// "pixel-perfect" assets appear blurry
    Linear,
}

impl Default for RafxFilterType {
    fn default() -> Self {
        RafxFilterType::Nearest
    }
}

/// Affects image sampling, particularly for UV coordinates outside the [0, 1] range. Similar to
/// VkSamplerAddressMode
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

/// Similar to VkSamplerMipmapMode
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

/// A clear value for color attachments
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

/// A clear values for depth/stencil attachments. One or both values may be used depending on the
/// format of the attached image
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

/// Determines if a barrier is transferring a resource from one queue to another.
pub enum RafxBarrierQueueTransition {
    /// No queue transition will take place
    None,

    /// A barrier for the "sending" queue. Contains the "receiving" queue. (the "sending" queue is
    /// inferred by the queue on which the barrier is submitted)
    ReleaseTo(RafxQueueType),

    /// A barrier for the "receiving" queue. Contains the "sending" queue. (the "receiving" queue is
    /// inferred by the queue on which the barrier is submitted)
    AcquireFrom(RafxQueueType),
}

impl Default for RafxBarrierQueueTransition {
    fn default() -> Self {
        RafxBarrierQueueTransition::None
    }
}

/// A memory barrier for buffers. This is used to transition buffers between resource states and
/// possibly from one queue to another
pub struct RafxBufferBarrier<'a> {
    pub buffer: &'a RafxBuffer,
    pub src_state: RafxResourceState,
    pub dst_state: RafxResourceState,
    pub queue_transition: RafxBarrierQueueTransition,
}

/// A memory barrier for textures. This is used to transition textures between resource states and
/// possibly from one queue to another.
pub struct RafxTextureBarrier<'a> {
    pub texture: &'a RafxTexture,
    pub src_state: RafxResourceState,
    pub dst_state: RafxResourceState,
    pub queue_transition: RafxBarrierQueueTransition,
    /// If set, only the specified array element is included
    pub array_slice: Option<u16>,
    /// If set, only the specified mip level is included
    pub mip_slice: Option<u8>,
}

impl<'a> RafxTextureBarrier<'a> {
    /// Creates a simple state transition
    pub fn state_transition(
        texture: &'a RafxTexture,
        src_state: RafxResourceState,
        dst_state: RafxResourceState,
    ) -> RafxTextureBarrier {
        RafxTextureBarrier {
            texture,
            src_state,
            dst_state,
            queue_transition: RafxBarrierQueueTransition::None,
            array_slice: None,
            mip_slice: None,
        }
    }
}

/// Represents an image owned by the swapchain
#[derive(Clone)]
pub struct RafxSwapchainImage {
    pub texture: RafxTexture,
    pub swapchain_image_index: u32,
}

/// A color render target bound during a renderpass
#[derive(Debug)]
pub struct RafxColorRenderTargetBinding<'a> {
    pub texture: &'a RafxTexture,
    pub load_op: RafxLoadOp,
    pub store_op: RafxStoreOp,
    pub mip_slice: Option<u8>,
    pub array_slice: Option<u16>,
    pub clear_value: RafxColorClearValue,
    pub resolve_target: Option<&'a RafxTexture>,
    pub resolve_store_op: RafxStoreOp,
    pub resolve_mip_slice: Option<u8>,
    pub resolve_array_slice: Option<u16>,
}

/// A depth/stencil render target to be bound during a renderpass
#[derive(Debug)]
pub struct RafxDepthStencilRenderTargetBinding<'a> {
    pub texture: &'a RafxTexture,
    pub depth_load_op: RafxLoadOp,
    pub stencil_load_op: RafxLoadOp,
    pub depth_store_op: RafxStoreOp,
    pub stencil_store_op: RafxStoreOp,
    pub mip_slice: Option<u8>,
    pub array_slice: Option<u16>,
    pub clear_value: RafxDepthStencilClearValue,
}

/// A vertex buffer to be bound during a renderpass
pub struct RafxVertexBufferBinding<'a> {
    pub buffer: &'a RafxBuffer,
    pub byte_offset: u64,
}

/// An index buffer to be bound during a renderpass
pub struct RafxIndexBufferBinding<'a> {
    pub buffer: &'a RafxBuffer,
    pub byte_offset: u64,
    pub index_type: RafxIndexType,
}

/// Parameters for copying a buffer to a texture
#[derive(Default)]
pub struct RafxCmdCopyBufferToTextureParams {
    pub buffer_offset: u64,
    pub array_layer: u16,
    pub mip_level: u8,
}

/// Parameters for blitting one image to another (vulkan backend only)
pub struct RafxCmdBlitParams {
    pub src_state: RafxResourceState,
    pub dst_state: RafxResourceState,
    pub src_extents: [RafxExtents3D; 2],
    pub dst_extents: [RafxExtents3D; 2],
    pub src_mip_level: u8,
    pub dst_mip_level: u8,
    pub array_slices: Option<[u16; 2]>,
}

/// A rafx-specific index that refers to a particular binding. Instead of doing name/binding lookups
/// every frame, query the descriptor index during startup and use it instead. This is a more
/// efficient way to address descriptors.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct RafxDescriptorIndex(pub(crate) u32);

/// Selects a particular descriptor in a descriptor set
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

/// Used when binding buffers to descriptor sets
#[derive(Default, Clone, Copy, Debug)]
pub struct RafxOffsetSize {
    pub byte_offset: u64,
    pub size: u64,
}

/// Specifies what value to assign to a descriptor set
#[derive(Default, Debug)]
pub struct RafxDescriptorElements<'a> {
    pub textures: Option<&'a [&'a RafxTexture]>,
    pub samplers: Option<&'a [&'a RafxSampler]>,
    pub buffers: Option<&'a [&'a RafxBuffer]>,
    pub buffer_offset_sizes: Option<&'a [RafxOffsetSize]>,
}

/// Used when binding a texture to select between different ways to bind the texture
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RafxTextureBindType {
    // Color or depth only
    Srv,
    // stencil?
    SrvStencil,
    // Bind all mip levels of the 0th provided texture
    UavMipChain,
    // Bind a particular mip slice of all provided textures
    UavMipSlice(u32),
}

/// Describes how to update a single descriptor
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
