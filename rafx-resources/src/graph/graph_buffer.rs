use crate::graph::{RenderGraphNodeId, RenderGraphResourceName};
use ash::vk;

/// Unique ID for a particular usage (read or write) of a specific buffer
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphBufferUsageId(pub(super) usize);

/// An ID for a buffer used within the graph between passes
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VirtualBufferId(pub(super) usize);

/// An ID for a buffer allocation (possibly reused)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalBufferId(pub(super) usize);

/// Unique ID provided for any buffer registered as an output buffer
#[derive(Debug, Copy, Clone)]
pub struct RenderGraphOutputBufferId(pub(super) usize);

/// Unique ID for a particular version of a buffer. Any time a buffer is modified, a new version is
/// produced
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphBufferVersionId {
    pub(super) index: usize,
    pub(super) version: usize,
}

/// A "virtual" buffer that the render graph knows about. The render graph will allocate buffers as
/// needed, but can reuse the same buffer for multiple resources if the lifetimes of those buffers
/// don't overlap
#[derive(Debug)]
pub struct RenderGraphBufferResource {
    pub(super) name: Option<RenderGraphResourceName>,

    pub(super) versions: Vec<RenderGraphBufferResourceVersionInfo>,
}

impl RenderGraphBufferResource {
    pub(super) fn new() -> Self {
        RenderGraphBufferResource {
            name: None,
            versions: Default::default(),
        }
    }
}

/// Defines what created a RenderGraphBufferUsage
#[derive(Debug)]
pub enum RenderGraphBufferUser {
    Node(RenderGraphNodeId),
    Output(RenderGraphOutputBufferId),
}

/// A usage of a particular buffer
#[derive(Debug)]
pub struct RenderGraphBufferUsage {
    pub(super) user: RenderGraphBufferUser,
    pub(super) usage_type: RenderGraphBufferUsageType,
    pub(super) version: RenderGraphBufferVersionId,
    pub(super) access_flags: vk::AccessFlags,
    pub(super) stage_flags: vk::PipelineStageFlags,
}

/// Immutable, fully-specified attributes of a buffer. A *constraint* is partially specified and
/// the graph will use constraints to solve for the specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphBufferSpecification {
    pub size: u64,
    pub usage_flags: vk::BufferUsageFlags,
    // sharing mode - always exclusive
}

impl RenderGraphBufferSpecification {
    /// Returns true if no fields in the two constraints are conflicting
    pub fn can_merge(
        &self,
        other: &RenderGraphBufferSpecification,
    ) -> bool {
        if self.size != other.size {
            return false;
        }

        true
    }

    /// Merge other's constraints into self, but only if there are no conflicts. No modification
    /// occurs if any conflict exists
    pub fn try_merge(
        &mut self,
        other: &RenderGraphBufferSpecification,
    ) -> bool {
        if !self.can_merge(other) {
            return false;
        }

        self.usage_flags |= other.usage_flags;

        true
    }
}

/// Constraints on a buffer. Constraints are set per-field and start out None (i.e. unconstrained)
/// The rendergraph will derive specifications from the constraints
#[derive(Default, Clone, Debug)]
pub struct RenderGraphBufferConstraint {
    // Rename to RenderGraphBufferUsageConstraint?
    pub size: Option<u64>,
    pub usage_flags: vk::BufferUsageFlags,
}

impl From<RenderGraphBufferSpecification> for RenderGraphBufferConstraint {
    fn from(specification: RenderGraphBufferSpecification) -> Self {
        RenderGraphBufferConstraint {
            size: Some(specification.size),
            usage_flags: specification.usage_flags,
        }
    }
}

impl RenderGraphBufferConstraint {
    pub fn try_convert_to_specification(self) -> Option<RenderGraphBufferSpecification> {
        // Format is the only thing we can't default sensibly
        if self.size.is_none() {
            None
        } else {
            Some(RenderGraphBufferSpecification {
                size: self.size.unwrap(),
                usage_flags: self.usage_flags,
            })
        }
    }
}

impl RenderGraphBufferConstraint {
    /// Returns true if no fields in the two constraints are conflicting
    pub fn can_merge(
        &self,
        other: &RenderGraphBufferConstraint,
    ) -> bool {
        if self.size.is_some() && other.size.is_some() && self.size != other.size {
            return false;
        }

        true
    }

    /// Merge other's constraints into self, but only if there are no conflicts. No modification
    /// occurs if any conflict exists
    pub fn try_merge(
        &mut self,
        other: &RenderGraphBufferConstraint,
    ) -> bool {
        if !self.can_merge(other) {
            return false;
        }

        if self.size.is_none() && other.size.is_some() {
            self.size = other.size;
        }

        self.usage_flags |= other.usage_flags;

        true
    }

    /// Merge other's constraints into self. We will merge fields where we can and skip fields with
    /// conflicts
    pub fn partial_merge(
        &mut self,
        other: &RenderGraphBufferConstraint,
    ) -> bool {
        let mut complete_merge = true;

        if self.size.is_some() && other.size.is_some() && self.size != other.size {
            complete_merge = false;
        } else if other.size.is_some() {
            self.size = other.size;
        }

        self.usage_flags |= other.usage_flags;

        complete_merge
    }

    /// Sets the constraints based on the given specification
    pub fn set(
        &mut self,
        other: &RenderGraphBufferSpecification,
    ) {
        *self = other.clone().into();
    }
}

/// How a buffer is being used
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RenderGraphBufferUsageType {
    Create,
    //Input,
    Read,
    ModifyRead,
    ModifyWrite,
    Output,
}

impl RenderGraphBufferUsageType {
    //TODO: Add support to see if multiple writes actually overlap
    pub fn is_read_only(&self) -> bool {
        match self {
            RenderGraphBufferUsageType::Read => true,
            RenderGraphBufferUsageType::Output => true,
            RenderGraphBufferUsageType::ModifyRead => false,
            RenderGraphBufferUsageType::Create => false,
            // RenderGraphBufferUsageType::Input => false,
            RenderGraphBufferUsageType::ModifyWrite => false,
        }
    }
}

/// Information about a specific version of the buffer.
#[derive(Debug)]
pub struct RenderGraphBufferResourceVersionInfo {
    /// What node created the buffer (keep in mind these are virtual buffers, not buffers provided
    /// from outside the graph. So every buffer will have a creator node)
    pub(super) creator_node: RenderGraphNodeId,

    pub(super) create_usage: RenderGraphBufferUsageId,
    pub(super) read_usages: Vec<RenderGraphBufferUsageId>,
}

impl RenderGraphBufferResourceVersionInfo {
    pub(super) fn new(
        creator: RenderGraphNodeId,
        create_usage: RenderGraphBufferUsageId,
    ) -> Self {
        RenderGraphBufferResourceVersionInfo {
            creator_node: creator,
            create_usage,
            read_usages: Default::default(),
        }
    }

    pub(super) fn add_read_usage(
        &mut self,
        usage: RenderGraphBufferUsageId,
    ) {
        self.read_usages.push(usage);
    }
}
