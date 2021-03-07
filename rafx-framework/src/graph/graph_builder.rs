use super::*;
use crate::nodes::{RenderPhase, RenderPhaseIndex};
use crate::resources::{ImageViewResource, ResourceArc};
use crate::BufferResource;
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::{
    RafxColorClearValue, RafxDepthStencilClearValue, RafxResourceState, RafxResourceType,
    RafxResult,
};

#[derive(Copy, Clone)]
pub enum RenderGraphQueue {
    DefaultGraphics,
    Index(u32),
}

// /// An image that is being provided to the render graph that can be read from
// #[derive(Debug)]
// pub struct RenderGraphInputImage {
//     pub usage: RenderGraphImageUsageId,
//     pub specification: RenderGraphImageSpecification,
// }

/// An image that is being provided to the render graph that can be written to
#[derive(Debug)]
pub struct RenderGraphOutputImage {
    pub output_image_id: RenderGraphOutputImageId,
    pub usage: RenderGraphImageUsageId,
    pub specification: RenderGraphImageSpecification,
    pub dst_image: ResourceArc<ImageViewResource>,

    pub(super) final_state: RafxResourceState,
}

// /// A buffer that is being provided to the render graph that can be read from
// #[derive(Debug)]
// pub struct RenderGraphInputBuffer {
//     pub usage: RenderGraphBufferUsageId,
//     pub specification: RenderGraphBufferSpecification,
// }

/// A buffer that is being provided to the render graph that can be written to
#[derive(Debug)]
pub struct RenderGraphOutputBuffer {
    pub output_buffer_id: RenderGraphOutputBufferId,
    pub usage: RenderGraphBufferUsageId,
    pub specification: RenderGraphBufferSpecification,
    pub dst_buffer: ResourceArc<BufferResource>,
}

/// A collection of nodes and resources. Nodes represent an event or process that will occur at
/// a certain time. (For now, they just represent subpasses that may be merged with each other.)
/// Resources represent images and buffers that may be read/written by nodes.
#[derive(Default)]
pub struct RenderGraphBuilder {
    /// Nodes that have been registered in the graph
    pub(super) nodes: Vec<RenderGraphNode>,

    /// Image resources that have been registered in the graph. These resources are "virtual" until
    /// the graph is scheduled. In other words, we don't necessarily allocate an image for every
    /// resource as some resources can share the same image internally if their lifetime don't
    /// overlap. Additionally, a resource can be bound to input and output images. If this is the
    /// case, we will try to use those images rather than creating new ones.
    pub(super) image_resources: Vec<RenderGraphImageResource>,
    pub(super) buffer_resources: Vec<RenderGraphBufferResource>,

    /// All read/write accesses to images. Image writes create new "versions" of the image. So all
    /// image versions have one writer and 0 or more readers. This indirectly defines the order of
    /// execution for the graph.
    pub(super) image_usages: Vec<RenderGraphImageUsage>,
    pub(super) buffer_usages: Vec<RenderGraphBufferUsage>,

    /// Images that are passed into the graph that can be read from
    //pub(super) input_images: Vec<RenderGraphInputImage>,

    /// Images that are passed into the graph to be written to.
    pub(super) output_images: Vec<RenderGraphOutputImage>,
    pub(super) output_buffers: Vec<RenderGraphOutputBuffer>,

    //
    // Callbacks
    //
    pub(super) visit_node_callbacks:
        FnvHashMap<RenderGraphNodeId, RenderGraphNodeVisitNodeCallback>,
    pub(super) render_phase_dependencies:
        FnvHashMap<RenderGraphNodeId, FnvHashSet<RenderPhaseIndex>>,
}

impl RenderGraphBuilder {
    //NOTE: While the image aspect flags may seem redundant with subresource_range here, the
    // subresource_range should indicate the image view's supported aspects and the provided
    // image aspect flags the aspects that are actually being used
    pub(super) fn add_image_usage(
        &mut self,
        user: RenderGraphImageUser,
        version: RenderGraphImageVersionId,
        usage_type: RenderGraphImageUsageType,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        let usage_id = RenderGraphImageUsageId(self.image_usages.len());

        self.image_usages.push(RenderGraphImageUsage {
            user,
            usage_type,
            version,
            view_options,
        });
        usage_id
    }

