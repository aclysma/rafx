use crate::graph::graph_buffer::PhysicalBufferId;
use crate::graph::graph_image::{PhysicalImageId, PhysicalImageViewId, VirtualImageId};
use crate::graph::graph_node::RenderGraphNodeName;
use crate::graph::{RenderGraphImageUsageId, RenderGraphNodeId};
use crate::vk_description as dsc;
use ash::vk;
use fnv::FnvHashMap;
use std::sync::Arc;

/// Represents the invalidate or flush of a RenderGraphPassImageBarriers
#[derive(Debug)]
pub struct RenderGraphImageBarrier {
    pub(super) access_flags: vk::AccessFlags,
    pub(super) stage_flags: vk::PipelineStageFlags,
}

impl Default for RenderGraphImageBarrier {
    fn default() -> Self {
        RenderGraphImageBarrier {
            access_flags: vk::AccessFlags::empty(),
            stage_flags: vk::PipelineStageFlags::empty(),
        }
    }
}

/// Information provided per image used in a pass to properly synchronize access to it from
/// different passes
#[derive(Debug)]
pub struct RenderGraphPassImageBarriers {
    pub(super) invalidate: RenderGraphImageBarrier,
    pub(super) flush: RenderGraphImageBarrier,
    pub(super) layout: vk::ImageLayout,
    pub(super) used_by_sampling: bool,
}

impl RenderGraphPassImageBarriers {
    pub(super) fn new(layout: vk::ImageLayout) -> Self {
        RenderGraphPassImageBarriers {
            flush: Default::default(),
            invalidate: Default::default(),
            layout,
            used_by_sampling: false,
        }
    }
}

/// All the barriers required for a single node (i.e. subpass). Nodes represent passes that may be
/// merged to be subpasses within a single pass.
#[derive(Debug)]
pub struct RenderGraphNodeResourceBarriers {
    pub(super) image_barriers: FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers>,
    pub(super) buffer_barriers: FnvHashMap<PhysicalBufferId, RenderGraphPassBufferBarriers>,
}

/// Represents the invalidate or flush of a RenderGraphPassBufferBarriers
#[derive(Debug)]
pub struct RenderGraphBufferBarrier {
    pub(super) access_flags: vk::AccessFlags,
    pub(super) stage_flags: vk::PipelineStageFlags,
}

impl Default for RenderGraphBufferBarrier {
    fn default() -> Self {
        RenderGraphBufferBarrier {
            access_flags: vk::AccessFlags::empty(),
            stage_flags: vk::PipelineStageFlags::empty(),
        }
    }
}

/// Information provided per buffer used in a pass to properly synchronize access to it from
/// different passes
#[derive(Debug)]
pub struct RenderGraphPassBufferBarriers {
    pub(super) invalidate: RenderGraphBufferBarrier,
    pub(super) flush: RenderGraphBufferBarrier,
}

impl RenderGraphPassBufferBarriers {
    pub(super) fn new() -> Self {
        RenderGraphPassBufferBarriers {
            flush: Default::default(),
            invalidate: Default::default(),
        }
    }
}

/// All the barriers required for a single node (i.e. subpass). Nodes represent passes that may be
/// merged to be subpasses within a single pass.
#[derive(Debug)]
pub struct RenderGraphNodeBufferBarriers {
    pub(super) barriers: FnvHashMap<PhysicalBufferId, RenderGraphPassBufferBarriers>,
}

const MAX_COLOR_ATTACHMENTS: usize = 4;
const MAX_RESOLVE_ATTACHMENTS: usize = 4;

/// Metadata for a subpass
#[derive(Debug)]
pub struct RenderGraphSubpass {
    pub(super) node: RenderGraphNodeId,

    pub(super) color_attachments: [Option<usize>; MAX_COLOR_ATTACHMENTS], // could ref back to node
    pub(super) resolve_attachments: [Option<usize>; MAX_RESOLVE_ATTACHMENTS],
    pub(super) depth_attachment: Option<usize>,
}

/// Clear value for either a color attachment or depth/stencil attachment
#[derive(Clone)]
pub enum AttachmentClearValue {
    Color(vk::ClearColorValue),
    DepthStencil(vk::ClearDepthStencilValue),
}

impl std::fmt::Debug for AttachmentClearValue {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            AttachmentClearValue::Color(_) => {
                f.debug_struct("AttachmentClearValue(Color)").finish()
            }
            AttachmentClearValue::DepthStencil(value) => f
                .debug_struct("AttachmentClearValue(DepthStencil)")
                .field("depth", &value.depth)
                .field("stencil", &value.stencil)
                .finish(),
        }
    }
}

impl Into<vk::ClearValue> for AttachmentClearValue {
    fn into(self) -> vk::ClearValue {
        match self {
            AttachmentClearValue::Color(color) => vk::ClearValue { color },
            AttachmentClearValue::DepthStencil(depth_stencil) => vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: depth_stencil.depth,
                    stencil: depth_stencil.stencil,
                },
            },
        }
    }
}

/// Attachment for a render pass
#[derive(Debug)]
pub struct RenderGraphPassAttachment {
    pub(super) usage: RenderGraphImageUsageId,
    pub(super) virtual_image: VirtualImageId,
    pub(super) image: Option<PhysicalImageId>,
    pub(super) image_view: Option<PhysicalImageViewId>,
    pub(super) load_op: vk::AttachmentLoadOp,
    pub(super) stencil_load_op: vk::AttachmentLoadOp,
    pub(super) store_op: vk::AttachmentStoreOp,
    pub(super) stencil_store_op: vk::AttachmentStoreOp,
    pub(super) clear_color: Option<AttachmentClearValue>,
    pub(super) format: vk::Format,
    pub(super) samples: vk::SampleCountFlags,
    pub(super) initial_layout: dsc::ImageLayout,
    pub(super) final_layout: dsc::ImageLayout,
}

