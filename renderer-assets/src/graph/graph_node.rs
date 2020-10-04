use super::*;
use std::fmt::Formatter;

//
// Nodes
//
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphNodeId(pub(super) usize);

pub type RenderGraphNodeName = &'static str;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RenderGraphAttachmentType {
    Color(usize),
    Depth,
    Resolve(usize),
}

#[derive(Debug, Clone)]
pub struct RenderGraphImageCreate {
    pub image: RenderGraphImageUsageId,
    pub constraint: RenderGraphImageConstraint,
    pub attachment_type: RenderGraphAttachmentType,
}

#[derive(Debug, Clone)]
pub struct RenderGraphImageRead {
    pub image: RenderGraphImageUsageId,
    pub constraint: RenderGraphImageConstraint,
    pub attachment_type: RenderGraphAttachmentType,
}

#[derive(Debug, Clone)]
pub struct RenderGraphImageModify {
    pub input: RenderGraphImageUsageId,
    pub output: RenderGraphImageUsageId,
    pub constraint: RenderGraphImageConstraint,
    pub attachment_type: RenderGraphAttachmentType,
}

#[derive(Debug, Copy, Clone)]
pub enum RenderGraphPassAttachmentType {
    Create,
    Read,
    Modify,
}

impl RenderGraphPassAttachmentType {
    pub fn is_write(&self) -> bool {
        match self {
            RenderGraphPassAttachmentType::Create => true,
            RenderGraphPassAttachmentType::Modify => true,
            RenderGraphPassAttachmentType::Read => false,
        }
    }
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
    pub write_image: Option<RenderGraphImageUsageId>,
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
#[derive(Debug)]
pub struct RenderGraphNode {
    id: RenderGraphNodeId,
    name: Option<RenderGraphNodeName>,
    //pub(super) action: Option<Box<dyn RenderGraphNodeAction>>,

    // This stores creates/reads/modifies for all images.. more detailed information about them
    // may be included in other lists (like color_attachments)
    pub(super) image_creates: Vec<RenderGraphImageCreate>,
    pub(super) image_reads: Vec<RenderGraphImageRead>,
    pub(super) image_modifies: Vec<RenderGraphImageModify>,

    // Indexed by attachment index
    pub(super) color_attachments: Vec<Option<RenderGraphPassColorAttachmentInfo>>,
    pub(super) depth_attachment: Option<RenderGraphPassDepthAttachmentInfo>,
    pub(super) resolve_attachments: Vec<Option<RenderGraphPassResolveAttachmentInfo>>,
}

impl RenderGraphNode {
    // Create a render node with the given ID.
    pub(super) fn new(id: RenderGraphNodeId) -> Self {
        RenderGraphNode {
            id,
            name: None,
            image_creates: Default::default(),
            image_reads: Default::default(),
            image_modifies: Default::default(),
            color_attachments: Default::default(),
            depth_attachment: Default::default(),
            resolve_attachments: Default::default(),
        }
    }

    pub fn id(&self) -> RenderGraphNodeId {
        self.id
    }

    pub fn name(&self) -> Option<RenderGraphNodeName> {
        self.name
    }
}

//
// A helper for configuring a node. This helper allows us to have a borrow against the rest of
// the graph data, allowing us to write data into resources as well as nodes
//
pub struct RenderGraphNodeConfigureContext<'a> {
    pub(super) graph: &'a mut RenderGraph,
    pub(super) node_id: RenderGraphNodeId,
}

impl<'a> RenderGraphNodeConfigureContext<'a> {
    pub fn id(&self) -> RenderGraphNodeId {
        self.node_id
    }

    pub fn set_name(
        &mut self,
        name: RenderGraphNodeName,
    ) {
        self.graph.node_mut(self.node_id).name = Some(name);
    }

    pub fn create_color_attachment(
        &mut self,
        color_attachment_index: usize,
        clear_color_value: Option<vk::ClearColorValue>,
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        self.graph.create_color_attachment(
            self.node_id,
            color_attachment_index,
            clear_color_value,
            constraint,
        )
    }

    pub fn create_depth_attachment(
        &mut self,
        clear_depth_stencil_value: Option<vk::ClearDepthStencilValue>,
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        self.graph
            .create_depth_attachment(self.node_id, clear_depth_stencil_value, constraint)
    }

    pub fn read_color_attachment(
        &mut self,
        image: RenderGraphImageUsageId,
        color_attachment_index: usize,
        constraint: RenderGraphImageConstraint,
    ) {
        self.graph
            .read_color_attachment(self.node_id, image, color_attachment_index, constraint)
    }

    pub fn read_depth_attachment(
        &mut self,
        image: RenderGraphImageUsageId,
        constraint: RenderGraphImageConstraint,
    ) {
        self.graph
            .read_depth_attachment(self.node_id, image, constraint)
    }

    pub fn modify_color_attachment(
        &mut self,
        image: RenderGraphImageUsageId,
        color_attachment_index: usize,
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        self.graph
            .modify_color_attachment(self.node_id, image, color_attachment_index, constraint)
    }

    pub fn modify_depth_attachment(
        &mut self,
        image: RenderGraphImageUsageId,
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        self.graph
            .modify_depth_attachment(self.node_id, image, constraint)
    }

    // sample?
    // force_no_cull?
}
