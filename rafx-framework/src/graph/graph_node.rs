use super::*;
use crate::graph::graph_builder::RenderGraphQueue;
use rafx_api::{RafxColorClearValue, RafxDepthStencilClearValue};
use std::fmt::Formatter;

//
// Nodes
//
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphNodeId(pub(super) usize);

pub type RenderGraphNodeName = &'static str;

#[derive(Debug, Clone)]
pub struct RenderGraphImageCreate {
    pub image: RenderGraphImageUsageId,
    pub constraint: RenderGraphImageConstraint,
}

#[derive(Debug, Clone)]
pub struct RenderGraphImageRead {
    pub image: RenderGraphImageUsageId,
    pub constraint: RenderGraphImageConstraint,
}

#[derive(Debug, Clone)]
pub struct RenderGraphImageModify {
    pub input: RenderGraphImageUsageId,
    pub output: RenderGraphImageUsageId,
    pub constraint: RenderGraphImageConstraint,
}

#[derive(Debug, Clone)]
pub struct RenderGraphImageCopy {
    pub input: RenderGraphImageUsageId,
    pub output: RenderGraphImageUsageId,
    pub constraint: RenderGraphImageConstraint,
}

#[derive(Debug, Clone)]
pub struct RenderGraphBufferCreate {
    pub buffer: RenderGraphBufferUsageId,
    pub constraint: RenderGraphBufferConstraint,
}

#[derive(Debug, Clone)]
pub struct RenderGraphBufferRead {
    pub buffer: RenderGraphBufferUsageId,
    pub constraint: RenderGraphBufferConstraint,
}

#[derive(Debug, Clone)]
pub struct RenderGraphBufferModify {
    pub input: RenderGraphBufferUsageId,
    pub output: RenderGraphBufferUsageId,
    pub constraint: RenderGraphBufferConstraint,
}

#[derive(Debug, Clone)]
pub struct RenderGraphBufferCopy {
    pub input: RenderGraphBufferUsageId,
    pub output: RenderGraphBufferUsageId,
    pub constraint: RenderGraphBufferConstraint,
}

#[derive(Debug, Copy, Clone)]
pub enum RenderGraphPassAttachmentType {
    Create,
    Read,
    Modify,
}

pub struct RenderGraphPassColorAttachmentInfo {
    pub attachment_type: RenderGraphPassAttachmentType,
    pub clear_color_value: Option<RafxColorClearValue>,
    pub read_image: Option<RenderGraphImageUsageId>,
    pub write_image: Option<RenderGraphImageUsageId>,
}

impl std::fmt::Debug for RenderGraphPassColorAttachmentInfo {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ColorAttachmentInfo")
            .field("attachment_type", &self.attachment_type)
            .field("read_image", &self.read_image)
            .field("write_image", &self.write_image)
            .finish()
    }
}

pub struct RenderGraphPassDepthAttachmentInfo {
    pub attachment_type: RenderGraphPassAttachmentType,
    pub clear_depth_stencil_value: Option<RafxDepthStencilClearValue>,
    pub read_image: Option<RenderGraphImageUsageId>,
    pub write_image: Option<RenderGraphImageUsageId>,
    pub has_depth: bool,
    pub has_stencil: bool,
}

impl std::fmt::Debug for RenderGraphPassDepthAttachmentInfo {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DepthAttachmentInfo")
            .field("attachment_type", &self.attachment_type)
            .field("read_image", &self.read_image)
            .field("write_image", &self.write_image)
            .finish()
    }
}

pub struct RenderGraphPassResolveAttachmentInfo {
    pub attachment_type: RenderGraphPassAttachmentType,
    pub write_image: RenderGraphImageUsageId,
}

impl std::fmt::Debug for RenderGraphPassResolveAttachmentInfo {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DepthAttachmentInfo")
            .field("attachment_type", &self.attachment_type)
            .field("write_image", &self.write_image)
            .finish()
    }
}

//
// Graph nodes represent a "schedulable" event, generally a renderpass. It reads/writes resources.
//
pub struct RenderGraphNode {
    id: RenderGraphNodeId,
    pub(super) name: Option<RenderGraphNodeName>,
    #[allow(dead_code)]
    pub(super) queue: RenderGraphQueue,
    pub(super) can_be_culled: bool,

    // This stores creates/reads/modifies for all images/buffers.. more detailed information about
    // them may be included in other lists (like color_attachments). This is mainly used to
    // determine image/buffer reuse compatibility across nodes. NOT for barriers/resource state
    // managements
    pub(super) image_creates: Vec<RenderGraphImageCreate>,
    pub(super) image_reads: Vec<RenderGraphImageRead>,
    pub(super) image_modifies: Vec<RenderGraphImageModify>,
    pub(super) image_copies: Vec<RenderGraphImageCopy>,

    pub(super) buffer_creates: Vec<RenderGraphBufferCreate>,
    pub(super) buffer_reads: Vec<RenderGraphBufferRead>,
    pub(super) buffer_modifies: Vec<RenderGraphBufferModify>,
    pub(super) buffer_copies: Vec<RenderGraphBufferCopy>,

    // Used when we want to force one node to execute before this one
    pub(super) explicit_dependencies: Vec<RenderGraphNodeId>,

