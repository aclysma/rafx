use super::*;
use crate::render_features::{RenderPhase, RenderPhaseIndex};
use crate::resources::{ImageViewResource, ResourceArc};
use crate::BufferResource;
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::{
    RafxCmdCopyBufferToBufferParams, RafxCmdCopyTextureToTextureParams, RafxColorClearValue,
    RafxDepthStencilClearValue, RafxLoadOp, RafxResourceState, RafxResourceType, RafxResult,
};

#[derive(Copy, Clone)]
pub enum RenderGraphQueue {
    DefaultGraphics,
    Index(u32),
}

/// An image that is being provided to the render graph that can be read/written by the graph
#[derive(Debug)]
pub struct RenderGraphExternalImage {
    pub external_image_id: RenderGraphExternalImageId,
    pub specification: RenderGraphImageSpecification,
    pub view_options: RenderGraphImageViewOptions,
    pub image_resource: ResourceArc<ImageViewResource>,
    pub image_resource_index: usize,

    // These are set when calling read_external_image/write_external_image
    pub input_usage: Option<RenderGraphImageUsageId>,
    pub output_usage: Option<RenderGraphImageUsageId>,

    //TODO: Use initial state
    #[allow(dead_code)]
    pub(super) initial_state: RafxResourceState,
    pub(super) final_state: RafxResourceState,
}

/// A buffer that is being provided to the render graph that can be read/written by the graph
#[derive(Debug)]
pub struct RenderGraphExternalBuffer {
    pub external_buffer_id: RenderGraphExternalBufferId,
    pub specification: RenderGraphBufferSpecification,
    pub buffer_resource: ResourceArc<BufferResource>,
    pub buffer_resource_index: usize,

    pub input_usage: Option<RenderGraphBufferUsageId>,
    pub output_usage: Option<RenderGraphBufferUsageId>,

    //TODO: Use initial/final state
    #[allow(dead_code)]
    pub(super) initial_state: RafxResourceState,
    #[allow(dead_code)]
    pub(super) final_state: RafxResourceState,
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

    /// Images that are passed into the graph that can be read/written by the graph
    pub(super) external_images: Vec<RenderGraphExternalImage>,
    pub(super) external_buffers: Vec<RenderGraphExternalBuffer>,

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

    /// Create an image but do not use it (may be passed around and used elsewhere)
    pub fn create_unattached_image(
        &mut self,
        create_node: RenderGraphNodeId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        self.add_image_create(create_node, constraint, view_options)
    }

    /// Create a new image and use it as a writable color attachment in a renderpass
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

    /// Create a new image and use it as a writable depth attachment in a renderpass
    pub fn create_depth_attachment(
        &mut self,
        node: RenderGraphNodeId,
        clear_depth_stencil_value: Option<RafxDepthStencilClearValue>,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        self.create_depth_stencil_attachment(
            node,
            clear_depth_stencil_value,
            constraint,
            view_options,
            true,
            false,
        )
    }

