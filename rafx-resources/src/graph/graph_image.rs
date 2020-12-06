use super::*;
use crate::vk_description as dsc;
use ash::vk;

/// Unique ID for a particular usage (read or write) of a specific image
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphImageUsageId(pub(super) usize);

/// An ID for an image used within the graph between passes
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VirtualImageId(pub(super) usize);

/// An ID for an image allocation (possibly reused)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalImageId(pub(super) usize);

/// An ID for an image view allocation (possibly reused)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalImageViewId(pub(super) usize);

/// Unique ID provided for any image registered as an output image
#[derive(Debug, Copy, Clone)]
pub struct RenderGraphOutputImageId(pub(super) usize);

/// Unique ID for a particular version of an image. Any time an image is modified, a new version is
/// produced
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphImageVersionId {
    pub(super) index: usize,
    pub(super) version: usize,
}

/// A "virtual" image that the render graph knows about. The render graph will allocate images as
/// needed, but can reuse the same image for multiple resources if the lifetimes of those images
/// don't overlap
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphImageView {
    pub(super) physical_image: PhysicalImageId,
    pub(super) subresource_range: dsc::ImageSubresourceRange,
    pub(super) view_type: dsc::ImageViewType,
}

/// Defines what created a RenderGraphImageUsage
#[derive(Debug)]
pub enum RenderGraphImageUser {
    Node(RenderGraphNodeId),
    Output(RenderGraphOutputImageId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderGraphImageExtents {
    MatchSurface,
    // (width, height, depth)
    Custom(u32, u32, u32),
}

impl RenderGraphImageExtents {
    pub fn into_vk_extent_3d(
        self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> vk::Extent3D {
        match self {
            RenderGraphImageExtents::MatchSurface => vk::Extent3D {
                width: swapchain_surface_info.extents.width,
                height: swapchain_surface_info.extents.height,
                depth: 1,
            },
            RenderGraphImageExtents::Custom(width, height, depth) => vk::Extent3D {
                width,
                height,
                depth,
            },
        }
    }

    pub fn into_vk_extent_2d(
        self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> vk::Extent2D {
        let extent_3d = self.into_vk_extent_3d(swapchain_surface_info);
        vk::Extent2D {
            width: extent_3d.width,
            height: extent_3d.height,
        }
    }
}

impl Default for RenderGraphImageExtents {
    fn default() -> Self {
        RenderGraphImageExtents::MatchSurface
    }
}

#[derive(Clone, Debug)]
pub enum RenderGraphImageSubresourceRange {
    // Use the entire image
    AllMipsAllLayers,
    // Mip 0 with given layer
    NoMipsSingleLayer(u32),
    // Mip 0 layer 0
    NoMipsNoLayers,
    Custom(dsc::ImageSubresourceRange),
}

impl RenderGraphImageSubresourceRange {
    pub fn into_subresource_range(
        &self,
        specification: &RenderGraphImageSpecification,
    ) -> dsc::ImageSubresourceRange {
        match self {
            RenderGraphImageSubresourceRange::AllMipsAllLayers => {
                dsc::ImageSubresourceRange::default_all_mips_all_layers(
                    dsc::ImageAspectFlag::from_vk_image_aspect_flags(specification.aspect_flags),
                    specification.mip_count,
                    specification.layer_count,
                )
            }
            RenderGraphImageSubresourceRange::NoMipsSingleLayer(layer) => {
                dsc::ImageSubresourceRange::default_no_mips_single_layer(
                    dsc::ImageAspectFlag::from_vk_image_aspect_flags(specification.aspect_flags),
                    *layer,
                )
            }
            RenderGraphImageSubresourceRange::NoMipsNoLayers => {
                dsc::ImageSubresourceRange::default_no_mips_no_layers(
                    dsc::ImageAspectFlag::from_vk_image_aspect_flags(specification.aspect_flags),
                )
            }
            RenderGraphImageSubresourceRange::Custom(custom) => custom.clone(),
        }
    }
}

impl Default for RenderGraphImageSubresourceRange {
    fn default() -> Self {
        RenderGraphImageSubresourceRange::AllMipsAllLayers
    }
}

/// A usage of a particular image
#[derive(Debug)]
pub struct RenderGraphImageUsage {
    pub(super) user: RenderGraphImageUser,
    pub(super) usage_type: RenderGraphImageUsageType,
    pub(super) version: RenderGraphImageVersionId,

    pub(super) preferred_layout: dsc::ImageLayout,
    pub(super) subresource_range: RenderGraphImageSubresourceRange,
    pub(super) view_type: dsc::ImageViewType,
    //pub(super) access_flags: vk::AccessFlags,
    //pub(super) stage_flags: vk::PipelineStageFlags,
    //pub(super) image_aspect_flags: vk::ImageAspectFlags,
}

pub type RenderGraphResourceName = &'static str;

/// Immutable, fully-specified attributes of an image. A *constraint* is partially specified and
/// the graph will use constraints to solve for the specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphImageSpecification {
    // Rename to RenderGraphImageUsageSpecification?
    pub samples: vk::SampleCountFlags,
    pub format: vk::Format,
    pub aspect_flags: vk::ImageAspectFlags,
    pub usage_flags: vk::ImageUsageFlags,
    pub create_flags: vk::ImageCreateFlags,
    pub extents: RenderGraphImageExtents,
    pub layer_count: u32,
    pub mip_count: u32,
    // image type - always 2D
    // extents - always matches the render surface
    // tiling - always optimal
    // layout - controlled by graph
    // sharing mode - always exclusive
}

impl RenderGraphImageSpecification {
    /// Returns true if no fields in the two constraints are conflicting
    pub fn can_merge(
        &self,
        other: &RenderGraphImageSpecification,
    ) -> bool {
        if self.samples != other.samples {
            return false;
        }
        if self.format != other.format {
            return false;
        }
        if self.mip_count != other.mip_count {
            return false;
        }
        if self.layer_count != other.layer_count {
            return false;
        }
        if self.extents != other.extents {
            return false;
        }

        true
    }

    /// Merge other's constraints into self, but only if there are no conflicts. No modification
    /// occurs if any conflict exists
    pub fn try_merge(
        &mut self,
        other: &RenderGraphImageSpecification,
    ) -> bool {
        if !self.can_merge(other) {
            return false;
        }

        self.aspect_flags |= other.aspect_flags;
        self.usage_flags |= other.usage_flags;

        true
    }
}

/// Constraints on an image. Constraints are set per-field and start out None (i.e. unconstrained)
/// The rendergraph will derive specifications from the constraints
#[derive(Default, Clone, Debug)]
pub struct RenderGraphImageConstraint {
    // Rename to RenderGraphImageUsageConstraint?
    pub samples: Option<vk::SampleCountFlags>,
    pub format: Option<vk::Format>,
    pub aspect_flags: vk::ImageAspectFlags,
    pub usage_flags: vk::ImageUsageFlags,
    pub create_flags: vk::ImageCreateFlags,
    pub extents: Option<RenderGraphImageExtents>,
    //pub dimensions: vk::ImageSubresource
    pub layer_count: Option<u32>,
    pub mip_count: Option<u32>,
}

impl From<RenderGraphImageSpecification> for RenderGraphImageConstraint {
    fn from(specification: RenderGraphImageSpecification) -> Self {
        RenderGraphImageConstraint {
            samples: Some(specification.samples),
            format: Some(specification.format),
            layer_count: Some(specification.layer_count),
            mip_count: Some(specification.mip_count),
            extents: Some(specification.extents),
            aspect_flags: specification.aspect_flags,
            usage_flags: specification.usage_flags,
            create_flags: specification.create_flags,
        }
    }
}

impl RenderGraphImageConstraint {
    pub fn try_convert_to_specification(self) -> Option<RenderGraphImageSpecification> {
        // Format is the only thing we can't default sensibly
        if self.format.is_none() {
            None
        } else {
            Some(RenderGraphImageSpecification {
                samples: self.samples.unwrap_or(vk::SampleCountFlags::TYPE_1),
                format: self.format.unwrap(),
                layer_count: self.layer_count.unwrap_or(1),
                mip_count: self.mip_count.unwrap_or(1),
                extents: self
                    .extents
                    .unwrap_or(RenderGraphImageExtents::MatchSurface),
                aspect_flags: self.aspect_flags,
                usage_flags: self.usage_flags,
                create_flags: self.create_flags,
            })
        }
    }
}

impl RenderGraphImageConstraint {
    /// Returns true if no fields in the two constraints are conflicting
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
        if self.layer_count.is_some()
            && other.layer_count.is_some()
            && self.layer_count != other.layer_count
        {
            return false;
        }
        if self.mip_count.is_some()
            && other.mip_count.is_some()
            && self.mip_count != other.mip_count
        {
            return false;
        }
        if self.extents.is_some() && other.extents.is_some() && self.extents != other.extents {
            return false;
        }

        true
    }

    /// Merge other's constraints into self, but only if there are no conflicts. No modification
    /// occurs if any conflict exists
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
        if self.layer_count.is_none() && other.layer_count.is_some() {
            self.layer_count = other.layer_count;
        }
        if self.mip_count.is_none() && other.mip_count.is_some() {
            self.mip_count = other.mip_count;
        }
        if self.extents.is_none() && other.extents.is_some() {
            self.extents = other.extents;
        }

        self.aspect_flags |= other.aspect_flags;
        self.usage_flags |= other.usage_flags;
        self.create_flags |= other.create_flags;

        true
    }

    /// Merge other's constraints into self. We will merge fields where we can and skip fields with
    /// conflicts
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

        if self.layer_count.is_some()
            && other.layer_count.is_some()
            && self.layer_count != other.layer_count
        {
            complete_merge = false;
        } else if other.layer_count.is_some() {
            self.layer_count = other.layer_count;
        }

        if self.mip_count.is_some()
            && other.mip_count.is_some()
            && self.mip_count != other.mip_count
        {
            complete_merge = false;
        } else if other.mip_count.is_some() {
            self.mip_count = other.mip_count;
        }

        if self.extents.is_some() && other.extents.is_some() && self.extents != other.extents {
            complete_merge = false;
        } else if other.extents.is_some() {
            self.extents = other.extents;
        }

        self.aspect_flags |= other.aspect_flags;
        self.usage_flags |= other.usage_flags;
        self.create_flags |= other.create_flags;

        complete_merge
    }

    /// Sets the constraints based on the given specification
    pub fn set(
        &mut self,
        other: &RenderGraphImageSpecification,
    ) {
        *self = other.clone().into();
    }
}

/// How an image is being used
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
    //TODO: Add support to see if multiple writes actually overlap
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

/// Information about a specific version of the image.
#[derive(Debug)]
pub struct RenderGraphImageResourceVersionInfo {
    /// What node created the image (keep in mind these are virtual images, not images provided
    /// from outside the graph. So every image will have a creator node)
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
