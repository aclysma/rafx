use super::*;
use fnv::{FnvHashSet, FnvHashMap};
use crate::vk_description as dsc;
use crate::vk_description::{AttachmentReference, AttachmentDescription, SwapchainSurfaceInfo};
use std::collections::HashMap;
use ash::vk::ClearValue;
use crate::resources::{ResourceArc, ImageViewResource};
use crate::graph::prepared_graph::RenderGraphPlanOutputImage;

/// The specification for the image by image usage
pub struct DetermineImageConstraintsResult {
    images: FnvHashMap<RenderGraphImageUsageId, RenderGraphImageSpecification>,
}

impl DetermineImageConstraintsResult {
    pub fn specification(
        &self,
        image: RenderGraphImageUsageId,
    ) -> &RenderGraphImageSpecification {
        self.images.get(&image).unwrap()
    }
}

#[derive(Debug)]
struct PhysicalImageInfo {
    usages: Vec<RenderGraphImageUsageId>,
    versions: Vec<RenderGraphImageVersionId>,
    specification: RenderGraphImageSpecification,
}

impl PhysicalImageInfo {
    fn new(specification: RenderGraphImageSpecification) -> Self {
        PhysicalImageInfo {
            usages: Default::default(),
            versions: Default::default(),
            specification,
        }
    }
}

/// Assignment of usages to actual images. This allows a single image to be passed through a
/// sequence of reads and writes
#[derive(Debug)]
pub struct AssignPhysicalImagesResult {
    map_image_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId>,
    // physical_image_usages: FnvHashMap<PhysicalImageId, Vec<RenderGraphImageUsageId>>,
    // physical_image_versions: FnvHashMap<PhysicalImageId, Vec<RenderGraphImageVersionId>>,
    physical_image_infos: FnvHashMap<PhysicalImageId, PhysicalImageInfo>,
}

/// An image that is being provided to the render graph that can be read from
#[derive(Debug)]
pub struct RenderGraphInputImage {
    pub usage: RenderGraphImageUsageId,
    pub specification: RenderGraphImageSpecification,
}

/// An image that is being provided to the render graph that can be written to
#[derive(Debug)]
pub struct RenderGraphOutputImage {
    pub output_image_id: RenderGraphOutputImageId,
    pub usage: RenderGraphImageUsageId,
    pub specification: RenderGraphImageSpecification,
    pub dst_image: ResourceArc<ImageViewResource>,

    pub(super) final_layout: dsc::ImageLayout,
    pub(super) final_access_flags: vk::AccessFlags,
    pub(super) final_stage_flags: vk::PipelineStageFlags,
}

/// Represents the invalidate or flush of a RenderGraphPassImageBarriers
#[derive(Debug)]
pub struct RenderGraphImageBarrier {
    access_flags: vk::AccessFlags,
    stage_flags: vk::PipelineStageFlags,
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
    invalidate: RenderGraphImageBarrier,
    flush: RenderGraphImageBarrier,
    layout: vk::ImageLayout,
}

impl RenderGraphPassImageBarriers {
    fn new(layout: vk::ImageLayout) -> Self {
        RenderGraphPassImageBarriers {
            flush: Default::default(),
            invalidate: Default::default(),
            layout,
        }
    }
}

/// All the barriers required for a single node (i.e. subpass). Nodes represent passes that may be
/// merged to be subpasses within a single pass.
#[derive(Debug)]
pub struct RenderGraphNodeImageBarriers {
    barriers: FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers>,
}

const MAX_COLOR_ATTACHMENTS: usize = 4;
const MAX_RESOLVE_ATTACHMENTS: usize = 4;

/// Metadata for a subpass
#[derive(Debug)]
pub struct RenderGraphSubpass {
    node: RenderGraphNodeId,

    color_attachments: [Option<usize>; MAX_COLOR_ATTACHMENTS], // could ref back to node
    resolve_attachments: [Option<usize>; MAX_RESOLVE_ATTACHMENTS],
    depth_attachment: Option<usize>,
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
            AttachmentClearValue::Color(value) => {
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
    fn into(self) -> ClearValue {
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
    image: PhysicalImageId,
    load_op: vk::AttachmentLoadOp,
    stencil_load_op: vk::AttachmentLoadOp,
    store_op: vk::AttachmentStoreOp,
    stencil_store_op: vk::AttachmentStoreOp,
    clear_color: Option<AttachmentClearValue>,
    format: vk::Format,
    samples: vk::SampleCountFlags,
    initial_layout: dsc::ImageLayout,
    final_layout: dsc::ImageLayout,
}

impl RenderGraphPassAttachment {
    fn into_pass_desc(&self) -> dsc::AttachmentDescription {
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

/// Metadata required to create a renderpass
#[derive(Debug)]
pub struct RenderGraphPass {
    attachments: Vec<RenderGraphPassAttachment>,
    subpasses: Vec<RenderGraphSubpass>,
    // clear colors?
}

/// An ID for an image (possibly aliased)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalImageId(usize);

#[derive(Default)]
struct PhysicalImageIdAllocator {
    next_id: usize,
}

impl PhysicalImageIdAllocator {
    fn allocate(&mut self) -> PhysicalImageId {
        let id = PhysicalImageId(self.next_id);
        self.next_id += 1;
        id
    }
}

pub struct RenderGraphOutputPass {
    pub(super) subpass_nodes: Vec<RenderGraphNodeId>,
    pub(super) description: dsc::RenderPass,
    pub(super) attachment_images: Vec<PhysicalImageId>,
    pub(super) clear_values: Vec<vk::ClearValue>,
    pub(super) extents: vk::Extent2D,
}

impl std::fmt::Debug for RenderGraphOutputPass {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RenderGraphOutputPass")
            .field("description", &self.description)
            .field("attachment_images", &self.attachment_images)
            .field("extents", &self.extents)
            .finish()
    }
}

/// A collection of nodes and resources. Nodes represent an event or process that will occur at
/// a certain time. (For now, they just represent subpasses that may be merged with each other.)
/// Resources represent images and buffers that may be read/written by nodes.
#[derive(Default, Debug)]
pub struct RenderGraph {
    /// Nodes that have been registered in the graph
    nodes: Vec<RenderGraphNode>,

    /// Image resources that have been registered in the graph. These resources are "virtual" until
    /// the graph is scheduled. In other words, we don't necessarily allocate an image for every
    /// resource as some resources can share the same image internally if their lifetime don't
    /// overlap. Additionally, a resource can be bound to input and output images. If this is the
    /// case, we will try to use those images rather than creating new ones.
    image_resources: Vec<RenderGraphImageResource>,

    /// All read/write accesses to images. Image writes create new "versions" of the image. So all
    /// image versions have one writer and 0 or more readers. This indirectly defines the order of
    /// execution for the graph.
    image_usages: Vec<RenderGraphImageUsage>,

    /// Images that are passed into the graph that can be read from
    pub(super) input_images: Vec<RenderGraphInputImage>,

    /// Images that are passed into the graph to be written to.
    pub(super) output_images: Vec<RenderGraphOutputImage>,
}

impl RenderGraph {
    pub(super) fn add_image_usage(
        &mut self,
        version: RenderGraphImageVersionId,
        usage_type: RenderGraphImageUsageType,
        preferred_layout: dsc::ImageLayout,
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
        image_aspect_flags: vk::ImageAspectFlags,
    ) -> RenderGraphImageUsageId {
        let usage_id = RenderGraphImageUsageId(self.image_usages.len());
        self.image_usages.push(RenderGraphImageUsage {
            usage_type,
            version,
            preferred_layout,
            //access_flags,
            //stage_flags,
            //image_aspect_flags
        });
        usage_id
    }

    // Add an image that can be used by nodes
    pub(super) fn add_image_create(
        &mut self,
        create_node: RenderGraphNodeId,
        attachment_type: RenderGraphAttachmentType,
        constraint: RenderGraphImageConstraint,
        preferred_layout: dsc::ImageLayout,
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
        image_aspect_flags: vk::ImageAspectFlags,
    ) -> RenderGraphImageUsageId {
        let version_id = RenderGraphImageVersionId {
            index: self.image_resources.len(),
            version: 0,
        };
        let usage_id = self.add_image_usage(
            version_id,
            RenderGraphImageUsageType::Create,
            preferred_layout,
            access_flags,
            stage_flags,
            image_aspect_flags,
        );

        let mut resource = RenderGraphImageResource::new();

        let mut version_info = RenderGraphImageResourceVersionInfo::new(create_node, usage_id);
        resource.versions.push(version_info);

        // Add it to the graph
        self.image_resources.push(resource);

        self.nodes[create_node.0]
            .image_creates
            .push(RenderGraphImageCreate {
                //image: image_id,
                image: usage_id,
                constraint,
                attachment_type,
            });

        usage_id
    }

    pub(super) fn add_image_read(
        &mut self,
        read_node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        attachment_type: RenderGraphAttachmentType,
        constraint: RenderGraphImageConstraint,
        preferred_layout: dsc::ImageLayout,
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
        image_aspect_flags: vk::ImageAspectFlags,
    ) -> RenderGraphImageUsageId {
        let version_id = self.image_usages[image.0].version;

        let usage_id = self.add_image_usage(
            version_id,
            RenderGraphImageUsageType::Read,
            preferred_layout,
            access_flags,
            stage_flags,
            image_aspect_flags,
        );

        self.image_resources[version_id.index].versions[version_id.version]
            .add_read_usage(usage_id);

        self.nodes[read_node.0]
            .image_reads
            .push(RenderGraphImageRead {
                image: usage_id,
                constraint,
                attachment_type,
            });

        usage_id
    }

    pub(super) fn add_image_modify(
        &mut self,
        modify_node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        attachment_type: RenderGraphAttachmentType,
        constraint: RenderGraphImageConstraint,
        preferred_layout: dsc::ImageLayout,
        read_access_flags: vk::AccessFlags,
        read_stage_flags: vk::PipelineStageFlags,
        read_image_aspect_flags: vk::ImageAspectFlags,
        write_access_flags: vk::AccessFlags,
        write_stage_flags: vk::PipelineStageFlags,
        write_image_aspect_flags: vk::ImageAspectFlags,
    ) -> (RenderGraphImageUsageId, RenderGraphImageUsageId) {
        let read_version_id = self.image_usages[image.0].version;

        let read_usage_id = self.add_image_usage(
            read_version_id,
            RenderGraphImageUsageType::ModifyRead,
            preferred_layout,
            read_access_flags,
            read_stage_flags,
            read_image_aspect_flags,
        );

        self.image_resources[read_version_id.index].versions[read_version_id.version]
            .add_read_usage(read_usage_id);

        // Create a new version and add it to the image
        let version = self.image_resources[read_version_id.index].versions.len();

        let write_version_id = RenderGraphImageVersionId {
            index: read_version_id.index,
            version,
        };
        let write_usage_id = self.add_image_usage(
            write_version_id,
            RenderGraphImageUsageType::ModifyWrite,
            preferred_layout,
            write_access_flags,
            write_stage_flags,
            write_image_aspect_flags,
        );

        let mut version_info =
            RenderGraphImageResourceVersionInfo::new(modify_node, write_usage_id);
        self.image_resources[read_version_id.index]
            .versions
            .push(version_info);

        self.nodes[modify_node.0]
            .image_modifies
            .push(RenderGraphImageModify {
                input: read_usage_id,
                output: write_usage_id,
                constraint,
                attachment_type,
            });

        (read_usage_id, write_usage_id)
    }

    fn set_color_attachment(
        &mut self,
        node: RenderGraphNodeId,
        color_attachment_index: usize,
        color_attachment: RenderGraphPassColorAttachmentInfo,
    ) {
        //TODO: Check constraint does not conflict with the matching resolve attachment, if there is one
        let mut node_color_attachments = &mut self.nodes[node.0].color_attachments;
        if node_color_attachments.len() <= color_attachment_index {
            node_color_attachments.resize_with(color_attachment_index + 1, || None);
        }

        assert!(node_color_attachments[color_attachment_index].is_none());
        node_color_attachments[color_attachment_index] = Some(color_attachment);
    }

    fn set_depth_attachment(
        &mut self,
        node: RenderGraphNodeId,
        depth_attachment: RenderGraphPassDepthAttachmentInfo,
    ) {
        let mut node_depth_attachment = &mut self.nodes[node.0].depth_attachment;
        assert!(node_depth_attachment.is_none());
        *node_depth_attachment = Some(depth_attachment);
    }

    fn set_resolve_attachment(
        &mut self,
        node: RenderGraphNodeId,
        resolve_attachment_index: usize,
        resolve_attachment: RenderGraphPassResolveAttachmentInfo,
    ) {
        //TODO: Check constraint is non-MSAA and is not conflicting with the matching color attachment, if there is one
        let mut node_resolve_attachments = &mut self.nodes[node.0].resolve_attachments;
        if node_resolve_attachments.len() <= resolve_attachment_index {
            node_resolve_attachments.resize_with(resolve_attachment_index + 1, || None);
        }

        assert!(node_resolve_attachments[resolve_attachment_index].is_none());
        node_resolve_attachments[resolve_attachment_index] = Some(resolve_attachment);
    }

    pub fn create_color_attachment(
        &mut self,
        node: RenderGraphNodeId,
        color_attachment_index: usize,
        clear_color_value: Option<vk::ClearColorValue>,
        mut constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::COLOR;
        constraint.usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::Color(color_attachment_index);

        // Add the read to the graph
        let create_image = self.add_image_create(
            node,
            attachment_type,
            constraint,
            dsc::ImageLayout::ColorAttachmentOptimal,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::ImageAspectFlags::COLOR,
        );

        self.set_color_attachment(
            node,
            color_attachment_index,
            RenderGraphPassColorAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Create,
                clear_color_value,
                read_image: None,
                write_image: Some(create_image),
            },
        );

        create_image
    }

    pub fn create_depth_attachment(
        &mut self,
        node: RenderGraphNodeId,
        clear_depth_stencil_value: Option<vk::ClearDepthStencilValue>,
        mut constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::DepthStencil;

        // Add the read to the graph
        let create_image = self.add_image_create(
            node,
            attachment_type,
            constraint,
            dsc::ImageLayout::DepthAttachmentOptimal,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH,
        );

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Create,
                clear_depth_stencil_value,
                read_image: None,
                write_image: Some(create_image),
                has_depth: true,
                has_stencil: false,
            },
        );

        create_image
    }

