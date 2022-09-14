use crate::graph::graph_buffer::PhysicalBufferId;
use crate::graph::graph_image::{PhysicalImageId, PhysicalImageViewId, VirtualImageId};
use crate::graph::graph_node::RenderGraphNodeName;
use crate::graph::{RenderGraphImageUsageId, RenderGraphNodeId};
use crate::GraphicsPipelineRenderTargetMeta;
use fnv::FnvHashMap;
use rafx_api::{
    RafxColorClearValue, RafxDepthStencilClearValue, RafxFormat, RafxLoadOp, RafxResourceState,
    RafxSampleCount, RafxStoreOp,
};

/// Information provided per image used in a pass to properly synchronize access to it from
/// different passes
#[derive(Debug)]
pub struct RenderGraphPassImageBarriers {
    pub(super) resource_state: RafxResourceState,
}

impl RenderGraphPassImageBarriers {
    pub(super) fn new(resource_state: RafxResourceState) -> Self {
        RenderGraphPassImageBarriers { resource_state }
    }
}

/// All the barriers required for a single node (i.e. subpass). Nodes represent passes that may be
/// merged to be subpasses within a single pass.
#[derive(Debug)]
pub struct RenderGraphNodeResourceBarriers {
    pub(super) image_barriers: FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers>,
    pub(super) buffer_barriers: FnvHashMap<PhysicalBufferId, RenderGraphPassBufferBarriers>,
}

/// Information provided per buffer used in a pass to properly synchronize access to it from
/// different passes
#[derive(Debug)]
pub struct RenderGraphPassBufferBarriers {
    pub(super) resource_state: RafxResourceState,
}

impl RenderGraphPassBufferBarriers {
    pub(super) fn new(resource_state: RafxResourceState) -> Self {
        RenderGraphPassBufferBarriers { resource_state }
    }
}

/// All the barriers required for a single node (i.e. subpass). Nodes represent passes that may be
/// merged to be subpasses within a single pass.
#[derive(Debug)]
pub struct RenderGraphNodeBufferBarriers {
    #[allow(unused)]
    pub(super) barriers: FnvHashMap<PhysicalBufferId, RenderGraphPassBufferBarriers>,
}

pub const MAX_COLOR_ATTACHMENTS: usize = 4;
pub const MAX_RESOLVE_ATTACHMENTS: usize = 4;

/// Clear value for either a color attachment or depth/stencil attachment
#[derive(Clone)]
pub enum AttachmentClearValue {
    Color(RafxColorClearValue),
    DepthStencil(RafxDepthStencilClearValue),
}

impl AttachmentClearValue {
    pub fn to_color_clear_value(self) -> RafxColorClearValue {
        match self {
            AttachmentClearValue::Color(color) => color,
            _ => panic!("wrong color type"),
        }
    }

    pub fn to_depth_stencil_clear_value(self) -> RafxDepthStencilClearValue {
        match self {
            AttachmentClearValue::DepthStencil(color) => color,
            _ => panic!("wrong color type"),
        }
    }
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

/// Attachment for a render pass
#[derive(Debug)]
pub struct RenderGraphPassAttachment {
    pub(super) usage: RenderGraphImageUsageId,
    pub(super) virtual_image: VirtualImageId,
    pub(super) image: Option<PhysicalImageId>,
    pub(super) image_view: Option<PhysicalImageViewId>,
    pub(super) load_op: RafxLoadOp,
    pub(super) stencil_load_op: RafxLoadOp,
    pub(super) store_op: RafxStoreOp,
    pub(super) stencil_store_op: RafxStoreOp,
    pub(super) clear_color: Option<AttachmentClearValue>,
    //pub(super) array_slice: Option<u16>,
    //pub(super) mip_slice: Option<u8>,
    pub(super) format: RafxFormat,
    pub(super) samples: RafxSampleCount,
    pub(super) initial_state: RafxResourceState,
    pub(super) final_state: RafxResourceState,
}

#[derive(Debug)]
pub struct PrepassBarrier {
    pub image_barriers: Vec<PrepassImageBarrier>,
    pub buffer_barriers: Vec<PrepassBufferBarrier>,
}

#[derive(Debug)]
pub struct PostpassBarrier {
    // layout transition
    pub image_barriers: Vec<PrepassImageBarrier>,
    pub buffer_barriers: Vec<PrepassBufferBarrier>,
    // resolve? probably do that in rafx api level
}

#[derive(Debug)]
pub struct PrepassImageBarrier {
    pub image: PhysicalImageId,
    pub old_state: RafxResourceState,
    pub new_state: RafxResourceState,
}

#[derive(Debug)]
pub struct PrepassBufferBarrier {
    pub buffer: PhysicalBufferId,
    pub old_state: RafxResourceState,
    pub new_state: RafxResourceState,
}

/// Metadata required to create a renderpass
#[derive(Debug)]
pub struct RenderGraphRenderPass {
    pub(super) node_id: RenderGraphNodeId,
    pub(super) attachments: Vec<RenderGraphPassAttachment>,