    // Add an image that can be used by nodes
    pub(super) fn add_image_create(
        &mut self,
        create_node: RenderGraphNodeId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        let version_id = RenderGraphImageVersionId {
            index: self.image_resources.len(),
            version: 0,
        };
        let usage_id = self.add_image_usage(
            RenderGraphImageUser::Node(create_node),
            version_id,
            RenderGraphImageUsageType::Create,
            view_options,
        );

        let mut resource = RenderGraphImageResource::new();

        let version_info = RenderGraphImageResourceVersionInfo::new(create_node, usage_id);
        resource.versions.push(version_info);

        // Add it to the graph
        self.image_resources.push(resource);

        self.nodes[create_node.0]
            .image_creates
            .push(RenderGraphImageCreate {
                //image: image_id,
                image: usage_id,
                constraint,
            });

        usage_id
    }

    pub(super) fn add_image_read(
        &mut self,
        read_node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        let version_id = self.image_usages[image.0].version;

        let usage_id = self.add_image_usage(
            RenderGraphImageUser::Node(read_node),
            version_id,
            RenderGraphImageUsageType::Read,
            view_options,
        );

        self.image_resources[version_id.index].versions[version_id.version]
            .add_read_usage(usage_id);

        self.nodes[read_node.0]
            .image_reads
            .push(RenderGraphImageRead {
                image: usage_id,
                constraint,
            });

        usage_id
    }

    pub(super) fn add_image_modify(
        &mut self,
        modify_node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> (RenderGraphImageUsageId, RenderGraphImageUsageId) {
        let read_version_id = self.image_usages[image.0].version;

        let read_usage_id = self.add_image_usage(
            RenderGraphImageUser::Node(modify_node),
            read_version_id,
            RenderGraphImageUsageType::ModifyRead,
            view_options.clone(),
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
            RenderGraphImageUser::Node(modify_node),
            write_version_id,
            RenderGraphImageUsageType::ModifyWrite,
            view_options,
        );

        let version_info = RenderGraphImageResourceVersionInfo::new(modify_node, write_usage_id);
        self.image_resources[read_version_id.index]
            .versions
            .push(version_info);

        self.nodes[modify_node.0]
            .image_modifies
            .push(RenderGraphImageModify {
                input: read_usage_id,
                output: write_usage_id,
                constraint,
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
        let node_color_attachments = &mut self.nodes[node.0].color_attachments;
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
        let node_depth_attachment = &mut self.nodes[node.0].depth_attachment;
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
        let node_resolve_attachments = &mut self.nodes[node.0].resolve_attachments;
        if node_resolve_attachments.len() <= resolve_attachment_index {
            node_resolve_attachments.resize_with(resolve_attachment_index + 1, || None);
        }

        assert!(node_resolve_attachments[resolve_attachment_index].is_none());
        node_resolve_attachments[resolve_attachment_index] = Some(resolve_attachment);
    }

    pub fn create_unattached_image(
        &mut self,
        create_node: RenderGraphNodeId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        self.add_image_create(create_node, constraint, view_options)
    }

    pub fn create_color_attachment(
        &mut self,
        node: RenderGraphNodeId,
        color_attachment_index: usize,
        clear_color_value: Option<RafxColorClearValue>,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_COLOR;

        // Add the read to the graph
        let create_image = self.add_image_create(node, constraint, view_options);

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
        clear_depth_stencil_value: Option<RafxDepthStencilClearValue>,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_DEPTH_STENCIL;

        // Add the read to the graph
        let create_image = self.add_image_create(node, constraint, view_options);

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
        clear_depth_stencil_value: Option<RafxDepthStencilClearValue>,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_DEPTH_STENCIL;

        // Add the read to the graph
        let create_image = self.add_image_create(node, constraint, view_options);

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
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_COLOR;

        let create_image = self.add_image_create(node, constraint, view_options);

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
        view_options: RenderGraphImageViewOptions,
    ) {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_COLOR;

        // Add the read to the graph
        let read_image = self.add_image_read(node, image, constraint, view_options);

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
        view_options: RenderGraphImageViewOptions,
    ) {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_DEPTH_STENCIL;

        // Add the read to the graph
        let read_image = self.add_image_read(node, image, constraint, view_options);

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
        view_options: RenderGraphImageViewOptions,
    ) {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_DEPTH_STENCIL;

        // Add the read to the graph
        let read_image = self.add_image_read(node, image, constraint, view_options);

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
        clear_color_value: Option<RafxColorClearValue>,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_COLOR;

        // Add the read to the graph
        let (read_image, write_image) =
            self.add_image_modify(node, image, constraint, view_options);

        self.set_color_attachment(
            node,
            color_attachment_index,
            RenderGraphPassColorAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Modify,
                clear_color_value,
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
        clear_depth_stencil_value: Option<RafxDepthStencilClearValue>,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_DEPTH_STENCIL;

        // Add the read to the graph
        let (read_image, write_image) =
            self.add_image_modify(node, image, constraint, view_options);

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Modify,
                clear_depth_stencil_value,
                read_image: Some(read_image),
                write_image: Some(write_image),
                has_depth: true,
                has_stencil: false,
            },
        );

        write_image
    }

    pub fn modify_depth_stencil_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        clear_depth_stencil_value: Option<RafxDepthStencilClearValue>,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::RENDER_TARGET_DEPTH_STENCIL;

        // Add the read to the graph
        let (read_image, write_image) =
            self.add_image_modify(node, image, constraint, view_options);

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Modify,
                clear_depth_stencil_value,
                read_image: Some(read_image),
                write_image: Some(write_image),
                has_depth: true,
                has_stencil: true,
            },
        );