    pub fn create_depth_stencil_attachment(
        &mut self,
        node: RenderGraphNodeId,
        clear_depth_stencil_value: Option<vk::ClearDepthStencilValue>,
        mut constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::DepthStencil;

        // Add the read to the graph
        let create_image = self.add_image_create(
            node,
            attachment_type,
            constraint,
            dsc::ImageLayout::DepthStencilAttachmentOptimal,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
        );

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Create,
                clear_depth_stencil_value,
                read_image: None,
                write_image: Some(create_image),
                has_depth: true,
                has_stencil: true,
            },
        );

        create_image
    }

    pub fn create_resolve_attachment(
        &mut self,
        node: RenderGraphNodeId,
        resolve_attachment_index: usize,
        mut constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::COLOR;
        constraint.usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::Resolve(resolve_attachment_index);

        let create_image = self.add_image_create(
            node,
            attachment_type,
            constraint,
            dsc::ImageLayout::ColorAttachmentOptimal,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::ImageAspectFlags::COLOR,
        );

        self.set_resolve_attachment(
            node,
            resolve_attachment_index,
            RenderGraphPassResolveAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Create,
                write_image: create_image,
            },
        );

        create_image
    }

    pub fn read_color_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        color_attachment_index: usize,
        mut constraint: RenderGraphImageConstraint,
    ) {
        constraint.aspect_flags |= vk::ImageAspectFlags::COLOR;
        constraint.usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::Color(color_attachment_index);

        // Add the read to the graph
        let read_image = self.add_image_read(
            node,
            image,
            attachment_type,
            constraint,
            dsc::ImageLayout::ColorAttachmentOptimal,
            vk::AccessFlags::COLOR_ATTACHMENT_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::ImageAspectFlags::COLOR,
        );

        self.set_color_attachment(
            node,
            color_attachment_index,
            RenderGraphPassColorAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Read,
                clear_color_value: None,
                read_image: Some(read_image),
                write_image: None,
            },
        );
    }

    pub fn read_depth_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
    ) {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::DepthStencil;

        // Add the read to the graph
        let read_image = self.add_image_read(
            node,
            image,
            attachment_type,
            constraint,
            dsc::ImageLayout::DepthAttachmentOptimal,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH,
        );

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Read,
                clear_depth_stencil_value: None,
                read_image: Some(read_image),
                write_image: None,
                has_depth: true,
                has_stencil: false,
            },
        );
    }

    pub fn read_depth_stencil_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
    ) {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::DepthStencil;

        // Add the read to the graph
        let read_image = self.add_image_read(
            node,
            image,
            attachment_type,
            constraint,
            dsc::ImageLayout::DepthStencilAttachmentOptimal,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
        );

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Read,
                clear_depth_stencil_value: None,
                read_image: Some(read_image),
                write_image: None,
                has_depth: true,
                has_stencil: true,
            },
        );
    }

    pub fn modify_color_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        color_attachment_index: usize,
        mut constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::COLOR;
        constraint.usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::Color(color_attachment_index);

        // Add the read to the graph
        let (read_image, write_image) = self.add_image_modify(
            node,
            image,
            attachment_type,
            constraint,
            dsc::ImageLayout::ColorAttachmentOptimal,
            vk::AccessFlags::COLOR_ATTACHMENT_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::ImageAspectFlags::COLOR,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::ImageAspectFlags::COLOR,
        );

        self.set_color_attachment(
            node,
            color_attachment_index,
            RenderGraphPassColorAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Modify,
                clear_color_value: None,
                read_image: Some(read_image),
                write_image: Some(write_image),
            },
        );

        write_image
    }

    pub fn modify_depth_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::DepthStencil;

        // Add the read to the graph
        let (read_image, write_image) = self.add_image_modify(
            node,
            image,
            attachment_type,
            constraint,
            dsc::ImageLayout::DepthAttachmentOptimal,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH,
        );

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Modify,
                clear_depth_stencil_value: None,
                read_image: Some(read_image),
                write_image: Some(write_image),
                has_depth: true,
                has_stencil: false,
            },
        );

        read_image
    }

    pub fn modify_depth_stencil_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let attachment_type = RenderGraphAttachmentType::DepthStencil;

        // Add the read to the graph
        let (read_image, write_image) = self.add_image_modify(
            node,
            image,
            attachment_type,
            constraint,
            dsc::ImageLayout::DepthStencilAttachmentOptimal,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
        );

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Modify,
                clear_depth_stencil_value: None,
                read_image: Some(read_image),
                write_image: Some(write_image),
                has_depth: true,
                has_stencil: true,
            },
        );

        read_image
    }

    pub fn configure_image(
        &mut self,
        image_id: RenderGraphImageUsageId,
    ) -> RenderGraphImageResourceConfigureContext {
        RenderGraphImageResourceConfigureContext {
            graph: self,
            image_id,
        }
    }

    // Add a node which can use resources
    pub fn add_node(&mut self) -> RenderGraphNodeConfigureContext {
        let node_id = RenderGraphNodeId(self.nodes.len());
        self.nodes.push(RenderGraphNode::new(node_id));
        self.configure_node(node_id)
    }

    pub fn configure_node(
        &mut self,
        node_id: RenderGraphNodeId,
    ) -> RenderGraphNodeConfigureContext {
        RenderGraphNodeConfigureContext {
            graph: self,
            node_id,
        }
    }

    //
    // Get nodes
    //
    pub(super) fn node(
        &self,
        node_id: RenderGraphNodeId,
    ) -> &RenderGraphNode {
        &self.nodes[node_id.0]
    }

    pub(super) fn node_mut(
        &mut self,
        node_id: RenderGraphNodeId,
    ) -> &mut RenderGraphNode {
        &mut self.nodes[node_id.0]
    }

    //
    // Get images
    //
    pub(super) fn image_resource(
        &self,
        usage_id: RenderGraphImageUsageId,
    ) -> &RenderGraphImageResource {
        let version = self.image_usages[usage_id.0].version;
        &self.image_resources[version.index]
    }

    pub(super) fn image_resource_mut(
        &mut self,
        usage_id: RenderGraphImageUsageId,
    ) -> &mut RenderGraphImageResource {
        let version = self.image_usages[usage_id.0].version;
        &mut self.image_resources[version.index]
    }

    //
    // Get image version infos
    //
    pub(super) fn image_version_info(
        &self,
        usage_id: RenderGraphImageUsageId,
    ) -> &RenderGraphImageResourceVersionInfo {
        let version = self.image_usages[usage_id.0].version;
        &self.image_resources[version.index].versions[version.version]
    }

    pub(super) fn image_version_info_mut(
        &mut self,
        usage_id: RenderGraphImageUsageId,
    ) -> &mut RenderGraphImageResourceVersionInfo {
        let version = self.image_usages[usage_id.0].version;
        &mut self.image_resources[version.index].versions[version.version]
    }

    pub(super) fn image_version_id(
        &self,
        usage_id: RenderGraphImageUsageId,
    ) -> RenderGraphImageVersionId {
        self.image_usages[usage_id.0].version
    }

    // Recursively called to topologically sort the nodes to determine execution order
    // https://en.wikipedia.org/wiki/Topological_sorting#Depth-first_search
    fn visit_node(
        &self,
        node_id: RenderGraphNodeId,
        visited: &mut Vec<bool>,
        visiting: &mut Vec<bool>,
        visiting_stack: &mut Vec<RenderGraphNodeId>,
        ordered_list: &mut Vec<RenderGraphNodeId>,
    ) {
        // This node is already visited and inserted into ordered_list
        if visited[node_id.0] {
            return;
        }

        // This node is already being visited higher up in the stack. This indicates a cycle in the
        // graph
        if visiting[node_id.0] {
            log::info!("Found cycle in graph");
            log::info!("{:?}", self.node(node_id));
            for v in visiting_stack.iter().rev() {
                log::info!("{:?}", self.node(*v));
            }
            panic!("Graph has a cycle");
        }

        // When we enter the node, mark the node as being in-progress of being visited to help
        // detect cycles in the graph
        visiting[node_id.0] = true;
        visiting_stack.push(node_id);

        //
        // Visit children
        //
        //log::info!("  Begin visit {:?}", node_id);
        let node = self.node(node_id);

        // The order of visiting nodes here matters. If we consider merging subpasses, trying to
        // visit a node we want to merge with last will show that there are no indirect dependencies
        // between the merge candidate and the node we are visiting now. However, if there are
        // indirect dependencies, visiting a different node might mark the merge candidate as
        // already visited. So while we could try to visit the merge candidate last, it won't
        // guarantee that the merge candidate will actually be inserted later in the ordered_list
        // than any other dependency
        //
        // Also if we are trying to merge with a dependency (which will execute before us), then any
        // other requirements we have will need to be fulfilled before that node starts
        //
        // Other priorities might be to front-load anything compute-light/GPU-heavy. We can do this
        // by some sort of flagging/priority system to influence this logic. We could also expose
        // a way for end-users to add arbitrary dependencies for the sole purpose of influencing the
        // ordering here.
        //
        // As a first pass implementation, just ensure any merge dependencies are visited last so
        // that we will be more likely to be able to merge passes

        //
        // Delay visiting these nodes so that we get the best chance possible of mergeable passes
        // being adjacent to each other in the orderered list
        //
        let mut merge_candidates = FnvHashSet::default();
        if let Some(depth_attachment) = &node.depth_attachment {
            if let Some(read_image) = depth_attachment.read_image {
                let upstream_node = self.image_version_info(read_image).creator_node;

                // This might be too expensive to check
                if self.can_passes_merge(upstream_node, node.id()) {
                    merge_candidates.insert(upstream_node);
                }
            }
        }

        for color_attachment in &node.color_attachments {
            // If this is an attachment we are reading, then the node that created it is a merge candidate
            if let Some(read_image) = color_attachment.as_ref().and_then(|x| x.read_image) {
                let upstream_node = self.image_version_info(read_image).creator_node;

                // This might be too expensive to check
                if self.can_passes_merge(upstream_node, node.id()) {
                    merge_candidates.insert(upstream_node);
                }
            }
        }

        //
        // Visit all the nodes we aren't delaying
        //
        for read in &node.image_reads {
            let upstream_node = self.image_version_info(read.image).creator_node;
            if !merge_candidates.contains(&upstream_node) {
                self.visit_node(
                    upstream_node,
                    visited,
                    visiting,
                    visiting_stack,
                    ordered_list,
                );
            }
        }

        for modify in &node.image_modifies {
            let upstream_node = self.image_version_info(modify.input).creator_node;
            if !merge_candidates.contains(&upstream_node) {
                self.visit_node(
                    upstream_node,
                    visited,
                    visiting,
                    visiting_stack,
                    ordered_list,
                );
            }
        }

        //
        // Now visit the nodes we delayed visiting
        //
        for merge_candidate in merge_candidates {
            self.visit_node(
                merge_candidate,
                visited,
                visiting,
                visiting_stack,
                ordered_list,
            );
        }

        // All our pre-requisites were visited, so it's now safe to push this node onto the
        // orderered list
        ordered_list.push(node_id);
        visited[node_id.0] = true;

        // We are no longer visiting this node
        //log::info!("  End visit {:?}", node_id);
        visiting_stack.pop();
        visiting[node_id.0] = false;
    }

    fn determine_node_order(&self) -> Vec<RenderGraphNodeId> {
        // As we depth-first traverse nodes, mark them as visiting and push them onto this stack.
        // We will use this to detect and print out cycles
        let mut visiting = vec![false; self.nodes.len()];
        let mut visiting_stack = Vec::default();

        // The order of nodes, upstream to downstream. As we depth-first traverse nodes, push nodes
        // with no unvisited dependencies onto this list and mark them as visited
        let mut visited = vec![false; self.nodes.len()];
        let mut ordered_list = Vec::default();

        // Iterate all the images we need to output. This will visit all the nodes we need to execute,
        // potentially leaving out nodes we can cull.
        for output_image_id in &self.output_images {
            // Find the node that creates the output image
            let output_node = self.image_version_info(output_image_id.usage).creator_node;
            log::info!(
                "Traversing dependencies of output image created by node {:?} {:?}",
                output_node,
                self.node(output_node).name()
            );

            self.visit_node(
                output_node,
                &mut visited,
                &mut visiting,
                &mut visiting_stack,
                &mut ordered_list,
            );
        }

        ordered_list
    }

    //TODO: Redundant with can_merge_nodes
    fn can_passes_merge(
        &self,
        prev: RenderGraphNodeId,
        next: RenderGraphNodeId,
    ) -> bool {
        // Reasons to reject merging:
        // - Queues match and are not compute based
        // - Global flag to disable merging
        // - Don't need to mipmap previous outputs
        // - Next doesn't need to sample from any previous output (image or buffer)
        // - Using different depth attachment? Not sure why

        // Reasons to allow merging:
        // - They share any color or depth attachments

        false
    }

    fn get_create_usage(
        &self,
        usage: RenderGraphImageUsageId,
    ) -> RenderGraphImageUsageId {
        let version = self.image_usages[usage.0].version;
        self.image_resources[version.index].versions[version.version].create_usage
    }

    fn determine_image_constraints(
        &self,
        node_execution_order: &[RenderGraphNodeId],
    ) -> DetermineImageConstraintsResult {
        let mut image_version_states: FnvHashMap<
            RenderGraphImageUsageId,
            RenderGraphImageConstraint,
        > = Default::default();

        log::info!("Propagating image constraints");

        log::info!("  Set up input images");

        //
        // Propagate input image state specifications into images. Inputs are fully specified and
        // their constraints will never be overwritten
        //
        for input_image in &self.input_images {
            log::info!(
                "    Image {:?} {:?}",
                input_image,
                self.image_resource(input_image.usage).name
            );
            image_version_states
                .entry(self.get_create_usage(input_image.usage))
                .or_default()
                .set(&input_image.specification);

            // Don't bother setting usage constraint for 0
        }

        log::info!("  Propagate image constraints FORWARD");

        //
        // Iterate forward through nodes to determine what states images need to be in. We only need
        // to handle operations that produce a new version of a resource. These operations do not
        // need to fully specify image info, but whatever they do specify will be carried forward
        // and not overwritten
        //
        for node_id in node_execution_order.iter() {
            let node = self.node(*node_id);
            log::info!("    node {:?} {:?}", node_id, node.name());

            //
            // Propagate constraints into images this node creates.
            //
            for image_create in &node.image_creates {
                let image = self.image_version_info(image_create.image);
                // An image cannot be created within the graph and imported externally at the same
                // time. (The code here assumes no input and will not produce correct results if there
                // is an input image)
                //TODO: Input images are broken, we don't properly represent an image being created
                // OR receiving an input. We probably need to make creator in
                // RenderGraphImageResourceVersionInfo Option or an enum with input/create options
                //assert!(image.input_image.is_none());

                log::info!(
                    "      Create image {:?} {:?}",
                    image_create.image,
                    self.image_resource(image_create.image).name
                );

                let mut version_state = image_version_states
                    .entry(self.get_create_usage(image_create.image))
                    .or_default();

                if !version_state.try_merge(&image_create.constraint) {
                    // Should not happen as this should be our first visit to this image
                    panic!("Unexpected constraints on image being created");
                }

                log::info!(
                    "        Forward propagate constraints {:?} {:?}",
                    image_create.image,
                    version_state
                );

                // Don't bother setting usage constraint for 0
            }

            // We don't need to propagate anything forward on reads

            //
            // Propagate constraints forward for images being modified.
            //
            for image_modify in &node.image_modifies {
                log::info!(
                    "      Modify image {:?} {:?} -> {:?} {:?}",
                    image_modify.input,
                    self.image_resource(image_modify.input).name,
                    image_modify.output,
                    self.image_resource(image_modify.output).name
                );

                let image = self.image_version_info(image_modify.input);
                //log::info!("  Modify image {:?} {:?}", image_modify.input, self.image_resource(image_modify.input).name);
                let input_state = image_version_states
                    .entry(self.get_create_usage(image_modify.input))
                    .or_default();
                let mut image_modify_constraint = image_modify.constraint.clone();

                // Merge the input image constraints with this node's constraints
                if !image_modify_constraint
                    .partial_merge(&input_state /*.combined_constraints*/)
                {
                    // This would need to be resolved by inserting some sort of fixup

                    // We will detect this on the backward pass, no need to do anything here
                    /*
                    let required_fixup = ImageConstraintRequiredFixup::Modify(node.id(), image_modify.clone());
                    log::info!("        *** Found required fixup: {:?}", required_fixup);
                    log::info!("            {:?}", input_state.constraint);
                    log::info!("            {:?}", image_modify_constraint);
                    required_fixups.push(required_fixup);
                    */
                    //log::info!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", input_state.combined_constraints, image_modify.constraint);
                }

                //TODO: Should we set the usage constraint here? For now will wait until backward propagation

                let mut output_state = image_version_states
                    .entry(self.get_create_usage(image_modify.output))
                    .or_default();

                // Now propagate forward to the image version we write
                if !output_state
                    //.combined_constraints
                    .partial_merge(&image_modify_constraint)
                {
                    // // This should only happen if modifying an input image
                    // assert!(image.input_image.is_some());
                    // This would need to be resolved by inserting some sort of fixup

                    // We will detect this on the backward pass, no need to do anything here
                    /*
                    let required_fixup = ImageConstraintRequiredFixup::Modify(node.id(), image_modify.clone());
                    log::info!("        *** Found required fixup {:?}", required_fixup);
                    log::info!("            {:?}", image_modify_constraint);
                    log::info!("            {:?}", output_state.constraint);
                    required_fixups.push(required_fixup);
                    */
                    //log::info!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", output_state.constraint, input_state.constraint);
                }

                log::info!("        Forward propagate constraints {:?}", output_state);
            }
        }

        log::info!("  Set up output images");

        //
        // Propagate output image state specifications into images
        //
        for output_image in &self.output_images {
            log::info!(
                "    Image {:?} {:?}",
                output_image,
                self.image_resource(output_image.usage).name
            );
            let mut output_image_version_state = image_version_states
                .entry(self.get_create_usage(output_image.usage))
                .or_default();
            let output_constraint = output_image.specification.clone().into();
            if !output_image_version_state.partial_merge(&output_constraint) {
                // This would need to be resolved by inserting some sort of fixup
                /*
                log::info!("      *** Found required OUTPUT fixup");
                log::info!(
                    "          {:?}",
                    output_image_version_state //.combined_constraints
                );
                log::info!("          {:?}", output_image.specification);
                */
                //log::info!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", output_image_version_state.constraint, output_specification);
            }

            image_version_states.insert(
                output_image.usage,
                output_image.specification.clone().into(),
            );
        }

        log::info!("  Propagate image constraints BACKWARD");

        //
        // Iterate backwards through nodes, determining the state the image must be in at every
        // step
        //
        for node_id in node_execution_order.iter().rev() {
            let node = self.node(*node_id);
            log::info!("    node {:?} {:?}", node_id, node.name());

            // Don't need to worry about creates, we back propagate to them when reading/modifying

            //
            // Propagate backwards from reads
            //
            for image_read in &node.image_reads {
                log::info!(
                    "      Read image {:?} {:?}",
                    image_read.image,
                    self.image_resource(image_read.image).name
                );

                let version_state = image_version_states
                    .entry(self.get_create_usage(image_read.image))
                    .or_default();
                if !version_state
                    //.combined_constraints
                    .partial_merge(&image_read.constraint)
                {
                    // This would need to be resolved by inserting some sort of fixup
                    /*
                    log::info!("        *** Found required READ fixup");
                    log::info!(
                        "            {:?}",
                        version_state /*.combined_constraints*/
                    );
                    log::info!("            {:?}", image_read.constraint);
                    */
                    //log::info!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", version_state.constraint, image_read.constraint);
                }

                // If this is an image read with no output, it's possible the constraint on the read is incomplete.
                // So we need to merge the image state that may have information forward-propagated
                // into it with the constraints on the read. (Conceptually it's like we're forward
                // propagating here because the main forward propagate pass does not handle reads.
                // TODO: We could consider moving this to the forward pass
                let mut image_read_constraint = image_read.constraint.clone();
                image_read_constraint.partial_merge(&version_state /*.combined_constraints*/);
                log::info!(
                    "        Read constraints will be {:?}",
                    image_read_constraint
                );
                if let Some(spec) = image_read_constraint.try_convert_to_specification() {
                    image_version_states.insert(image_read.image, spec.into());
                } else {
                    panic!(
                        "Not enough information in the graph to determine the specification for image {:?} {:?} being read by node {:?} {:?}. Constraints are: {:?}",
                        image_read.image,
                        self.image_resource(image_read.image).name,
                        node.id(),
                        node.name(),
                        image_version_states.get(&image_read.image)
                    );
                }
            }

            //
            // Propagate backwards from modifies
            //
            for image_modify in &node.image_modifies {
                log::info!(
                    "      Modify image {:?} {:?} <- {:?} {:?}",
                    image_modify.input,
                    self.image_resource(image_modify.input).name,
                    image_modify.output,
                    self.image_resource(image_modify.output).name
                );
                // The output image constraint already takes image_modify.constraint into account from
                // when we propagated image constraints forward
                let output_image_constraint = image_version_states
                    .entry(self.get_create_usage(image_modify.output))
                    .or_default()
                    .clone();
                let mut input_state = image_version_states
                    .entry(self.get_create_usage(image_modify.input))
                    .or_default();
                if !input_state.partial_merge(&output_image_constraint) {
                    // This would need to be resolved by inserting some sort of fixup
                    /*
                    log::info!("        *** Found required MODIFY fixup");
                    log::info!(
                        "            {:?}",
                        input_state /*.combined_constraints*/
                    );
                    log::info!("            {:?}", image_modify.constraint);
                    */
                    //log::info!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", input_state.constraint, image_modify.constraint);
                }

                image_version_states.insert(image_modify.input, output_image_constraint.clone());
            }
        }

        let mut image_specs = FnvHashMap::default();

        for (k, v) in image_version_states {
            image_specs.insert(k, v.try_convert_to_specification().unwrap());
        }

        DetermineImageConstraintsResult {
            images: image_specs,
        }
    }

    fn insert_resolves(
        &mut self,
        node_execution_order: &[RenderGraphNodeId],
        image_constraint_results: &mut DetermineImageConstraintsResult,
    ) {
        log::info!("Insert resolves in graph where necessary");
        for node_id in node_execution_order {
            let mut resolves_to_add = Vec::default();

            let node = self.node(*node_id);
            log::info!("  node {:?}", node_id);
            // Iterate through all color attachments
            for (color_attachment_index, color_attachment) in
                node.color_attachments.iter().enumerate()
            {
                if let Some(color_attachment) = color_attachment {
                    log::info!("    color attachment {}", color_attachment_index);
                    // If this color attachment outputs an image
                    if let Some(write_image) = color_attachment.write_image {
                        let write_version = self.image_usages[write_image.0].version;
                        // Skip if it's not an MSAA image
                        let write_spec = image_constraint_results.specification(write_image);
                        if write_spec.samples == vk::SampleCountFlags::TYPE_1 {
                            log::info!("      already non-MSAA");
                            continue;
                        }

                        // Calculate the spec that we would have after the resolve
                        let mut resolve_spec = write_spec.clone();
                        resolve_spec.samples = vk::SampleCountFlags::TYPE_1;

                        let mut usages_to_move = vec![];

                        // Look for any usages we need to fix
                        for (usage_index, read_usage) in self
                            .image_version_info(write_image)
                            .read_usages
                            .iter()
                            .enumerate()
                        {
                            log::info!(
                                "      usage {}, {:?}",
                                usage_index,
                                self.image_usages[read_usage.0].usage_type
                            );
                            let read_spec = image_constraint_results.specification(*read_usage);
                            if *read_spec == *write_spec {
                                continue;
                            } else if *read_spec == resolve_spec {
                                usages_to_move.push(*read_usage);
                                break;
                            } else {
                                log::info!("        incompatibility cannot be fixed via renderpass resolve");
                                log::info!("{:?}", resolve_spec);
                                log::info!("{:?}", read_spec);
                            }
                        }

                        if !usages_to_move.is_empty() {
                            resolves_to_add.push((
                                color_attachment_index,
                                resolve_spec,
                                usages_to_move,
                            ));
                        }
                    }
                }
            }

            for (resolve_attachment_index, resolve_spec, usages_to_move) in resolves_to_add {
                log::info!(
                    "        ADDING RESOLVE FOR NODE {:?} ATTACHMENT {}",
                    node_id,
                    resolve_attachment_index
                );
                let image = self.create_resolve_attachment(
                    *node_id,
                    resolve_attachment_index,
                    resolve_spec.clone().into(),
                );
                image_constraint_results
                    .images
                    .insert(image, resolve_spec.into());

                for usage in usages_to_move {
                    let from = self.image_usages[usage.0].version;
                    let to = self.image_usages[image.0].version;
                    log::info!(
                        "          MOVE USAGE {:?} from {:?} to {:?}",
                        usage,
                        from,
                        to
                    );
                    self.move_read_usage_to_image(usage, from, to)
                }
            }
        }
    }

    fn move_read_usage_to_image(
        &mut self,
        usage: RenderGraphImageUsageId,
        from: RenderGraphImageVersionId,
        to: RenderGraphImageVersionId,
    ) {
        self.image_resources[from.index].versions[from.version].remove_read_usage(usage);
        self.image_resources[to.index].versions[to.version].add_read_usage(usage);
    }

    fn assign_physical_images(
        &mut self,
        node_execution_order: &[RenderGraphNodeId],
        image_constraint_results: &mut DetermineImageConstraintsResult,
    ) -> AssignPhysicalImagesResult {
        let mut map_image_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId> =
            FnvHashMap::default();
        let mut physical_image_infos: FnvHashMap<PhysicalImageId, PhysicalImageInfo> =
            FnvHashMap::default();
        // let mut physical_image_usages: FnvHashMap<PhysicalImageId, Vec<RenderGraphImageUsageId>> =
        //     FnvHashMap::default();
        // let mut physical_image_versions: FnvHashMap<
        //     PhysicalImageId,
        //     Vec<RenderGraphImageVersionId>,
        // > = FnvHashMap::default();

        let mut physical_image_id_allocator = PhysicalImageIdAllocator::default();
        //TODO: Associate input images here? We can wait until we decide which images are shared
        log::info!("Associate images written by nodes with physical images");
        for node in node_execution_order.iter() {
            let node = self.node(*node);
            log::info!("  node {:?} {:?}", node.id().0, node.name());

            // A list of all images we write to from this node. We will try to share the images
            // being written forward into the nodes of downstream reads. This can chain such that
            // the same image is shared by many nodes
            let mut written_images = vec![];

            for create in &node.image_creates {
                // An image that's created always allocates an image (we can try to alias/pool these later)
                let physical_image = physical_image_id_allocator.allocate();
                log::info!(
                    "    Create {:?} will use image {:?}",
                    create.image,
                    physical_image
                );
                map_image_to_physical.insert(create.image, physical_image);
                // physical_image_usages
                //     .entry(physical_image)
                //     .or_default()
                //     .push(create.image);
                // physical_image_versions
                //     .entry(physical_image)
                //     .or_default()
                //     .push(self.image_usages[create.image.0].version);
                let mut physical_image_info =
                    PhysicalImageInfo::new(image_constraint_results.images[&create.image].clone());

                physical_image_info.usages.push(create.image);
                physical_image_info
                    .versions
                    .push(self.image_usages[create.image.0].version);
                physical_image_infos.insert(physical_image, physical_image_info);

                // Queue this image write to try to share the image forward
                written_images.push(create.image);
            }

            for modify in &node.image_modifies {
                // The physical image in the read portion of a modify must also be the write image.
                // The format of the input/output is guaranteed to match
                assert_eq!(
                    image_constraint_results.specification(modify.input),
                    image_constraint_results.specification(modify.output)
                );

                // Assign the image
                let physical_image = map_image_to_physical.get(&modify.input).unwrap().clone();
                log::info!(
                    "    Modify {:?} will pass through image {:?}",
                    modify.output,
                    physical_image
                );
                map_image_to_physical.insert(modify.output, physical_image);
                // physical_image_usages
                //     .entry(physical_image)
                //     .or_default()
                //     .push(modify.output);
                // physical_image_versions
                //     .entry(physical_image)
                //     .or_default()
                //     .push(self.image_usages[modify.output.0].version);
                let mut physical_image_info =
                    PhysicalImageInfo::new(image_constraint_results.images[&modify.output].clone());
                physical_image_info.usages.push(modify.output);
                physical_image_info
                    .versions
                    .push(self.image_usages[modify.output.0].version);
                physical_image_infos.insert(physical_image, physical_image_info);

                // Queue this image write to try to share the image forward
                written_images.push(modify.output);
            }

            for written_image in written_images {
                // Count the downstream users of this image based on if they need read-only access
                // or write access. We need this information to determine which usages we can share
                // the output data with.
                //TODO: This could be smarter to handle the case of a resource being read/written
                // in different lifetimes
                let written_image_version_info = self.image_version_info(written_image);
                let mut read_count = 0;
                let mut write_count = 0;
                for usage in &written_image_version_info.read_usages {
                    if self.image_usages[usage.0].usage_type.is_read_only() {
                        read_count += 1;
                    } else {
                        write_count += 1;
                    }
                }

                // If we don't already have an image
                // let written_physical_image = mapping.entry(written_image)
                //     .or_insert_with(|| image_allocator.allocate(&image_constraint_results.specification(written_image)));

                let write_physical_image = *map_image_to_physical.get(&written_image).unwrap();
                let write_type = self.image_usages[written_image.0].usage_type;

                for usage_resource_id in &written_image_version_info.read_usages {
                    // We can't share images if they aren't the same format
                    let written_spec = image_constraint_results.specification(written_image);
                    let usage_spec = image_constraint_results.specification(*usage_resource_id);
                    let specifications_match = *written_spec == *usage_spec;

                    // We can't share images unless it's a read or it's an exclusive write
                    let is_read_or_exclusive_write = (read_count > 0
                        && self.image_usages[usage_resource_id.0]
                            .usage_type
                            .is_read_only())
                        || write_count <= 1;

                    let read_type = self.image_usages[usage_resource_id.0].usage_type;
                    if specifications_match && is_read_or_exclusive_write {
                        // it's a shared read or an exclusive write
                        log::info!(
                            "    Usage {:?} will share an image with {:?} ({:?} -> {:?})",
                            written_image,
                            usage_resource_id,
                            write_type,
                            read_type
                        );
                        let overwritten_image =
                            map_image_to_physical.insert(*usage_resource_id, write_physical_image);
                        // physical_image_usages
                        //     .get_mut(&write_physical_image)
                        //     .unwrap()
                        //     //.or_default()
                        //     .push(*usage_resource_id);

                        physical_image_infos
                            .get_mut(&write_physical_image)
                            .unwrap()
                            .usages
                            .push(*usage_resource_id);

                        assert!(overwritten_image.is_none());
                    } else {
                        // allocate new image
                        let specification = image_constraint_results.specification(written_image);
                        let physical_image = physical_image_id_allocator.allocate();
                        log::info!(
                            "    Allocate image {:?} for {:?} ({:?} -> {:?})  (specifications_match match: {} is_read_or_exclusive_write: {})",
                            physical_image,
                            usage_resource_id,
                            write_type,
                            read_type,
                            specifications_match,
                            is_read_or_exclusive_write
                        );
                        if !specifications_match {
                            log::trace!("      written: {:?}", written_spec);
                            log::trace!("      usage  : {:?}", usage_spec);
                        }
                        let overwritten_image =
                            map_image_to_physical.insert(*usage_resource_id, physical_image);
                        // physical_image_usages
                        //     .get_mut(&physical_image)
                        //     .unwrap()
                        //     //.or_default()
                        //     .push(*usage_resource_id);
                        // physical_image_infos
                        //     .get_mut(&physical_image)
                        //     .unwrap()
                        //     .usages
                        //     .push(*usage_resource_id);

                        let mut physical_image_info = PhysicalImageInfo::new(
                            image_constraint_results.images[&usage_resource_id].clone(),
                        );
                        physical_image_info.usages.push(*usage_resource_id);
                        physical_image_infos.insert(physical_image, physical_image_info);
                        assert!(overwritten_image.is_none());
                    }
                }
            }
        }

        // vulkan image layouts: https://github.com/nannou-org/nannou/issues/271#issuecomment-465876622
        AssignPhysicalImagesResult {
            //physical_image_usages,
            map_image_to_physical,
            //physical_image_versions,
            physical_image_infos,
        }
    }

    //TODO: Redundant with can_passes_merge
    fn can_merge_nodes(
        &self,
        before_node_id: RenderGraphNodeId,
        after_node_id: RenderGraphNodeId,
        image_constraints: &DetermineImageConstraintsResult,
        physical_images: &AssignPhysicalImagesResult,
    ) -> bool {
        let before_node = self.node(before_node_id);
        let after_node = self.node(after_node_id);

        //TODO: Reject if not on the same queue, and not both graphics nodes

        //TODO: Reject if after reads something that before writes

        //TODO: Check if depth attachments are not the same?
        // https://developer.arm.com/documentation/101897/0200/fragment-shading/multipass-rendering
        // implies that the depth buffer must not change but this could be mali specific

        //TODO: Verify that some color or depth attachment gets used between the passes to justify
        // merging them. Unclear if this is necessarily desirable but likely is

        // For now don't merge anything
        false
    }

    fn build_physical_passes(
        &self,
        node_execution_order: &[RenderGraphNodeId],
        image_constraints: &DetermineImageConstraintsResult,
        physical_images: &AssignPhysicalImagesResult,
        //determine_image_layouts_result: &DetermineImageLayoutsResult
    ) -> Vec<RenderGraphPass> {
        let mut pass_node_sets = Vec::default();

        let mut subpass_nodes = Vec::default();
        for node_id in node_execution_order {
            let mut add_to_current = true;
            for subpass_node in &subpass_nodes {
                if !self.can_merge_nodes(
                    *subpass_node,
                    *node_id,
                    image_constraints,
                    physical_images,
                ) {
                    add_to_current = false;
                    break;
                }
            }

            if add_to_current {
                subpass_nodes.push(*node_id);
            } else {
                pass_node_sets.push(subpass_nodes);
                subpass_nodes = Vec::default();
                subpass_nodes.push(*node_id);
            }
        }

        if !subpass_nodes.is_empty() {
            pass_node_sets.push(subpass_nodes);
        }

        log::info!("gather pass info");
        let mut passes = Vec::default();
        for pass_node_set in pass_node_sets {
            log::info!("  nodes in pass: {:?}", pass_node_set);
            fn find_or_insert_attachment(
                attachments: &mut Vec<RenderGraphPassAttachment>,
                image: PhysicalImageId,
            ) -> (usize, bool) {
                if let Some(position) = attachments.iter().position(|x| x.image == image) {
                    (position, false)
                } else {
                    attachments.push(RenderGraphPassAttachment {
                        image,
                        load_op: vk::AttachmentLoadOp::DONT_CARE,
                        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                        store_op: vk::AttachmentStoreOp::DONT_CARE,
                        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                        clear_color: Default::default(),
                        format: vk::Format::UNDEFINED,
                        samples: vk::SampleCountFlags::TYPE_1,
                        initial_layout: dsc::ImageLayout::Undefined,
                        final_layout: dsc::ImageLayout::Undefined,
                    });
                    (attachments.len() - 1, true)
                }
            }

            let mut pass = RenderGraphPass {
                attachments: Default::default(),
                subpasses: Default::default(),
            };

            for node_id in pass_node_set {
                log::info!("    subpass node: {:?}", node_id);
                let mut subpass = RenderGraphSubpass {
                    node: node_id,
                    color_attachments: Default::default(),
                    resolve_attachments: Default::default(),
                    depth_attachment: Default::default(),
                };

                let subpass_node = self.node(node_id);

                for (color_attachment_index, color_attachment) in
                    subpass_node.color_attachments.iter().enumerate()
                {
                    if let Some(color_attachment) = color_attachment {
                        let read_or_write_usage = color_attachment
                            .read_image
                            .or(color_attachment.write_image)
                            .unwrap();
                        let physical_image = physical_images
                            .map_image_to_physical
                            .get(&read_or_write_usage)
                            .unwrap();
                        let version_id = self.image_version_id(read_or_write_usage);
                        let specification =
                            image_constraints.images.get(&read_or_write_usage).unwrap();
                        log::info!("      physical attachment (color): {:?}", physical_image);

                        let (pass_attachment_index, is_first_usage) =
                            find_or_insert_attachment(&mut pass.attachments, *physical_image);
                        subpass.color_attachments[color_attachment_index] =
                            Some(pass_attachment_index);

                        let mut attachment = &mut pass.attachments[pass_attachment_index];
                        if is_first_usage {
                            // Check if we load or clear
                            if color_attachment.read_image.is_some() {
                                attachment.load_op = vk::AttachmentLoadOp::LOAD;
                            } else if color_attachment.clear_color_value.is_some() {
                                attachment.load_op = vk::AttachmentLoadOp::CLEAR;
                                attachment.clear_color = Some(AttachmentClearValue::Color(
                                    color_attachment.clear_color_value.unwrap(),
                                ))
                            };

                            attachment.format = specification.format;
                            attachment.samples = specification.samples;
                        };

                        let store_op = if let Some(write_image) = color_attachment.write_image {
                            if !self.image_version_info(write_image).read_usages.is_empty() {
                                vk::AttachmentStoreOp::STORE
                            } else {
                                vk::AttachmentStoreOp::DONT_CARE
                            }
                        } else {
                            vk::AttachmentStoreOp::DONT_CARE
                        };

                        attachment.store_op = store_op;
                        attachment.stencil_store_op = vk::AttachmentStoreOp::DONT_CARE;
                    }
                }

                for (resolve_attachment_index, resolve_attachment) in
                    subpass_node.resolve_attachments.iter().enumerate()
                {
                    if let Some(resolve_attachment) = resolve_attachment {
                        let write_image = resolve_attachment.write_image;
                        let physical_image = physical_images
                            .map_image_to_physical
                            .get(&write_image)
                            .unwrap();
                        let version_id = self.image_version_id(write_image);
                        let specification = image_constraints.images.get(&write_image).unwrap();
                        log::info!("      physical attachment (resolve): {:?}", physical_image);

                        let (pass_attachment_index, is_first_usage) =
                            find_or_insert_attachment(&mut pass.attachments, *physical_image);
                        subpass.resolve_attachments[resolve_attachment_index] =
                            Some(pass_attachment_index);

                        assert!(is_first_usage); // Not sure if this assert is valid
                        let mut attachment = &mut pass.attachments[pass_attachment_index];
                        attachment.format = specification.format;
                        attachment.samples = specification.samples;

                        //TODO: Should we skip resolving if there is no reader?
                        let store_op =
                            if !self.image_version_info(write_image).read_usages.is_empty() {
                                vk::AttachmentStoreOp::STORE
                            } else {
                                vk::AttachmentStoreOp::DONT_CARE
                            };

                        attachment.store_op = store_op;
                        attachment.stencil_store_op = vk::AttachmentStoreOp::DONT_CARE;
                    }
                }

                if let Some(depth_attachment) = &subpass_node.depth_attachment {
                    let read_or_write_usage = depth_attachment
                        .read_image
                        .or(depth_attachment.write_image)
                        .unwrap();
                    let physical_image = physical_images
                        .map_image_to_physical
                        .get(&read_or_write_usage)
                        .unwrap();
                    let version_id = self.image_version_id(read_or_write_usage);
                    let specification = image_constraints.images.get(&read_or_write_usage).unwrap();
                    log::info!("      physical attachment (depth): {:?}", physical_image);

                    let (pass_attachment_index, is_first_usage) =
                        find_or_insert_attachment(&mut pass.attachments, *physical_image);
                    subpass.depth_attachment = Some(pass_attachment_index);

                    let mut attachment = &mut pass.attachments[pass_attachment_index];
                    if is_first_usage {
                        // Check if we load or clear
                        //TODO: Support load_op for stencil

                        if depth_attachment.read_image.is_some() {
                            if depth_attachment.has_depth {
                                attachment.load_op = vk::AttachmentLoadOp::LOAD;
                            }

                            if depth_attachment.has_stencil {
                                attachment.stencil_load_op = vk::AttachmentLoadOp::LOAD;
                            }
                        } else if depth_attachment.clear_depth_stencil_value.is_some() {
                            if depth_attachment.has_depth {
                                attachment.load_op = vk::AttachmentLoadOp::CLEAR;
                            }
                            if depth_attachment.has_stencil {
                                attachment.stencil_load_op = vk::AttachmentLoadOp::CLEAR;
                            }
                            attachment.clear_color = Some(AttachmentClearValue::DepthStencil(
                                depth_attachment.clear_depth_stencil_value.unwrap(),
                            ))
                        };

                        attachment.format = specification.format;
                        attachment.samples = specification.samples;
                    };

                    let store_op = if let Some(write_image) = depth_attachment.write_image {
                        if !self.image_version_info(write_image).read_usages.is_empty() {
                            vk::AttachmentStoreOp::STORE
                        } else {
                            vk::AttachmentStoreOp::DONT_CARE
                        }
                    } else {
                        vk::AttachmentStoreOp::DONT_CARE
                    };

                    if depth_attachment.has_depth {
                        attachment.store_op = store_op;
                    }

                    if depth_attachment.has_stencil {
                        attachment.stencil_store_op = store_op;
                    }
                }

                //TODO: Input attachments

                pass.subpasses.push(subpass);
            }

            passes.push(pass);
        }

        passes
    }

    fn build_node_barriers(
        &self,
        node_execution_order: &[RenderGraphNodeId],
        image_constraints: &DetermineImageConstraintsResult,
        physical_images: &AssignPhysicalImagesResult,
        //determine_image_layouts_result: &DetermineImageLayoutsResult,
    ) -> Vec<RenderGraphNodeImageBarriers> {
        let mut barriers = Vec::default();

        for node_id in node_execution_order {
            let node = self.node(*node_id);
            //let mut invalidate_barriers: FnvHashMap<PhysicalImageId, RenderGraphImageBarrier> = Default::default();
            //let mut flush_barriers: FnvHashMap<PhysicalImageId, RenderGraphImageBarrier> = Default::default();
            let mut node_barriers: FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers> =
                Default::default();

            for (color_attachment_index, color_attachment) in
                node.color_attachments.iter().enumerate()
            {
                if let Some(color_attachment) = color_attachment {
                    let read_or_write_usage = color_attachment
                        .read_image
                        .or(color_attachment.write_image)
                        .unwrap();
                    let physical_image = physical_images
                        .map_image_to_physical
                        .get(&read_or_write_usage)
                        .unwrap();
                    let version_id = self.image_version_id(read_or_write_usage);

                    let mut barrier = node_barriers.entry(*physical_image).or_insert_with(|| {
                        RenderGraphPassImageBarriers::new(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    });

                    if let Some(read_image) = color_attachment.read_image {
                        barrier.invalidate.access_flags |= vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                            | vk::AccessFlags::COLOR_ATTACHMENT_READ;
                        barrier.invalidate.stage_flags |=
                            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
                        //barrier.invalidate.layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
                        //invalidate_barrier.layout = determine_image_layouts_result.image_layouts[&version_id].read_layout.into();
                    }

                    if let Some(write_image) = color_attachment.write_image {
                        barrier.flush.access_flags |= vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
                        barrier.flush.stage_flags |=
                            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
                        //barrier.flush.layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
                        //flush_barrier.layout = determine_image_layouts_result.image_layouts[&version_id].write_layout.into();
                    }
                }
            }

            for (resolve_attachment_index, resolve_attachment) in
                node.resolve_attachments.iter().enumerate()
            {
                if let Some(resolve_attachment) = resolve_attachment {
                    let physical_image = physical_images
                        .map_image_to_physical
                        .get(&resolve_attachment.write_image)
                        .unwrap();
                    let version_id = self.image_version_id(resolve_attachment.write_image);

                    let mut barrier = node_barriers.entry(*physical_image).or_insert_with(|| {
                        RenderGraphPassImageBarriers::new(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    });

                    barrier.flush.access_flags |= vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
                    barrier.flush.stage_flags |= vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
                    //barrier.flush.layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
                    //flush_barrier.layout = determine_image_layouts_result.image_layouts[&version_id].write_layout.into();
                }
            }

            if let Some(depth_attachment) = &node.depth_attachment {
                let read_or_write_usage = depth_attachment
                    .read_image
                    .or(depth_attachment.write_image)
                    .unwrap();
                let physical_image = physical_images
                    .map_image_to_physical
                    .get(&read_or_write_usage)
                    .unwrap();
                let version_id = self.image_version_id(read_or_write_usage);

                let mut barrier = node_barriers.entry(*physical_image).or_insert_with(|| {
                    RenderGraphPassImageBarriers::new(
                        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    )
                });

                if depth_attachment.read_image.is_some() && depth_attachment.write_image.is_some() {
                    //barrier.invalidate.layout = vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
                    //barrier.invalidate.layout = determine_image_layouts_result.image_layouts[&version_id].read_layout.into();
                    barrier.invalidate.access_flags |=
                        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                            | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
                    barrier.invalidate.stage_flags |= vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;

                    //barrier.flush.layout = vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
                    //barrier.flush.layout = determine_image_layouts_result.image_layouts[&version_id].write_layout.into();
                    barrier.flush.access_flags |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
                    barrier.flush.stage_flags |= vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
                } else if depth_attachment.read_image.is_some() {
                    //barrier.invalidate.layout = vk::ImageLayout::DEPTH_READ_ONLY_STENCIL_ATTACHMENT_OPTIMAL;
                    //barrier.invalidate.layout = determine_image_layouts_result.image_layouts[&version_id].read_layout.into();
                    barrier.invalidate.access_flags |=
                        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ;
                    barrier.invalidate.stage_flags |= vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
                } else {
                    assert!(depth_attachment.write_image.is_some());
                    //barrier.flush.layout = vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
                    //barrier.flush.layout = determine_image_layouts_result.image_layouts[&version_id].write_layout.into();
                    barrier.flush.access_flags |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
                    barrier.flush.stage_flags |= vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
                }
            }

            // barriers.push(RenderGraphNodeImageBarriers {
            //     invalidates: invalidate_barriers,
            //     flushes: flush_barriers
            // });
            barriers.push(RenderGraphNodeImageBarriers {
                barriers: node_barriers,
            })
        }

        barriers
    }

    // * At this point we know format, samples, load_op, stencil_load_op, and initial_layout. We also
    //   know what needs to be flushed/invalidated
    // * We want to determine store_op, stencil_store_op, final_layout. And the validates/flushes
    //   we actually need to insert
    fn build_pass_barriers(
        &self,
        node_execution_order: &[RenderGraphNodeId],
        image_constraints: &DetermineImageConstraintsResult,
        physical_images: &AssignPhysicalImagesResult,
        node_barriers: &[RenderGraphNodeImageBarriers],
        passes: &mut [RenderGraphPass],
    ) -> Vec<Vec<dsc::SubpassDependency>> {
        log::info!("-- build_pass_barriers --");
        const MAX_PIPELINE_FLAG_BITS: usize = 15;
        let ALL_GRAPHICS: vk::PipelineStageFlags =
            vk::PipelineStageFlags::from_raw(0b111_1111_1110);

        struct ImageState {
            layout: vk::ImageLayout,
            pending_flush_access_flags: vk::AccessFlags,
            pending_flush_pipeline_stage_flags: vk::PipelineStageFlags,
            // One per pipeline stage
            invalidated: [vk::AccessFlags; MAX_PIPELINE_FLAG_BITS],
        }

        impl Default for ImageState {
            fn default() -> Self {
                ImageState {
                    layout: vk::ImageLayout::UNDEFINED,
                    pending_flush_access_flags: Default::default(),
                    pending_flush_pipeline_stage_flags: Default::default(),
                    invalidated: Default::default(),
                }
            }
        }

        // to support subpass, probably need image states for each previous subpass

        let mut image_states: Vec<ImageState> =
            Vec::with_capacity(physical_images.physical_image_infos.len());
        image_states.resize_with(physical_images.physical_image_infos.len(), || {
            Default::default()
        });

        let mut pass_dependencies = Vec::default();

        for (pass_index, pass) in passes.iter_mut().enumerate() {
            log::info!("pass {}", pass_index);
            let mut subpass_dependencies = Vec::default();
            let mut attachment_initial_layout: Vec<Option<dsc::ImageLayout>> = Default::default();
            attachment_initial_layout.resize_with(pass.attachments.len(), || None);

            //TODO: This does not support multipass
            assert_eq!(pass.subpasses.len(), 1);
            for (subpass_index, subpass) in pass.subpasses.iter_mut().enumerate() {
                log::info!("  subpass {}", subpass_index);
                let node = self.node(subpass.node);
                let node_barriers = &node_barriers[subpass.node.0];

                // Accumulate the invalidates here
                let mut invalidate_src_access_flags = vk::AccessFlags::empty();
                let mut invalidate_src_pipeline_stage_flags = vk::PipelineStageFlags::empty();
                let mut invalidate_dst_access_flags = vk::AccessFlags::empty();
                let mut invalidate_dst_pipeline_stage_flags = vk::PipelineStageFlags::empty();

                // Look at all the images we read and determine what invalidates we need
                for (physical_image_id, image_barrier) in &node_barriers.barriers {
                    log::info!("    image {:?}", physical_image_id);
                    let image_state = &mut image_states[physical_image_id.0];

                    // Include the previous writer's stage/access flags, if there were any
                    invalidate_src_access_flags |= image_state.pending_flush_access_flags;
                    invalidate_src_pipeline_stage_flags |=
                        image_state.pending_flush_pipeline_stage_flags;

                    // layout changes are write operations and can cause hazards. We need to
                    // block on any stages before that are reading or writing
                    let layout_change = image_state.layout != image_barrier.layout;
                    if layout_change {
                        log::info!(
                            "      layout change! {:?} -> {:?}",
                            image_state.layout,
                            image_barrier.layout
                        );
                        for i in 0..MAX_PIPELINE_FLAG_BITS {
                            if image_state.invalidated[i] != vk::AccessFlags::empty() {
                                // Add an execution barrier if we are transitioning the layout
                                // of something that is already being read from
                                let pipeline_stage = vk::PipelineStageFlags::from_raw(1 << i);
                                log::info!(
                                    "        add src execution barrier on stage {:?}",
                                    pipeline_stage
                                );
                                invalidate_src_pipeline_stage_flags |= pipeline_stage;
                                //invalidate_dst_pipeline_stage_flags |= image_barrier.invalidate.stage_flags;
                                //invalidate_dst_pipeline_stage_flags |= image_barrier.flush.stage_flags;
                            }

                            image_state.invalidated[i] = vk::AccessFlags::empty();
                        }

                        // And clear invalidation flag to require the image to be loaded
                        log::info!(
                            "        cleared all invalidated bits for image {:?}",
                            physical_image_id
                        );
                    }

                    // Requirements for this image
                    let mut image_invalidate_access_flags = image_barrier.invalidate.access_flags;
                    let mut image_invalidate_pipeline_stage_flags =
                        image_barrier.invalidate.stage_flags;

                    //TODO: Should I OR in the flush access/stages? Right now the invalidate barrier is including write
                    // access flags in the invalidate **but only for modifies**
                    image_invalidate_access_flags |= image_barrier.flush.access_flags;
                    image_invalidate_pipeline_stage_flags |= image_barrier.flush.stage_flags;

                    // Check if we have already done invalidates for this image previously, allowing
                    // us to skip some now
                    for i in 0..MAX_PIPELINE_FLAG_BITS {
                        let pipeline_stage = vk::PipelineStageFlags::from_raw(1 << i);
                        if pipeline_stage.intersects(image_invalidate_pipeline_stage_flags) {
                            // If the resource has been invalidate in this stage, we don't need to include this stage
                            // in the invalidation barrier
                            if image_state.invalidated[i].contains(image_invalidate_access_flags) {
                                log::info!(
                                    "      skipping invalidation for {:?} {:?}",
                                    pipeline_stage,
                                    image_invalidate_access_flags
                                );
                                image_invalidate_pipeline_stage_flags &= !pipeline_stage;
                            }
                        }
                    }

                    // All pipeline stages have seen invalidates for the relevant access flags
                    // already, so we don't need to do invalidates at all.
                    if image_invalidate_pipeline_stage_flags == vk::PipelineStageFlags::empty() {
                        log::info!("      no invalidation required, clearing access flags");
                        image_invalidate_access_flags = vk::AccessFlags::empty();
                    }

                    log::info!("      Access Flags: {:?}", image_invalidate_access_flags);
                    log::info!(
                        "      Pipeline Stage Flags: {:?}",
                        image_invalidate_pipeline_stage_flags
                    );

                    // OR the requirements in
                    invalidate_dst_access_flags |= image_invalidate_access_flags;
                    invalidate_dst_pipeline_stage_flags |= image_invalidate_pipeline_stage_flags;

                    // Set the initial layout for the attachment, but only if it's the first time we've seen it
                    //TODO: This is bad and does not properly handle an image being used in multiple ways requiring
                    // multiple layouts
                    for (attachment_index, attachment) in
                        &mut pass.attachments.iter_mut().enumerate()
                    {
                        //log::info!("      attachment {:?}", attachment.image);
                        if attachment.image == *physical_image_id {
                            if attachment_initial_layout[attachment_index].is_none() {
                                //log::info!("        initial layout {:?}", image_barrier.layout);
                                attachment_initial_layout[attachment_index] =
                                    Some(image_state.layout.into());
                                attachment.initial_layout = image_state.layout.into();
                            }

                            attachment.final_layout = image_barrier.layout.into();
                            break;
                        }
                    }

                    image_state.layout = image_barrier.layout;
                }
                //
                // for (physical_image_id, image_barrier) in &node_barriers.flushes {
                //     log::info!("    flush");
                //     let image_state = &mut image_states[physical_image_id.0];
                //
                //     for i in 0..MAX_PIPELINE_FLAG_BITS {
                //         if image_state.invalidated[i] != vk::AccessFlags::empty() {
                //             // Add an execution barrier if we are writing on something that
                //             // is already being read from
                //             let pipeline_stage = vk::PipelineStageFlags::from_raw(1 << i);
                //             invalidate_src_pipeline_stage_flags |= pipeline_stage;
                //             invalidate_dst_pipeline_stage_flags |= image_barrier.stage_flags;
                //         }
                //     }
                //
                //     for (attachment_index, attachment) in &mut pass.attachments.iter_mut().enumerate() {
                //         log::info!("      attachment {:?}", attachment.image);
                //         if attachment.image == *physical_image_id {
                //             log::info!("        final layout {:?}", image_barrier.layout);
                //             attachment.final_layout = image_barrier.layout.into();
                //             break;
                //         }
                //     }
                //
                //     assert!(image_state.layout == vk::ImageLayout::UNDEFINED || image_state.layout == image_barrier.layout);
                //     if image_state.layout != image_barrier.layout {
                //         invalidate_dst_pipeline_stage_flags |= image_barrier.stage_flags;
                //         invalidate_dst_access_flags |= image_barrier.access_flags;
                //     }
                //
                //     //image_state.layout = image_barrier.layout;
                // }

                // Update the image states
                for image_state in &mut image_states {
                    // Mark pending flushes as handled
                    //TODO: Check that ! is inverting bits
                    image_state.pending_flush_access_flags &= !invalidate_src_access_flags;
                    image_state.pending_flush_pipeline_stage_flags &=
                        !invalidate_src_pipeline_stage_flags;

                    // Mark resources that we are invalidating as having been invalidated for
                    // the appropriate pipeline stages
                    //TODO: Invalidate all later stages?
                    for i in 0..MAX_PIPELINE_FLAG_BITS {
                        let pipeline_stage = vk::PipelineStageFlags::from_raw(1 << i);
                        if pipeline_stage.intersects(invalidate_dst_pipeline_stage_flags) {
                            image_state.invalidated[i] |= invalidate_dst_access_flags;
                        }
                    }
                }

                // The first pass has no previous readers/writers and the spec requires that
                // pipeline stage flags is not 0
                if invalidate_src_pipeline_stage_flags.is_empty() {
                    invalidate_src_pipeline_stage_flags |= vk::PipelineStageFlags::TOP_OF_PIPE
                }

                // Build a subpass dependency (EXTERNAL -> 0)
                let invalidate_subpass_dependency = dsc::SubpassDependency {
                    dependency_flags: dsc::DependencyFlags::Empty,
                    src_access_mask: dsc::AccessFlags::from_access_flag_mask(
                        invalidate_src_access_flags,
                    ),
                    src_stage_mask: dsc::PipelineStageFlags::from_bits(
                        invalidate_src_pipeline_stage_flags.as_raw(),
                    )
                    .unwrap(),
                    dst_access_mask: dsc::AccessFlags::from_access_flag_mask(
                        invalidate_dst_access_flags,
                    ),
                    dst_stage_mask: dsc::PipelineStageFlags::from_bits(
                        invalidate_dst_pipeline_stage_flags.as_raw(),
                    )
                    .unwrap(),
                    src_subpass: dsc::SubpassDependencyIndex::External,
                    dst_subpass: dsc::SubpassDependencyIndex::Index(0),
                };
                subpass_dependencies.push(invalidate_subpass_dependency);

                for (physical_image_id, image_barrier) in &node_barriers.barriers {
                    let image_state = &mut image_states[physical_image_id.0];

                    // Queue up flushes to happen later
                    image_state.pending_flush_pipeline_stage_flags |=
                        image_barrier.flush.stage_flags;
                    image_state.pending_flush_access_flags |= image_barrier.flush.access_flags;

                    // If we write something, mark it as no longer invalidated
                    //TODO: Not sure if we invalidate specific stages or all stages
                    //TODO: Can we invalidate specific access instead of all access?
                    for i in 0..MAX_PIPELINE_FLAG_BITS {
                        image_state.invalidated[i] = vk::AccessFlags::empty();
                    }

                    // let layout_change = image_state.layout != image_barrier.layout;
                    // if layout_change {
                    //     for i in 0..MAX_PIPELINE_FLAG_BITS {
                    //         if image_state.invalidated[i] != vk::AccessFlags::empty() {
                    //             // Add an execution barrier if we are transitioning the layout
                    //             // of something that is already being read from
                    //             let pipeline_stage = vk::PipelineStageFlags::from_raw(1 << i);
                    //             invalidate_src_pipeline_stage_flags |= pipeline_stage;
                    //             invalidate_dst_pipeline_stage_flags |= image_barrier.stage_flags;
                    //         }
                    //     }
                    // }
                }

                // This hack clears final layout for attachments with DONT_CARE store_op. This is happening
                // for inserted resolves because the color attachment still has a write usage (and must have it
                // to put the attachment on the renderpass) but this is also creating a flush for the image which
                // means it gets placed into a layout
                // EDIT: Vulkan spec requires final layout not be UNDEFINED
                // for (attachment_index, attachment) in &mut pass.attachments.iter_mut().enumerate() {
                //     if attachment.store_op == vk::AttachmentStoreOp::DONT_CARE
                //         && attachment.stencil_store_op == vk::AttachmentStoreOp::DONT_CARE
                //     {
                //         attachment.final_layout = dsc::ImageLayout::Undefined;
                //     }
                // }

                // TODO: Figure out how to handle output images
                // TODO: This only works if no one else reads it?
                println!("Check for output images");
                for (output_image_index, output_image) in self.output_images.iter().enumerate() {
                    if self.image_version_info(output_image.usage).creator_node == subpass.node {
                        //output_image.
                        //self.image_usages[output_image.usage]

                        let output_physical_image =
                            physical_images.map_image_to_physical[&output_image.usage];
                        println!(
                            "Output image {} usage {:?} created by node {:?} physical image {:?}",
                            output_image_index,
                            output_image.usage,
                            subpass.node,
                            output_physical_image
                        );

                        for (attachment_index, attachment) in
                            &mut pass.attachments.iter_mut().enumerate()
                        {
                            if attachment.image == output_physical_image {
                                println!("  attachment {}", attachment_index);
                                attachment.final_layout = output_image.final_layout;
                            }
                        }

                        //TODO: Need a 0 -> EXTERNAL dependency here
                    }
                }

                //TODO: Need to do a dependency? Maybe by adding a flush?
            }

            pass_dependencies.push(subpass_dependencies);
        }

        pass_dependencies
    }

    fn create_renderpass_descriptions(
        mut passes: Vec<RenderGraphPass>,
        node_barriers: Vec<RenderGraphNodeImageBarriers>,
        subpass_dependencies: &Vec<Vec<dsc::SubpassDependency>>,
        physical_images: &AssignPhysicalImagesResult,
        swapchain_info: &SwapchainSurfaceInfo,
    ) -> Vec<RenderGraphOutputPass> {
        let mut renderpasses = Vec::with_capacity(passes.len());
        for (index, pass) in passes.iter().enumerate() {
            let mut renderpass_desc = dsc::RenderPass::default();
            let mut subpass_nodes = Vec::with_capacity(pass.subpasses.len());

            renderpass_desc.attachments.reserve(pass.attachments.len());
            for attachment in &pass.attachments {
                renderpass_desc
                    .attachments
                    .push(attachment.into_pass_desc());
            }

            renderpass_desc.attachments.reserve(pass.subpasses.len());
            for subpass in &pass.subpasses {
                let mut subpass_description = dsc::SubpassDescription {
                    pipeline_bind_point: dsc::PipelineBindPoint::Graphics,
                    input_attachments: Default::default(),
                    color_attachments: Default::default(),
                    resolve_attachments: Default::default(),
                    depth_stencil_attachment: Default::default(),
                };

                fn set_attachment_reference(
                    attachment_references_list: &mut Vec<dsc::AttachmentReference>,
                    list_index: usize,
                    attachment_reference: dsc::AttachmentReference,
                ) {
                    // Pad unused to get to the specified color attachment list_index
                    while attachment_references_list.len() <= list_index {
                        attachment_references_list.push(dsc::AttachmentReference {
                            attachment: dsc::AttachmentIndex::Unused,
                            layout: dsc::ImageLayout::Undefined,
                        })
                    }

                    attachment_references_list[list_index] = attachment_reference;
                }

                for (color_index, attachment_index) in subpass.color_attachments.iter().enumerate()
                {
                    if let Some(attachment_index) = attachment_index {
                        let physical_image = pass.attachments[*attachment_index].image;
                        set_attachment_reference(
                            &mut subpass_description.color_attachments,
                            color_index,
                            dsc::AttachmentReference {
                                attachment: dsc::AttachmentIndex::Index(*attachment_index as u32),
                                layout: node_barriers[subpass.node.0].barriers[&physical_image]
                                    .layout
                                    .into(),
                            },
                        );
                    }
                }

                for (resolve_index, attachment_index) in
                    subpass.resolve_attachments.iter().enumerate()
                {
                    if let Some(attachment_index) = attachment_index {
                        let physical_image = pass.attachments[*attachment_index].image;
                        set_attachment_reference(
                            &mut subpass_description.resolve_attachments,
                            resolve_index,
                            dsc::AttachmentReference {
                                attachment: dsc::AttachmentIndex::Index(*attachment_index as u32),
                                layout: node_barriers[subpass.node.0].barriers[&physical_image]
                                    .layout
                                    .into(),
                            },
                        );
                    }
                }

                if let Some(attachment_index) = subpass.depth_attachment {
                    let physical_image = pass.attachments[attachment_index].image;
                    subpass_description.depth_stencil_attachment = Some(dsc::AttachmentReference {
                        attachment: dsc::AttachmentIndex::Index(attachment_index as u32),
                        layout: node_barriers[subpass.node.0].barriers[&physical_image]
                            .layout
                            .into(),
                    });
                }

                renderpass_desc.subpasses.push(subpass_description);
                subpass_nodes.push(subpass.node);
            }

            let dependencies = &subpass_dependencies[index];
            renderpass_desc.dependencies.reserve(dependencies.len());
            for dependency in dependencies {
                // renderpass_desc.dependencies.push(dsc::SubpassDependency {
                //     src_subpass: dependency.src_subpass,
                //     dst_subpass: dependency.dst_subpass,
                //     src_stage_mask: dependency.clone().src_stage_mask,
                //     dst_stage_mask: dependency.clone().dst_stage_mask,
                //     src_access_mask: dependency.clone().src_access_mask,
                //     dst_access_mask: dependency.clone().dst_access_mask,
                //     dependency_flags: dependency.clone().dependency_flags,
                // })
                renderpass_desc.dependencies.push(dependency.clone());
            }

            let attachment_images = pass
                .attachments
                .iter()
                .map(|attachment| attachment.image)
                .collect();
            let clear_values = pass
                .attachments
                .iter()
                .map(|attachment| match &attachment.clear_color {
                    Some(clear_color) => clear_color.clone().into(),
                    None => vk::ClearValue::default(),
                })
                .collect();

            let output_pass = RenderGraphOutputPass {
                subpass_nodes,
                description: renderpass_desc,
                extents: swapchain_info.extents,
                attachment_images,
                clear_values,
            };

            renderpasses.push(output_pass);
        }

        renderpasses
    }

    fn print_physical_image_usage(
        &mut self,
        assign_physical_images_result: &AssignPhysicalImagesResult, /*, determine_image_layouts_result: &DetermineImageLayoutsResult*/
    ) {
        log::info!("Physical image usage:");
        for (physical_image_id, physical_image_info) in
            &assign_physical_images_result.physical_image_infos
        {
            log::info!("  image: {:?}", physical_image_id);
            for version_id in &physical_image_info.versions {
                log::info!("  version_id {:?}", version_id);
                let version =
                    &mut self.image_resources[version_id.index].versions[version_id.version];
                log::info!("  create: {:?}", version.create_usage);
                //log::info!("  create: {:?} {:?}", version.create_usage, self.image_usages[version.create_usage.0].preferred_layout);
                //log::info!("    create: {:?} {:?}", version.create_usage, determine_image_layouts_result.image_layouts[version_id].write_layout);
                for read in &version.read_usages {
                    log::info!("    read: {:?}", read);
                    //log::info!("    read: {:?} {:?}", read, self.image_usages[read.0].preferred_layout);
                    //log::info!("      read: {:?} {:?}", read, determine_image_layouts_result.image_layouts[version_id].read_layout);
                }
            }
        }
    }

    fn print_image_constraints(
        &self,
        image_constraint_results: &mut DetermineImageConstraintsResult,
    ) {
        log::info!("Image constraints:");
        for (image_index, image_resource) in self.image_resources.iter().enumerate() {
            log::info!("  Image {:?} {:?}", image_index, image_resource.name);
            for (version_index, version) in image_resource.versions.iter().enumerate() {
                log::info!("    Version {}", version_index);

                log::info!(
                    "      Writen as: {:?}",
                    image_constraint_results.specification(version.create_usage)
                );

                for (usage_index, usage) in version.read_usages.iter().enumerate() {
                    log::info!(
                        "      Read Usage {}: {:?}",
                        usage_index,
                        image_constraint_results.specification(*usage)
                    );
                }
            }
        }
    }

    fn print_image_compatibility(
        &self,
        image_constraint_results: &DetermineImageConstraintsResult,
    ) {
        log::info!("Image Compatibility Report:");
        for (image_index, image_resource) in self.image_resources.iter().enumerate() {
            log::info!("  Image {:?} {:?}", image_index, image_resource.name);
            for (version_index, version) in image_resource.versions.iter().enumerate() {
                let write_specification =
                    image_constraint_results.specification(version.create_usage);

                log::info!("    Version {}: {:?}", version_index, version);
                for (usage_index, usage) in version.read_usages.iter().enumerate() {
                    let read_specification = image_constraint_results.specification(*usage);

                    // TODO: Skip images we don't use?

                    if write_specification == read_specification {
                        log::info!("      read usage {} matches", usage_index);
                    } else {
                        log::info!("      read usage {} does not match", usage_index);
                        log::info!("        produced: {:?}", write_specification);
                        log::info!("        required: {:?}", read_specification);
                    }
                }
            }
        }
    }

    pub fn into_plan(
        mut self,
        swapchain_info: &SwapchainSurfaceInfo,
    ) -> RenderGraphPlan {
        //
        // Walk backwards through the DAG, starting from the output images, through all the upstream
        // dependencies of those images. We are doing a depth first search. Nodes that make no
        // direct or indirect contribution to an output image will not be included. As an
        // an implementation detail, we try to put renderpass merge candidates adjacent to each
        // other in this list
        //
        let node_execution_order = self.determine_node_order();

        // Print out the execution order
        log::info!("Execution order of unculled nodes:");
        for node in &node_execution_order {
            log::info!("  Node {:?} {:?}", node, self.node(*node).name());
        }

        //
        // Traverse the graph to determine specifications for all images that will be used. This
        // iterates forwards and backwards through the node graph. This allows us to specify
        // attributes about images (like format, sample count) in key areas and infer it elsewhere.
        // If there is not enough information to infer then the render graph cannot be used.
        //
        let mut image_constraint_results = self.determine_image_constraints(&node_execution_order);

        // Print out the constraints assigned to images
        self.print_image_constraints(&mut image_constraint_results);

        //
        // Add resolves to the graph - this will occur when a renderpass outputs a multisample image
        // to a renderpass that is expecting a non-multisampled image.
        //
        self.insert_resolves(&node_execution_order, &mut image_constraint_results);
        self.print_image_constraints(&mut image_constraint_results);

        // Print the cases where we can't reuse images
        self.print_image_compatibility(&image_constraint_results);

        //
        // Assign logical images to physical images. This should give us a minimal number of images.
        // This does not include aliasing images during graph execution. We handle this later.
        //
        let assign_physical_images_result =
            self.assign_physical_images(&node_execution_order, &mut image_constraint_results);
        log::info!("Physical image usage:");
        for (physical_image_id, logical_image_id_list) in
            &assign_physical_images_result.physical_image_infos
        {
            log::info!("  Physical image: {:?}", physical_image_id);
            for logical_image in &logical_image_id_list.usages {
                log::info!("    {:?}", logical_image);
            }
        }

        //let determine_image_layouts_result = self.determine_image_layouts(&node_execution_order, &image_constraint_results, &assign_physical_images_result);

        // Print the physical images
        self.print_physical_image_usage(
            &assign_physical_images_result, /*, &determine_image_layouts_result*/
        );

        //
        // Combine nodes into passes where possible
        //
        let mut passes = self.build_physical_passes(
            &node_execution_order,
            &image_constraint_results,
            &assign_physical_images_result, /*, &determine_image_layouts_result*/
        );
        log::info!("Merged Renderpasses:");
        for (index, pass) in passes.iter().enumerate() {
            log::info!("  pass {}", index);
            log::info!("    attachments:");
            for attachment in &pass.attachments {
                log::info!("      {:?}", attachment);
            }
            log::info!("    subpasses:");
            for subpass in &pass.subpasses {
                log::info!("      {:?}", subpass);
            }
        }

        let node_barriers = self.build_node_barriers(
            &node_execution_order,
            &image_constraint_results,
            &assign_physical_images_result, /*, &determine_image_layouts_result*/
        );
        log::info!("Barriers:");
        for (index, pass) in node_barriers.iter().enumerate() {
            log::info!("  pass {}", index);
            log::info!("    invalidates");
            for (physical_id, barriers) in &pass.barriers {
                log::info!("      {:?}: {:?}", physical_id, barriers.invalidate);
            }
            log::info!("    flushes");
            for (physical_id, barriers) in &pass.barriers {
                log::info!("      {:?}: {:?}", physical_id, barriers.flush);
            }
        }

        //TODO: Figure out in/out layouts for passes? Maybe insert some other fixes? Drop transient
        // images?

        // Print out subpass
        let subpass_dependencies = self.build_pass_barriers(
            &node_execution_order,
            &image_constraint_results,
            &assign_physical_images_result,
            &node_barriers,
            &mut passes,
        );

        log::info!("Merged Renderpasses:");
        for (index, pass) in passes.iter().enumerate() {
            log::info!("  pass {}", index);
            log::info!("    attachments:");
            for attachment in &pass.attachments {
                log::info!("      {:?}", attachment);
            }
            log::info!("    subpasses:");
            for subpass in &pass.subpasses {
                log::info!("      {:?}", subpass);
            }
            log::info!("    dependencies:");
            for subpass in &subpass_dependencies[index] {
                log::info!("      {:#?}", subpass);
            }
        }

        //TODO: Cull images that only exist within the lifetime of a single pass? (just passed among
        // subpasses)

        let renderpasses = RenderGraph::create_renderpass_descriptions(
            passes,
            node_barriers,
            &subpass_dependencies,
            &assign_physical_images_result,
            swapchain_info,
        );

        let mut output_images: FnvHashMap<PhysicalImageId, RenderGraphPlanOutputImage> =
            Default::default();
        for output_image in &self.output_images {
            let output_image_physical_id =
                assign_physical_images_result.map_image_to_physical[&output_image.usage];

            // println!(
            //     "Output: {:?} {:?}",
            //     output_image_physical_id, output_image.output_image_id
            // );
            output_images.insert(
                output_image_physical_id,
                RenderGraphPlanOutputImage {
                    output_id: output_image.output_image_id,
                    dst_image: output_image.dst_image.clone(),
                },
            );
        }

        let mut intermediate_images: FnvHashMap<PhysicalImageId, RenderGraphImageSpecification> =
            Default::default();
        for (physical_image, physical_image_info) in
            &assign_physical_images_result.physical_image_infos
        {
            if output_images.contains_key(&physical_image) {
                continue;
            }

            // println!(
            //     "Intermediate: {:?} {:?}",
            //     physical_image, physical_image_info.specification
            // );

            intermediate_images.insert(*physical_image, physical_image_info.specification.clone());
        }

        // Allocation of images
        // clear values
        // frame buffers
        // - size
        // - images
        // - renderpass
        // let required_images = assign_physical_images_result.physical_image_infos.iter().map(|(id, info)| info.specification);
        // for (physical_image, physical_image_info) in assign_physical_images_result.physical_image_infos {
        //     physical_image_info.specification
        // }

        for (renderpass_index, renderpass) in renderpasses.iter().enumerate() {
            println!("-- RENDERPASS {} --", renderpass_index);
            println!("{:#?}", renderpass);
        }

        println!("-- IMAGES --");
        for (physical_id, output_image) in &output_images {
            println!("Output Image: {:?} {:?}", physical_id, output_image);
        }
        for (physical_id, intermediate_image_spec) in &intermediate_images {
            println!(
                "Output Image: {:?} {:?}",
                physical_id, intermediate_image_spec
            );
        }

        RenderGraphPlan {
            passes: renderpasses,
            output_images,
            intermediate_images,
        }
    }
}
