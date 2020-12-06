use super::*;
use crate::graph::graph_builder::RenderGraphQueue;
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

#[derive(Debug, Copy, Clone)]
pub enum RenderGraphPassAttachmentType {
    Create,
    Read,
    Modify,
}

pub struct RenderGraphPassColorAttachmentInfo {
    pub attachment_type: RenderGraphPassAttachmentType,
    pub clear_color_value: Option<vk::ClearColorValue>,
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
    pub clear_depth_stencil_value: Option<vk::ClearDepthStencilValue>,
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

    // This stores creates/reads/modifies for all images.. more detailed information about them
    // may be included in other lists (like color_attachments)
    pub(super) image_creates: Vec<RenderGraphImageCreate>,
    pub(super) image_reads: Vec<RenderGraphImageRead>,
    pub(super) image_modifies: Vec<RenderGraphImageModify>,

    // Indexed by attachment index
    pub(super) color_attachments: Vec<Option<RenderGraphPassColorAttachmentInfo>>,
    pub(super) depth_attachment: Option<RenderGraphPassDepthAttachmentInfo>,
    pub(super) resolve_attachments: Vec<Option<RenderGraphPassResolveAttachmentInfo>>,

    pub(super) sampled_images: Vec<RenderGraphImageUsageId>,
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
            .field("color_attachments", &self.color_attachments)
            .field("depth_attachment", &self.depth_attachment)
            .field("resolve_attachments", &self.resolve_attachments)
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
            image_creates: Default::default(),
            image_reads: Default::default(),
            image_modifies: Default::default(),
            color_attachments: Default::default(),
            depth_attachment: Default::default(),
            resolve_attachments: Default::default(),
            sampled_images: Default::default(),
        }
    }

    pub fn id(&self) -> RenderGraphNodeId {
        self.id
    }

    pub fn name(&self) -> Option<RenderGraphNodeName> {
        self.name
    }
}