        write_image
    }

    pub fn sample_image(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::TEXTURE;

        // Add the read to the graph
        let usage = self.add_image_read(node, image, constraint, view_options);

        self.node_mut(node).sampled_images.push(usage);
        usage
    }

    pub fn set_output_image(
        &mut self,
        image_id: RenderGraphImageUsageId,
        dst_image: ResourceArc<ImageViewResource>,
        specification: RenderGraphImageSpecification,
        view_options: RenderGraphImageViewOptions,
        final_state: RafxResourceState,
    ) -> RenderGraphOutputImageId {
        let output_image_id = RenderGraphOutputImageId(self.output_images.len());

        let version_id = self.image_version_id(image_id);
        let usage_id = self.add_image_usage(
            RenderGraphImageUser::Output(output_image_id),
            version_id,
            RenderGraphImageUsageType::Output,
            view_options,
        );

        let image_version = self.image_version_info_mut(image_id);
        image_version.read_usages.push(usage_id);

        let output_image = RenderGraphOutputImage {
            output_image_id,
            usage: usage_id,
            specification,
            dst_image,
            final_state,
        };

        self.output_images.push(output_image);
        output_image_id
    }

    //NOTE: While the buffer aspect flags may seem redundant with subresource_range here, the
    // subresource_range should indicate the buffer view's supported aspects and the provided
    // buffer aspect flags the aspects that are actually being used
    pub(super) fn add_buffer_usage(
        &mut self,
        user: RenderGraphBufferUser,
        version: RenderGraphBufferVersionId,
        usage_type: RenderGraphBufferUsageType,
    ) -> RenderGraphBufferUsageId {
        let usage_id = RenderGraphBufferUsageId(self.buffer_usages.len());
        self.buffer_usages.push(RenderGraphBufferUsage {
            user,
            usage_type,
            version,
        });
        usage_id
    }

    // Add a buffer that can be used by nodes
    pub(super) fn add_buffer_create(
        &mut self,
        create_node: RenderGraphNodeId,
        constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        let version_id = RenderGraphBufferVersionId {
            index: self.buffer_resources.len(),
            version: 0,
        };
        let usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Node(create_node),
            version_id,
            RenderGraphBufferUsageType::Create,
        );

        let mut resource = RenderGraphBufferResource::new();

        let version_info = RenderGraphBufferResourceVersionInfo::new(create_node, usage_id);
        resource.versions.push(version_info);

        // Add it to the graph
        self.buffer_resources.push(resource);

        self.nodes[create_node.0]
            .buffer_creates
            .push(RenderGraphBufferCreate {
                buffer: usage_id,
                constraint,
            });

        usage_id
    }

    pub(super) fn add_buffer_read(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        let version_id = self.buffer_usage(buffer).version;

        let usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Node(read_node),
            version_id,
            RenderGraphBufferUsageType::Read,
        );

        self.buffer_resources[version_id.index].versions[version_id.version]
            .add_read_usage(usage_id);

        self.nodes[read_node.0]
            .buffer_reads
            .push(RenderGraphBufferRead {
                buffer: usage_id,
                constraint,
            });

        usage_id
    }

    pub(super) fn add_buffer_modify(
        &mut self,
        modify_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        constraint: RenderGraphBufferConstraint,
    ) -> (RenderGraphBufferUsageId, RenderGraphBufferUsageId) {
        let read_version_id = self.buffer_usage(buffer).version;

        let read_usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Node(modify_node),
            read_version_id,
            RenderGraphBufferUsageType::ModifyRead,
        );

        self.buffer_resources[read_version_id.index].versions[read_version_id.version]
            .add_read_usage(read_usage_id);

        // Create a new version and add it to the buffer
        let version = self.buffer_resources[read_version_id.index].versions.len();
        let write_version_id = RenderGraphBufferVersionId {
            index: read_version_id.index,
            version,
        };
        let write_usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Node(modify_node),
            write_version_id,
            RenderGraphBufferUsageType::ModifyWrite,
        );

        let version_info = RenderGraphBufferResourceVersionInfo::new(modify_node, write_usage_id);
        self.buffer_resources[read_version_id.index]
            .versions
            .push(version_info);

        self.nodes[modify_node.0]
            .buffer_modifies
            .push(RenderGraphBufferModify {
                input: read_usage_id,
                output: write_usage_id,
                constraint,
            });

        (read_usage_id, write_usage_id)
    }

    pub fn create_buffer(
        &mut self,
        create_node: RenderGraphNodeId,
        constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        self.add_buffer_create(create_node, constraint)
    }

    pub fn read_vertex_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::VERTEX_BUFFER;

        self.add_buffer_read(read_node, buffer, constraint)
    }

    pub fn read_index_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::INDEX_BUFFER;

        self.add_buffer_read(read_node, buffer, constraint)
    }

    pub fn read_indirect_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::INDIRECT_BUFFER;

        self.add_buffer_read(read_node, buffer, constraint)
    }

    pub fn read_uniform_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::UNIFORM_BUFFER;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        self.add_buffer_read(read_node, buffer, constraint)
    }

    pub fn create_storage_buffer(
        &mut self,
        create_node: RenderGraphNodeId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::BUFFER_READ_WRITE;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        self.add_buffer_create(create_node, constraint)
    }

    pub fn read_storage_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::BUFFER_READ_WRITE;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        self.add_buffer_read(read_node, buffer, constraint)
    }

    pub fn modify_storage_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::BUFFER_READ_WRITE;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        let (_read_buffer, write_buffer) = self.add_buffer_modify(read_node, buffer, constraint);

        write_buffer
    }

    pub fn set_output_buffer(
        &mut self,
        buffer_id: RenderGraphBufferUsageId,
        dst_buffer: ResourceArc<BufferResource>,
        specification: RenderGraphBufferSpecification,
    ) -> RenderGraphOutputBufferId {
        if specification.resource_type == RafxResourceType::UNDEFINED {
            panic!("An output buffer with empty resource_type in the specification is almost certainly a mistake.");
        }

        let output_buffer_id = RenderGraphOutputBufferId(self.output_buffers.len());

        let version_id = self.buffer_version_id(buffer_id);
        let usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Output(output_buffer_id),
            version_id,
            RenderGraphBufferUsageType::Output,
        );

        let buffer_version = self.buffer_version_info_mut(buffer_id);
        buffer_version.read_usages.push(usage_id);

        let output_buffer = RenderGraphOutputBuffer {
            output_buffer_id,
            usage: usage_id,
            specification,
            dst_buffer,
        };

        self.output_buffers.push(output_buffer);
        output_buffer_id
    }

    // Add a node which can use resources
    pub fn add_node(
        &mut self,
        name: RenderGraphNodeName,
        queue: RenderGraphQueue,
    ) -> RenderGraphNodeId {
        let node = RenderGraphNodeId(self.nodes.len());
        self.nodes
            .push(RenderGraphNode::new(node, Some(name), queue));
        node
    }

    pub fn add_node_unnamed(
        &mut self,
        queue: RenderGraphQueue,
    ) -> RenderGraphNodeId {
        let node = RenderGraphNodeId(self.nodes.len());
        self.nodes.push(RenderGraphNode::new(node, None, queue));
        node
    }

    pub fn set_node_name(
        &mut self,
        node_id: RenderGraphNodeId,
        name: RenderGraphNodeName,
    ) {
        self.node_mut(node_id).name = Some(name);
    }

    pub fn set_image_name(
        &mut self,
        image_id: RenderGraphImageUsageId,
        name: RenderGraphResourceName,
    ) {
        self.image_resource_mut(image_id).name = Some(name);
    }

    pub fn set_buffer_name(
        &mut self,
        buffer_id: RenderGraphBufferUsageId,
        name: RenderGraphResourceName,
    ) {
        self.buffer_resource_mut(buffer_id).name = Some(name);
    }

    //
    // Callbacks
    //

    /// Adds a callback that receives the renderpass associated with the node
    pub fn set_renderpass_callback<CallbackFnT>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: CallbackFnT,
    ) where
        CallbackFnT: Fn(VisitRenderpassNodeArgs) -> RafxResult<()> + 'static + Send,
    {
        let old = self.visit_node_callbacks.insert(
            node_id,
            RenderGraphNodeVisitNodeCallback::Renderpass(Box::new(f)),
        );
        // If this trips, multiple callbacks were set on the node
        assert!(old.is_none());
    }

    /// Adds a callback for compute based nodes
    pub fn set_compute_callback<CallbackFnT>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: CallbackFnT,
    ) where
        CallbackFnT: Fn(VisitComputeNodeArgs) -> RafxResult<()> + 'static + Send,
    {
        let old = self.visit_node_callbacks.insert(
            node_id,
            RenderGraphNodeVisitNodeCallback::Compute(Box::new(f)),
        );
        // If this trips, multiple callbacks were set on the node
        assert!(old.is_none());
    }

    pub fn add_render_phase_dependency<PhaseT: RenderPhase>(
        &mut self,
        node_id: RenderGraphNodeId,
    ) {
        self.render_phase_dependencies
            .entry(node_id)
            .or_default()
            .insert(PhaseT::render_phase_index());
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

    pub(super) fn image_version_create_usage(
        &self,
        usage: RenderGraphImageUsageId,
    ) -> RenderGraphImageUsageId {
        let version = self.image_usages[usage.0].version;
        self.image_resources[version.index].versions[version.version].create_usage
    }

    pub(super) fn redirect_image_usage(
        &mut self,
        usage: RenderGraphImageUsageId,
        from: RenderGraphImageVersionId,
        to: RenderGraphImageVersionId,
    ) {
        self.image_resources[from.index].versions[from.version].remove_read_usage(usage);
        self.image_resources[to.index].versions[to.version].add_read_usage(usage);
    }

    //
    // Get images
    //
    pub(super) fn buffer_resource(
        &self,
        usage_id: RenderGraphBufferUsageId,
    ) -> &RenderGraphBufferResource {
        let version = self.buffer_usage(usage_id).version;
        &self.buffer_resources[version.index]
    }

    pub(super) fn buffer_resource_mut(
        &mut self,
        usage_id: RenderGraphBufferUsageId,
    ) -> &mut RenderGraphBufferResource {
        let version = self.buffer_usage(usage_id).version;
        &mut self.buffer_resources[version.index]
    }

    //
    // Get buffer version infos
    //
    pub(super) fn buffer_usage(
        &self,
        usage_id: RenderGraphBufferUsageId,
    ) -> &RenderGraphBufferUsage {
        &self.buffer_usages[usage_id.0]
    }

    pub(super) fn buffer_version_info(
        &self,
        usage_id: RenderGraphBufferUsageId,
    ) -> &RenderGraphBufferResourceVersionInfo {
        let version = self.buffer_usage(usage_id).version;
        &self.buffer_resources[version.index].versions[version.version]
    }

    pub(super) fn buffer_version_info_mut(
        &mut self,
        usage_id: RenderGraphBufferUsageId,
    ) -> &mut RenderGraphBufferResourceVersionInfo {
        let version = self.buffer_usage(usage_id).version;
        &mut self.buffer_resources[version.index].versions[version.version]
    }

    pub(super) fn buffer_version_id(
        &self,
        usage_id: RenderGraphBufferUsageId,
    ) -> RenderGraphBufferVersionId {
        self.buffer_usage(usage_id).version
    }

    pub(super) fn buffer_version_create_usage(
        &self,
        usage: RenderGraphBufferUsageId,
    ) -> RenderGraphBufferUsageId {
        let version = self.buffer_usage(usage).version;
        self.buffer_resources[version.index].versions[version.version].create_usage
    }

    pub fn build_plan(self) -> RenderGraphPlan {
        profiling::scope!("Build Plan");
        RenderGraphPlan::new(self)
    }
}