    pub(super) color_attachments: [Option<usize>; MAX_COLOR_ATTACHMENTS], // could ref back to node
    pub(super) resolve_attachments: [Option<usize>; MAX_RESOLVE_ATTACHMENTS],
    pub(super) depth_attachment: Option<usize>,

    // For when we want to do layout transitions on non-attachments
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
}

#[derive(Debug)]
pub struct RenderGraphCallbackPass {
    pub(super) node: RenderGraphNodeId,
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
}

#[derive(Debug)]
pub enum RenderGraphPass {
    Render(RenderGraphRenderPass),
    Callback(RenderGraphCallbackPass),
}

impl RenderGraphPass {
    pub fn node(&self) -> RenderGraphNodeId {
        match self {
            RenderGraphPass::Render(renderpass) => renderpass.node_id,
            RenderGraphPass::Callback(compute_pass) => compute_pass.node,
        }
    }

    pub fn set_pre_pass_barrier(
        &mut self,
        barrier: PrepassBarrier,
    ) {
        match self {
            RenderGraphPass::Render(renderpass) => renderpass.pre_pass_barrier = Some(barrier),
            RenderGraphPass::Callback(compute_pass) => {
                compute_pass.pre_pass_barrier = Some(barrier);
            }
        }
    }
}

pub struct RenderGraphColorRenderTarget {
    pub image: PhysicalImageId,
    pub clear_value: RafxColorClearValue,
    pub load_op: RafxLoadOp,
    pub store_op: RafxStoreOp,
    pub array_slice: Option<u16>,
    pub mip_slice: Option<u8>,
    pub resolve_image: Option<PhysicalImageId>,
    pub resolve_store_op: RafxStoreOp,
    pub resolve_array_slice: Option<u16>,
    pub resolve_mip_slice: Option<u8>,
}

pub struct RenderGraphDepthStencilRenderTarget {
    pub image: PhysicalImageId,
    pub clear_value: RafxDepthStencilClearValue,
    pub depth_load_op: RafxLoadOp,
    pub stencil_load_op: RafxLoadOp,
    pub depth_store_op: RafxStoreOp,
    pub stencil_store_op: RafxStoreOp,
    pub array_slice: Option<u16>,
    pub mip_slice: Option<u8>,
}

pub struct RenderGraphOutputRenderPass {
    pub(super) node_id: RenderGraphNodeId,
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
    pub(super) debug_name: Option<RenderGraphNodeName>,
    pub(super) attachment_images: Vec<PhysicalImageViewId>,
    pub(super) color_render_targets: Vec<RenderGraphColorRenderTarget>,
    pub(super) depth_stencil_render_target: Option<RenderGraphDepthStencilRenderTarget>,
    pub(super) render_target_meta: GraphicsPipelineRenderTargetMeta,
}

impl std::fmt::Debug for RenderGraphOutputRenderPass {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RenderGraphOutputRenderPass")
            //.field("description", &self.description)
            .field("attachment_images", &self.attachment_images)
            //.field("extents", &self.extents)
            .finish()
    }
}

#[derive(Debug)]
pub struct RenderGraphOutputCallbackPass {
    pub(super) node: RenderGraphNodeId,
    pub(super) pre_pass_barrier: Option<PrepassBarrier>,
    pub(super) debug_name: Option<RenderGraphNodeName>,
}

#[derive(Debug)]
pub enum RenderGraphOutputPass {
    Render(RenderGraphOutputRenderPass),
    Callback(RenderGraphOutputCallbackPass),
}

impl RenderGraphOutputPass {
    pub fn node(&self) -> RenderGraphNodeId {
        match self {
            RenderGraphOutputPass::Render(pass) => pass.node_id,
            RenderGraphOutputPass::Callback(pass) => pass.node,
        }
    }

    pub fn pre_pass_barrier(&self) -> Option<&PrepassBarrier> {
        match self {
            RenderGraphOutputPass::Render(pass) => pass.pre_pass_barrier.as_ref(),
            RenderGraphOutputPass::Callback(pass) => pass.pre_pass_barrier.as_ref(),
        }
    }

    pub fn debug_name(&self) -> Option<RenderGraphNodeName> {
        match self {
            RenderGraphOutputPass::Render(pass) => pass.debug_name,
            RenderGraphOutputPass::Callback(pass) => pass.debug_name,
        }
    }
}