    /// Create a new image and use it as a writable depth/stencil attachment in a renderpass
    pub fn create_depth_stencil_attachment(
        &mut self,
        node: RenderGraphNodeId,
        clear_depth_stencil_value: Option<RafxDepthStencilClearValue>,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
        has_depth: bool,
        has_stencil: bool,
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
                has_depth,
                has_stencil,
            },
        );

        create_image
    }

    /// Create a new image and use it as a resolve attachment in a renderpass
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

    /// Use the given image as a read-only color attachment in a renderpass
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

    /// Use the given image as a read-only depth attachment in a renderpass
    pub fn read_depth_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) {
        self.read_depth_stencil_attachment(node, image, constraint, view_options, true, false);
    }

    /// Use the given image as a read-only depth/stencil attachment in a renderpass
    pub fn read_depth_stencil_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
        has_depth: bool,
        has_stencil: bool,
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
                has_depth,
                has_stencil,
            },
        );
    }

    /// Use the given image as a writable color attachment in a renderpass
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

    /// Use the given image as a writable depth attachment in a renderpass
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

    /// Use the given image as a writable depth/stencil attachment in a renderpass
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

    /// Use the given image as a shader resource (i.e. can sample from the image) in a renderpass
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

    /// Use the given buffer as a vertex buffer
    pub fn read_vertex_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::VERTEX_BUFFER;

        let usage = self.add_buffer_read(node, buffer, constraint);
        self.node_mut(node).vertex_buffer_reads.push(buffer);
        usage
    }

    /// Use the given buffer as an index buffer
    pub fn read_index_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::INDEX_BUFFER;

        let usage = self.add_buffer_read(node, buffer, constraint);
        self.node_mut(node).index_buffer_reads.push(buffer);
        usage
    }

    /// Use the given buffer as an indirect buffer
    pub fn read_indirect_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::INDIRECT_BUFFER;

        let usage = self.add_buffer_read(node, buffer, constraint);
        self.node_mut(node).indirect_buffer_reads.push(buffer);
        usage
    }

    /// Use the given buffer as a uniform buffer
    pub fn read_uniform_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::UNIFORM_BUFFER;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        let usage = self.add_buffer_read(node, buffer, constraint);
        self.node_mut(node).uniform_buffer_reads.push(buffer);
        usage
    }

    /// Use the given buffer as a copy source
    pub fn read_copy_src_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        let usage = self.add_buffer_read(node, buffer, constraint);
        self.node_mut(node).copy_src_buffer_reads.push(buffer);
        usage
    }

    /// Use the given buffer as a copy destination
    pub fn write_copy_dst_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        //NOTE: Be careful modifying this.. double check if any changes here copy_buffer_to_buffer()
        // need to be applied to the None dst_buffer codepath
        let (read_buffer, write_buffer) = self.add_buffer_modify(node, buffer, constraint);
        self.node_mut(node).copy_dst_buffer_writes.push(read_buffer);
        write_buffer
    }

    /// Use the given image as a copy source
    pub fn read_copy_src_image(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        let usage = self.add_image_read(node, image, constraint, view_options);
        self.node_mut(node).copy_src_image_reads.push(image);
        usage
    }

    /// Use the given image as a copy destination
    pub fn write_copy_dst_image(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        let (read_image, write_image) =
            self.add_image_modify(node, image, constraint, view_options);
        self.node_mut(node).copy_dst_image_writes.push(read_image);
        write_image
    }

    // Utility function for create_storage_buffer() and modify_storage_buffer() to setup a clear
    fn setup_buffer_clear_callback(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
    ) {
        self.set_callback(node, move |args| {
            let buffer = args.graph_context.buffer(buffer).unwrap();
            let builtin_pipelines = args.graph_context.resource_context().builtin_pipelines();
            builtin_pipelines.fill_buffer(
                &*args.command_buffer,
                args.graph_context.resource_context(),
                &buffer,
                0,
            )
        });
    }

    /// Create a storage buffer and use it as a writable shader resource
    pub fn create_storage_buffer(
        &mut self,
        node: RenderGraphNodeId,
        mut constraint: RenderGraphBufferConstraint,
        load_op: RafxLoadOp,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::BUFFER_READ_WRITE;

        match load_op {
            RafxLoadOp::DontCare => {
                let usage = self.add_buffer_create(node, constraint);
                self.node_mut(node).storage_buffer_creates.push(usage);
                usage
            }
            RafxLoadOp::Load => unimplemented!("RafxLoadOp::Load not supported in create_storage_buffer call. Use modify_storage_image instead."),
            RafxLoadOp::Clear => {
                // Add a node to clear the buffer
                let clear_node = self.add_node("create_storage_buffer_clear", self.node(node).queue);
                let cleared_buffer = self.add_buffer_create(clear_node, constraint);
                self.node_mut(clear_node).storage_buffer_creates.push(cleared_buffer);
                self.setup_buffer_clear_callback(clear_node, cleared_buffer);

                // Now set this node up to modify the buffer
                let (read_usage, write_usage) = self.add_buffer_modify(node, cleared_buffer, Default::default());
                self.node_mut(node).storage_buffer_modifies.push(read_usage);
                write_usage
            }
        }
    }

    /// Use the given storage buffer as a read-only shader resource
    pub fn read_storage_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
        // load_op is assumed to be Load, otherwise use create_storage_image
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::BUFFER_READ_WRITE;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        let usage = self.add_buffer_read(node, buffer, constraint);
        self.node_mut(node).storage_buffer_reads.push(usage);
        usage
    }

    /// Use the given storage buffer as a writable shader resource
    pub fn modify_storage_buffer(
        &mut self,
        node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
        load_op: RafxLoadOp,
    ) -> RenderGraphBufferUsageId {
        constraint.resource_type |= RafxResourceType::BUFFER_READ_WRITE;

        match load_op {
            // Don't clear the buffer
            RafxLoadOp::DontCare | RafxLoadOp::Load => {
                let (read_buffer, write_buffer) = self.add_buffer_modify(node, buffer, constraint);
                self.node_mut(node)
                    .storage_buffer_modifies
                    .push(read_buffer);
                write_buffer
            }
            RafxLoadOp::Clear => {
                // Add a node to clear the buffer
                let clear_node =
                    self.add_node("modify_storage_buffer_clear", self.node(node).queue);
                let (cleared_buffer_read, cleared_buffer_write) =
                    self.add_buffer_modify(clear_node, buffer, constraint);
                self.node_mut(clear_node)
                    .storage_buffer_modifies
                    .push(cleared_buffer_read);
                self.setup_buffer_clear_callback(clear_node, cleared_buffer_write);

                let (read_usage, write_usage) =
                    self.add_buffer_modify(node, cleared_buffer_write, Default::default());
                self.node_mut(node).storage_buffer_modifies.push(read_usage);
                write_usage
            }
        }
    }

    //NOTE: Image will not be cleared, use clear_image_before_pass() on the returned value if it needs to be
    // initialized to zero
    /// Create a storage image and use it as a writable shader resource
    pub fn create_storage_image(
        &mut self,
        node: RenderGraphNodeId,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::TEXTURE_READ_WRITE;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        let usage = self.add_image_create(node, constraint, view_options);
        self.node_mut(node).storage_image_creates.push(usage);
        usage
    }

    /// Use the given storage image as a read-only shader resource
    pub fn read_storage_image(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
        // load_op is assumed to be Load, otherwise use create_storage_image
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::TEXTURE_READ_WRITE;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        let usage = self.add_image_read(node, image, constraint, view_options);
        self.node_mut(node).storage_image_reads.push(usage);

        usage
    }

    /// Use the given storage image as a writable shader resource
    pub fn modify_storage_image(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        mut constraint: RenderGraphImageConstraint,
        view_options: RenderGraphImageViewOptions,
    ) -> RenderGraphImageUsageId {
        constraint.resource_type |= RafxResourceType::TEXTURE_READ_WRITE;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        let (read_image, write_image) =
            self.add_image_modify(node, image, constraint, view_options);

        self.node_mut(node).storage_image_modifies.push(read_image);
        write_image
    }

    /// Register a non-rendergraph image for use with the rendergraph. (Use read_external_image() or
    /// write_external_image() to access it)
    pub fn add_external_image(
        &mut self,
        image_resource: ResourceArc<ImageViewResource>,
        view_options: RenderGraphImageViewOptions,
        #[allow(unused_mut)] mut initial_state: RafxResourceState,
        final_state: RafxResourceState,
    ) -> RenderGraphExternalImageId {
        let image_view = image_resource.get_raw().image;
        let image = image_view.get_raw().image;

        // A vulkan-specific check - images may have started in an undefined layout. Normally this
        // is handled by transitioning the resource into the correct state, and there is logic in
        // the vulkan backend to assume the first transition is from UNDEFINED state. However if we
        // indicate the "normal" initial state for the frame, we might not even try to do a layout
        // transition and we will skip that code in the backend.
        #[cfg(feature = "rafx-vulkan")]
        {
            if let Some(vk_texture) = image.vk_texture() {
                if vk_texture.is_in_initial_undefined_layout() {
                    initial_state = RafxResourceState::UNDEFINED
                }
            }
        }

        let texture_def = image.texture_def();
        let specification = RenderGraphImageSpecification {
            resource_type: texture_def.resource_type,
            format: texture_def.format,
            extents: texture_def.extents,
            mip_count: texture_def.mip_count,
            layer_count: texture_def.array_length,
            samples: texture_def.sample_count,
        };

        let external_image_id = RenderGraphExternalImageId(self.external_images.len());

        // Add it to the graph
        let image_resource_index = self.image_resources.len();
        self.image_resources.push(RenderGraphImageResource::new());

        let external_image = RenderGraphExternalImage {
            external_image_id,
            specification,
            view_options,
            image_resource,
            image_resource_index,
            input_usage: None,
            output_usage: None,
            initial_state,
            final_state,
        };

        self.external_images.push(external_image);
        external_image_id
    }

    /// Register a non-rendergraph buffer for use with the rendergraph. (Use read_external_buffer() or
    /// write_external_buffer() to access it)
    pub fn add_external_buffer(
        &mut self,
        buffer_resource: ResourceArc<BufferResource>,
        initial_state: RafxResourceState,
        final_state: RafxResourceState,
    ) -> RenderGraphExternalBufferId {
        let buffer = buffer_resource.get_raw().buffer;
        let buffer_def = buffer.buffer_def();
        let specification = RenderGraphBufferSpecification {
            size: buffer_def.size,
            resource_type: buffer_def.resource_type,
        };

        let external_buffer_id = RenderGraphExternalBufferId(self.external_buffers.len());

        let buffer_resource_index = self.buffer_resources.len();
        self.buffer_resources.push(RenderGraphBufferResource::new());

        let external_buffer = RenderGraphExternalBuffer {
            external_buffer_id,
            specification,
            buffer_resource,
            buffer_resource_index,
            input_usage: None,
            output_usage: None,
            initial_state,
            final_state,
        };

        self.external_buffers.push(external_buffer);
        external_buffer_id
    }

    /// Use an external image as an input for a render graph node
    pub fn read_external_image(
        &mut self,
        external_image_id: RenderGraphExternalImageId,
    ) -> RenderGraphImageUsageId {
        let external_image = &self.external_images[external_image_id.0];
        let image_resource_index = external_image.image_resource_index;
        let view_options = external_image.view_options.clone();

        let version_id = RenderGraphImageVersionId {
            index: image_resource_index,
            version: 0,
        };

        let input_usage = self.add_image_usage(
            RenderGraphImageUser::Input(external_image_id),
            version_id,
            RenderGraphImageUsageType::Input,
            view_options,
        );

        let version_info =
            RenderGraphImageResourceVersionInfo::new(RenderGraphNodeId(0), input_usage);
        let resource = &mut self.image_resources[image_resource_index];
        resource.versions.push(version_info);

        self.external_images[external_image_id.0].input_usage = Some(input_usage);
        input_usage
    }

    /// Use an external buffer as an input for a render graph node
    pub fn read_external_buffer(
        &mut self,
        external_buffer_id: RenderGraphExternalBufferId,
    ) -> RenderGraphBufferUsageId {
        let external_buffer = &self.external_buffers[external_buffer_id.0];
        let buffer_resource_index = external_buffer.buffer_resource_index;

        let version_id = RenderGraphBufferVersionId {
            index: buffer_resource_index,
            version: 0,
        };

        let input_usage = self.add_buffer_usage(
            RenderGraphBufferUser::Input(external_buffer_id),
            version_id,
            RenderGraphBufferUsageType::Input,
        );

        let version_info =
            RenderGraphBufferResourceVersionInfo::new(RenderGraphNodeId(0), input_usage);
        let resource = &mut self.buffer_resources[buffer_resource_index];
        resource.versions.push(version_info);

        self.external_buffers[external_buffer_id.0].input_usage = Some(input_usage);
        input_usage
    }

    /// Use an external image as an output for a render graph node
    pub fn write_external_image(
        &mut self,
        external_image_id: RenderGraphExternalImageId,
        image_id: RenderGraphImageUsageId,
    ) {
        let external_image = &self.external_images[external_image_id.0];
        let view_options = external_image.view_options.clone();

        let version_id = self.image_version_id(image_id);
        let usage_id = self.add_image_usage(
            RenderGraphImageUser::Output(external_image_id),
            version_id,
            RenderGraphImageUsageType::Output,
            view_options,
        );

        let image_version = self.image_version_info_mut(image_id);
        image_version.read_usages.push(usage_id);

        self.external_images[external_image_id.0].output_usage = Some(usage_id);
    }

    /// Use an external buffer as an output for a render graph node
    pub fn write_external_buffer(
        &mut self,
        external_buffer_id: RenderGraphExternalBufferId,
        buffer_id: RenderGraphBufferUsageId,
    ) {
        let version_id = self.buffer_version_id(buffer_id);
        let usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Output(external_buffer_id),
            version_id,
            RenderGraphBufferUsageType::Output,
        );

        let buffer_version = self.buffer_version_info_mut(buffer_id);
        buffer_version.read_usages.push(usage_id);

        self.external_buffers[external_buffer_id.0].output_usage = Some(usage_id);
    }

    /// Schedule a GPU copy from one buffer to another. If dst_buffer is None, create a copy of the
    /// src_buffer.
    pub fn copy_buffer_to_buffer(
        &mut self,
        name: RenderGraphNodeName,
        queue: RenderGraphQueue,
        src_buffer: RenderGraphBufferUsageId,
        dst_buffer: Option<RenderGraphBufferUsageId>,
        params: Option<RafxCmdCopyBufferToBufferParams>,
    ) -> RenderGraphBufferUsageId {
        let node = self.add_node(name, queue);
        let src_buffer = self.read_copy_src_buffer(node, src_buffer, Default::default());
        let dst_buffer = if let Some(dst_buffer) = dst_buffer {
            self.write_copy_dst_buffer(node, dst_buffer, Default::default())
        } else {
            // This is a combination of create_storage_buffer() and write_copy_dst_buffer()
            let dst_buffer = self.add_buffer_create(node, Default::default());
            self.node_mut(node).copy_dst_buffer_writes.push(dst_buffer);
            dst_buffer
        };

        self.nodes[node.0]
            .buffer_copies
            .push(RenderGraphBufferCopy {
                input: src_buffer,
                output: dst_buffer,
                constraint: Default::default(),
            });

        self.set_callback(node, move |args| {
            let src = args.graph_context.buffer(src_buffer).unwrap();
            let dst = args.graph_context.buffer(dst_buffer).unwrap();

            let params = params.clone().unwrap_or_else(|| {
                let src_size = src.get_raw().buffer.buffer_def().size;
                let dst_size = dst.get_raw().buffer.buffer_def().size;
                assert_eq!(src_size, dst_size);

                RafxCmdCopyBufferToBufferParams {
                    src_byte_offset: 0,
                    dst_byte_offset: 0,
                    size: src_size,
                }
            });

            args.command_buffer.cmd_copy_buffer_to_buffer(
                &src.get_raw().buffer,
                &dst.get_raw().buffer,
                &params,
            )
        });

        dst_buffer
    }

    /// Create a buffer and schedule a GPU copy from the src_buffer into it. (Shorthand for using
    /// copy_buffer_to_buffer())
    pub fn clone_buffer(
        &mut self,
        name: RenderGraphNodeName,
        queue: RenderGraphQueue,
        src_buffer: RenderGraphBufferUsageId,
        params: Option<RafxCmdCopyBufferToBufferParams>,
    ) -> RenderGraphBufferUsageId {
        self.copy_buffer_to_buffer(name, queue, src_buffer, None, params)
    }

    /// Schedule a GPU copy from one image to another. If dst_image is None, create a copy of the
    /// src_image.
    pub fn copy_image_to_image(
        &mut self,
        name: RenderGraphNodeName,
        queue: RenderGraphQueue,
        src_image: RenderGraphImageUsageId,
        dst_image: Option<RenderGraphImageUsageId>,
        params: Option<RafxCmdCopyTextureToTextureParams>,
    ) -> RenderGraphImageUsageId {
        let node = self.add_node(name, queue);

        let array_slices = params.as_ref().map(|x| x.array_slices).flatten();
        let src_image = self.read_copy_src_image(
            node,
            src_image,
            Default::default(),
            RenderGraphImageViewOptions {
                texture_bind_type: None,
                array_slice: array_slices.map(|x| x[0]),
                mip_slice: Some(params.as_ref().map(|x| x.src_mip_level).unwrap_or(0)),
            },
        );

        let dst_image_view_options = RenderGraphImageViewOptions {
            texture_bind_type: None,
            array_slice: array_slices.map(|x| x[1]),
            mip_slice: Some(params.as_ref().map(|x| x.dst_mip_level).unwrap_or(0)),
        };

        let dst_image = if let Some(dst_image) = dst_image {
            self.write_copy_dst_image(node, dst_image, Default::default(), dst_image_view_options)
        } else {
            // This is a combination of create_storage_image() and write_copy_dst_image()
            let dst_image = self.add_image_create(node, Default::default(), dst_image_view_options);
            self.node_mut(node).copy_dst_image_writes.push(dst_image);
            dst_image
        };

        self.nodes[node.0].image_copies.push(RenderGraphImageCopy {
            input: src_image,
            output: dst_image,
            constraint: Default::default(),
        });

        self.set_callback(node, move |args| {
            let src = args.graph_context.image_view(src_image).unwrap();
            let dst = args.graph_context.image_view(dst_image).unwrap();

            let params = params.clone().unwrap_or_else(|| {
                let src_size = src.get_raw().image.get_raw().image.texture_def().extents;
                let dst_size = dst.get_raw().image.get_raw().image.texture_def().extents;
                assert_eq!(src_size, dst_size);

                RafxCmdCopyTextureToTextureParams {
                    src_offset: Default::default(),
                    dst_offset: Default::default(),
                    extents: src_size,
                    src_mip_level: 0,
                    dst_mip_level: 0,
                    array_slices: None,
                }
            });

            args.command_buffer.cmd_copy_texture_to_texture(
                &src.get_raw().image.get_raw().image,
                &dst.get_raw().image.get_raw().image,
                &params,
            )?;
            Ok(())
        });

        dst_image
    }

    /// Create a image and schedule a GPU copy from the src_buffer into it. (Shorthand for using
    /// copy_image_to_image())
    pub fn clone_image(
        &mut self,
        name: RenderGraphNodeName,
        queue: RenderGraphQueue,
        src_image: RenderGraphImageUsageId,
        params: Option<RafxCmdCopyTextureToTextureParams>,
    ) -> RenderGraphImageUsageId {
        self.copy_image_to_image(name, queue, src_image, None, params)
    }

    /// Render the src_image into the dst_image. This is less efficient than copy_image_to_image,
    /// but is more flexible. (It is essentially drawing the src_image as a quad on the dst_image)
    pub fn blit_image_to_image(
        &mut self,
        name: RenderGraphNodeName,
        queue: RenderGraphQueue,
        src_image: RenderGraphImageUsageId,
        src_min_uv: glam::Vec2,
        src_max_uv: glam::Vec2,
        dst_image: RenderGraphImageUsageId,
        dst_min_uv: glam::Vec2,
        dst_max_uv: glam::Vec2,
    ) -> RenderGraphImageUsageId {
        let node = self.add_node(name, queue);
        let src_image = self.sample_image(node, src_image, Default::default(), Default::default());
        let dst_image = self.modify_color_attachment(
            node,
            dst_image,
            0,
            None,
            Default::default(),
            Default::default(),
        );

        self.set_renderpass_callback(node, move |args| {
            let src_image = args.graph_context.image_view(src_image).unwrap();
            let dst_image = args.graph_context.image_view(dst_image).unwrap();
            let builtin_pipelines = args.graph_context.resource_context().builtin_pipelines();

            builtin_pipelines.blit_image(
                &*args.command_buffer,
                args.graph_context.resource_context(),
                &args.render_target_meta,
                &src_image,
                src_min_uv,
                src_max_uv,
                &dst_image,
                dst_min_uv,
                dst_max_uv,
            )
        });

        dst_image
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

    pub fn set_node_required(
        &mut self,
        node_id: RenderGraphNodeId,
    ) {
        self.node_mut(node_id).can_be_culled = false;
    }

    pub fn set_buffer_required(
        &mut self,
        buffer_usage: RenderGraphBufferUsageId,
    ) {
        let creator_node = self.buffer_version_info(buffer_usage).creator_node;
        self.set_node_required(creator_node);
    }

    pub fn set_image_required(
        &mut self,
        image_usage: RenderGraphImageUsageId,
    ) {
        let creator_node = self.image_version_info(image_usage).creator_node;
        self.set_node_required(creator_node);
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
            RenderGraphNodeVisitNodeCallback::Render(Box::new(f)),
        );
        // If this trips, multiple callbacks were set on the node
        assert!(old.is_none());
    }

    /// Adds a callback for compute based nodes
    pub fn set_callback<CallbackFnT>(
        &mut self,
        node_id: RenderGraphNodeId,
        f: CallbackFnT,
    ) where
        CallbackFnT: Fn(VisitComputeNodeArgs) -> RafxResult<()> + 'static + Send,
    {
        let old = self.visit_node_callbacks.insert(
            node_id,
            RenderGraphNodeVisitNodeCallback::Callback(Box::new(f)),
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
    pub(super) fn image_usage(
        &self,
        usage_id: RenderGraphImageUsageId,
    ) -> &RenderGraphImageUsage {
        &self.image_usages[usage_id.0]
    }

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
    // Get buffers
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

    //
    // Internal debug functions
    //
    pub(super) fn debug_user_name_of_image_usage(
        &self,
        usage: RenderGraphImageUsageId,
    ) -> String {
        let user = self.image_usage(usage).user;
        match user {
            RenderGraphImageUser::Node(node_id) => {
                format!("Node {:?} {:?}", node_id, self.node(node_id).name)
            }
            RenderGraphImageUser::Input(image_id) => format!("InputImage {:?}", image_id),
            RenderGraphImageUser::Output(image_id) => format!("OutputImage {:?}", image_id),
        }
    }

    pub(super) fn debug_user_name_of_buffer_usage(
        &self,
        usage: RenderGraphBufferUsageId,
    ) -> String {
        let user = self.buffer_usage(usage).user;
        match user {
            RenderGraphBufferUser::Node(node_id) => {
                format!("Node {:?} {:?}", node_id, self.node(node_id).name)
            }
            RenderGraphBufferUser::Input(buffer_id) => format!("InputBuffer {:?}", buffer_id),
            RenderGraphBufferUser::Output(buffer_id) => format!("OutputBuffer {:?}", buffer_id),
        }
    }

    pub fn build_plan(
        self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> RenderGraphPlan {
        profiling::scope!("Build Plan");
        RenderGraphPlan::new(self, swapchain_surface_info)
    }
}