impl RenderGraphPassAttachment {
    pub(super) fn create_attachment_description(&self) -> dsc::AttachmentDescription {
        dsc::AttachmentDescription {
            flags: dsc::AttachmentDescriptionFlags::None,
            format: dsc::AttachmentFormat::Format(self.format.into()),
            samples: dsc::SampleCountFlags::from_vk_sample_count_flags(self.samples).unwrap(),
            load_op: self.load_op.into(),
            store_op: self.store_op.into(),
            stencil_load_op: self.stencil_load_op.into(),
            stencil_store_op: self.stencil_store_op.into(),
            initial_layout: self.initial_layout,
            final_layout: self.final_layout,
        }
    }
}

#[derive(Debug)]
pub struct PrepassBarrier {
    pub src_stage: vk::PipelineStageFlags,
    pub dst_stage: vk::PipelineStageFlags,
    pub image_barriers: Vec<PrepassImageBarrier>,
    pub buffer_barriers: Vec<PrepassBufferBarrier>,
}

#[derive(Debug)]
pub struct PrepassImageBarrier {
    pub src_access: vk::AccessFlags,
    pub dst_access: vk::AccessFlags,
    pub old_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
    pub src_queue_family_index: u32,
    pub dst_queue_family_index: u32,
    pub image: PhysicalImageId,
    pub subresource_range: dsc::ImageSubresourceRange,
}

#[derive(Debug)]
pub struct PrepassBufferBarrier {
    pub src_access: vk::AccessFlags,
    pub dst_access: vk::AccessFlags,
    pub src_queue_family_index: u32,
    pub dst_queue_family_index: u32,
    pub buffer: PhysicalBufferId,
    pub size: u64,
}

/// Metadata required to create a renderpass
#[derive(Debug, Default)]
pub struct RenderGraphRenderPass {
    pub(super) nodes: Vec<RenderGraphNodeId>,
    pub(super) attachments: Vec<RenderGraphPassAttachment>,
    pub(super) subpasses: Vec<RenderGraphSubpass>,

    // For when we want to do layout transitions on non-attachments
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
    pub(super) extents: Option<vk::Extent2D>,
}

#[derive(Debug)]
pub struct RenderGraphComputePass {
    pub(super) node: RenderGraphNodeId,
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
}

#[derive(Debug)]
pub enum RenderGraphPass {
    Renderpass(RenderGraphRenderPass),
    Compute(RenderGraphComputePass),
}

impl RenderGraphPass {
    pub fn nodes(&self) -> &[RenderGraphNodeId] {
        match self {
            RenderGraphPass::Renderpass(renderpass) => renderpass.nodes.as_slice(),
            RenderGraphPass::Compute(compute_pass) => std::slice::from_ref(&compute_pass.node),
        }
    }

    pub fn set_pre_pass_barrier(
        &mut self,
        barrier: PrepassBarrier,
    ) {
        match self {
            RenderGraphPass::Renderpass(renderpass) => renderpass.pre_pass_barrier = Some(barrier),
            RenderGraphPass::Compute(compute_pass) => {
                compute_pass.pre_pass_barrier = Some(barrier);
            }
        }
    }
}

pub struct RenderGraphOutputRenderPass {
    pub(super) subpass_nodes: Vec<RenderGraphNodeId>,
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
    pub(super) debug_name: Option<RenderGraphNodeName>,
    pub(super) description: Arc<dsc::RenderPass>,
    pub(super) attachment_images: Vec<PhysicalImageViewId>,
    pub(super) clear_values: Vec<vk::ClearValue>,
    pub(super) extents: vk::Extent2D,
}

impl std::fmt::Debug for RenderGraphOutputRenderPass {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RenderGraphOutputRenderPass")
            .field("description", &self.description)
            .field("attachment_images", &self.attachment_images)
            .field("extents", &self.extents)
            .finish()
    }
}

#[derive(Debug)]
pub struct RenderGraphOutputComputePass {
    pub(super) node: RenderGraphNodeId,
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
    pub(super) debug_name: Option<RenderGraphNodeName>,
}

#[derive(Debug)]
pub enum RenderGraphOutputPass {
    Renderpass(RenderGraphOutputRenderPass),
    Compute(RenderGraphOutputComputePass),
}

impl RenderGraphOutputPass {
    pub fn nodes(&self) -> &[RenderGraphNodeId] {
        match self {
            RenderGraphOutputPass::Renderpass(pass) => &pass.subpass_nodes,
            RenderGraphOutputPass::Compute(pass) => std::slice::from_ref(&pass.node),
        }
    }

    pub fn pre_pass_barrier(&self) -> Option<&PrepassBarrier> {
        match self {
            RenderGraphOutputPass::Renderpass(pass) => pass.pre_pass_barrier.as_ref(),
            RenderGraphOutputPass::Compute(pass) => pass.pre_pass_barrier.as_ref(),
        }
    }

    pub fn debug_name(&self) -> Option<RenderGraphNodeName> {
        match self {
            RenderGraphOutputPass::Renderpass(pass) => pass.debug_name,
            RenderGraphOutputPass::Compute(pass) => pass.debug_name,
        }
    }
}