    // Attachments are indexed by attachment index
    pub(super) color_attachments: Vec<Option<RenderGraphPassColorAttachmentInfo>>,
    pub(super) depth_attachment: Option<RenderGraphPassDepthAttachmentInfo>,
    pub(super) resolve_attachments: Vec<Option<RenderGraphPassResolveAttachmentInfo>>,
    pub(super) sampled_images: Vec<RenderGraphImageUsageId>,
    pub(super) storage_image_creates: Vec<RenderGraphImageUsageId>,
    pub(super) storage_image_reads: Vec<RenderGraphImageUsageId>,
    pub(super) storage_image_modifies: Vec<RenderGraphImageUsageId>,
    pub(super) copy_src_image_reads: Vec<RenderGraphImageUsageId>,
    pub(super) copy_dst_image_writes: Vec<RenderGraphImageUsageId>,

    pub(super) vertex_buffer_reads: Vec<RenderGraphBufferUsageId>,
    pub(super) index_buffer_reads: Vec<RenderGraphBufferUsageId>,
    pub(super) indirect_buffer_reads: Vec<RenderGraphBufferUsageId>,
    pub(super) uniform_buffer_reads: Vec<RenderGraphBufferUsageId>,
    pub(super) storage_buffer_creates: Vec<RenderGraphBufferUsageId>,
    pub(super) storage_buffer_reads: Vec<RenderGraphBufferUsageId>,
    pub(super) storage_buffer_modifies: Vec<RenderGraphBufferUsageId>,
    pub(super) copy_src_buffer_reads: Vec<RenderGraphBufferUsageId>,
    pub(super) copy_dst_buffer_writes: Vec<RenderGraphBufferUsageId>,
}

impl std::fmt::Debug for RenderGraphNode {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RenderGraphNode")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("image_creates", &self.image_creates)
            .field("image_reads", &self.image_reads)
            .field("image_modifies", &self.image_modifies)
            .field("image_copies", &self.image_copies)
            .field("buffer_creates", &self.buffer_creates)
            .field("buffer_reads", &self.buffer_reads)
            .field("buffer_modifies", &self.buffer_modifies)
            .field("buffer_copies", &self.buffer_copies)
            .field("explicit_dependencies", &self.explicit_dependencies)
            .field("color_attachments", &self.color_attachments)
            .field("depth_attachment", &self.depth_attachment)
            .field("resolve_attachments", &self.resolve_attachments)
            .field("sampled_images", &self.sampled_images)
            .field("storage_image_create", &self.storage_image_creates)
            .field("storage_image_read", &self.storage_image_reads)
            .field("storage_image_modify", &self.storage_image_modifies)
            .field("copy_src_image_reads", &self.copy_src_image_reads)
            .field("copy_dst_image_writes", &self.copy_dst_image_writes)
            .field("vertex_buffer_reads", &self.vertex_buffer_reads)
            .field("index_buffer_reads", &self.index_buffer_reads)
            .field("indirect_buffer_reads", &self.indirect_buffer_reads)
            .field("uniform_buffer_reads", &self.uniform_buffer_reads)
            .field("storage_buffer_creates", &self.storage_buffer_creates)
            .field("storage_buffer_reads", &self.storage_buffer_reads)
            .field("storage_buffer_modifies", &self.storage_buffer_modifies)
            .field("copy_src_buffer_reads", &self.copy_src_buffer_reads)
            .field("copy_dst_buffer_writes", &self.copy_dst_buffer_writes)
            .finish()
    }
}

impl RenderGraphNode {
    // Create a render node with the given ID.
    pub(super) fn new(
        id: RenderGraphNodeId,
        name: Option<RenderGraphNodeName>,
        queue: RenderGraphQueue,
    ) -> Self {
        RenderGraphNode {
            id,
            name,
            queue,
            can_be_culled: true,
            image_creates: Default::default(),
            image_reads: Default::default(),
            image_modifies: Default::default(),
            image_copies: Default::default(),
            buffer_creates: Default::default(),
            buffer_reads: Default::default(),
            buffer_modifies: Default::default(),
            buffer_copies: Default::default(),
            explicit_dependencies: Default::default(),
            color_attachments: Default::default(),
            depth_attachment: Default::default(),
            resolve_attachments: Default::default(),
            sampled_images: Default::default(),
            storage_image_creates: Default::default(),
            storage_image_reads: Default::default(),
            storage_image_modifies: Default::default(),
            copy_src_image_reads: Default::default(),
            copy_dst_image_writes: Default::default(),
            vertex_buffer_reads: Default::default(),
            index_buffer_reads: Default::default(),
            indirect_buffer_reads: Default::default(),
            uniform_buffer_reads: Default::default(),
            storage_buffer_creates: Default::default(),
            storage_buffer_reads: Default::default(),
            storage_buffer_modifies: Default::default(),
            copy_src_buffer_reads: Default::default(),
            copy_dst_buffer_writes: Default::default(),
        }
    }

    pub fn id(&self) -> RenderGraphNodeId {
        self.id
    }

    pub fn name(&self) -> Option<RenderGraphNodeName> {
        self.name
    }
}
