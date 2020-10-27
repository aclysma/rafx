use ash::vk;
use super::*;
use crate::vk_description as dsc;

#[derive(Debug, Copy, Clone)]
pub struct RenderGraphOutputImageId(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphImageVersionId {
    pub(super) index: usize,
    pub(super) version: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphImageUsageId(pub(super) usize);

#[derive(Debug)]
pub enum RenderGraphImageUser {
    Node(RenderGraphNodeId),
    Output(RenderGraphOutputImageId),
}

#[derive(Debug)]
pub struct RenderGraphImageUsage {
    pub(super) user: RenderGraphImageUser,
    pub(super) usage_type: RenderGraphImageUsageType,
    pub(super) version: RenderGraphImageVersionId,

    pub(super) preferred_layout: dsc::ImageLayout,
    //pub(super) access_flags: vk::AccessFlags,
    //pub(super) stage_flags: vk::PipelineStageFlags,
    //pub(super) image_aspect_flags: vk::ImageAspectFlags,
}

pub type RenderGraphResourceName = &'static str;

//
// The immutable aspects of an image
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphImageSpecification {
    pub samples: vk::SampleCountFlags,
    pub format: vk::Format,
    pub aspect_flags: vk::ImageAspectFlags,
    pub usage_flags: vk::ImageUsageFlags,
}

//
// Constraints on an image. Constraints are set per-field and start out None (i.e. unconstrained)
//
#[derive(Default, Clone, Debug)]
pub struct RenderGraphImageConstraint {
    pub samples: Option<vk::SampleCountFlags>,
    pub format: Option<vk::Format>,
    pub aspect_flags: vk::ImageAspectFlags,
    pub usage_flags: vk::ImageUsageFlags,
}

impl From<RenderGraphImageSpecification> for RenderGraphImageConstraint {
    fn from(specification: RenderGraphImageSpecification) -> Self {
        RenderGraphImageConstraint {
            samples: Some(specification.samples),
            format: Some(specification.format),
            aspect_flags: specification.aspect_flags,
            usage_flags: specification.usage_flags,
        }
    }
}

impl RenderGraphImageConstraint {
    pub fn try_convert_to_specification(self) -> Option<RenderGraphImageSpecification> {
        if self.samples.is_none() || self.format.is_none() {
            None
        } else {
            Some(RenderGraphImageSpecification {
                samples: self.samples.unwrap(),
                format: self.format.unwrap(),
                aspect_flags: self.aspect_flags,
                usage_flags: self.usage_flags,
            })
        }
    }
}

impl RenderGraphImageConstraint {
    pub fn can_merge(
        &self,
        other: &RenderGraphImageConstraint,
    ) -> bool {
        if self.samples.is_some() && other.samples.is_some() && self.samples != other.samples {
            return false;
        }
        if self.format.is_some() && other.format.is_some() && self.format != other.format {
            return false;
        }

        true
    }

    pub fn try_merge(
        &mut self,
        other: &RenderGraphImageConstraint,
    ) -> bool {
        if !self.can_merge(other) {
            return false;
        }

        if self.samples.is_none() && other.samples.is_some() {
            self.samples = other.samples;
        }
        if self.format.is_none() && other.format.is_some() {
            self.format = other.format;
        }

        self.aspect_flags |= other.aspect_flags;
        self.usage_flags |= other.usage_flags;

        true
    }

    pub fn partial_merge(
        &mut self,
        other: &RenderGraphImageConstraint,
    ) -> bool {
        let mut complete_merge = true;

        if self.samples.is_some() && other.samples.is_some() && self.samples != other.samples {
            complete_merge = false;
        } else if other.samples.is_some() {
            self.samples = other.samples;
        }

        if self.format.is_some() && other.format.is_some() && self.format != other.format {
            complete_merge = false;
        } else if other.format.is_some() {
            self.format = other.format;
        }

        self.aspect_flags |= other.aspect_flags;
        self.usage_flags |= other.usage_flags;

        complete_merge
    }

    pub fn set(
        &mut self,
        other: &RenderGraphImageSpecification,
    ) {
        *self = other.clone().into();
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RenderGraphImageUsageType {
    Create,
    //Input,
    Read,
    ModifyRead,
    ModifyWrite,
    Output,
}

impl RenderGraphImageUsageType {
    pub fn is_read_only(&self) -> bool {
        match self {
            RenderGraphImageUsageType::Read => true,
            RenderGraphImageUsageType::Output => true,
            RenderGraphImageUsageType::ModifyRead => false,
            RenderGraphImageUsageType::Create => false,
            //RenderGraphImageUsageType::Input => false,
            RenderGraphImageUsageType::ModifyWrite => false,
        }
    }
}

#[derive(Debug)]
pub struct RenderGraphImageResourceVersionInfo {
    pub(super) creator_node: RenderGraphNodeId,

    pub(super) create_usage: RenderGraphImageUsageId,
    pub(super) read_usages: Vec<RenderGraphImageUsageId>,
}

impl RenderGraphImageResourceVersionInfo {
    pub(super) fn new(
        creator: RenderGraphNodeId,
        create_usage: RenderGraphImageUsageId,
    ) -> Self {
        RenderGraphImageResourceVersionInfo {
            creator_node: creator,
            create_usage,
            read_usages: Default::default(),
        }
    }

    pub(super) fn remove_read_usage(
        &mut self,
        usage: RenderGraphImageUsageId,
    ) {
        if let Some(position) = self.read_usages.iter().position(|x| *x == usage) {
            self.read_usages.swap_remove(position);
        }
    }

    pub(super) fn add_read_usage(
        &mut self,
        usage: RenderGraphImageUsageId,
    ) {
        self.read_usages.push(usage);
    }
}

//
// A "virtual" image that the render graph knows about. The render graph will allocate images as
// needed, but can reuse the same image for multiple resources if the lifetimes of those images
// don't overlap
//
#[derive(Debug)]
pub struct RenderGraphImageResource {
    pub(super) name: Option<RenderGraphResourceName>,

    pub(super) versions: Vec<RenderGraphImageResourceVersionInfo>,
}

impl RenderGraphImageResource {
    pub(super) fn new() -> Self {
        RenderGraphImageResource {
            name: None,
            versions: Default::default(),
        }
    }
}