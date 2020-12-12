use super::*;
use crate::resources::{ImageViewResource, ResourceArc};
use crate::vk_description::SwapchainSurfaceInfo;
use crate::{vk_description as dsc, BufferResource};

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

    pub(super) final_layout: dsc::ImageLayout,
    pub(super) final_access_flags: vk::AccessFlags,
    pub(super) final_stage_flags: vk::PipelineStageFlags,
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

    pub(super) final_access_flags: vk::AccessFlags,
    pub(super) final_stage_flags: vk::PipelineStageFlags,
}

/// A collection of nodes and resources. Nodes represent an event or process that will occur at
/// a certain time. (For now, they just represent subpasses that may be merged with each other.)
/// Resources represent images and buffers that may be read/written by nodes.
#[derive(Default, Debug)]
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
        preferred_layout: dsc::ImageLayout,
        subresource_range: RenderGraphImageSubresourceRange,
        view_type: dsc::ImageViewType,
        // _access_flags: vk::AccessFlags,
        // _stage_flags: vk::PipelineStageFlags,
        // _image_aspect_flags: vk::ImageAspectFlags,
    ) -> RenderGraphImageUsageId {
        let usage_id = RenderGraphImageUsageId(self.image_usages.len());
        self.image_usages.push(RenderGraphImageUsage {
            user,
            usage_type,
            version,
            preferred_layout,
            subresource_range,
            view_type,
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
        constraint: RenderGraphImageConstraint,
        preferred_layout: dsc::ImageLayout,
        subresource_range: RenderGraphImageSubresourceRange,
        view_type: dsc::ImageViewType,
        // access_flags: vk::AccessFlags,
        // stage_flags: vk::PipelineStageFlags,
        // image_aspect_flags: vk::ImageAspectFlags,
    ) -> RenderGraphImageUsageId {
        let version_id = RenderGraphImageVersionId {
            index: self.image_resources.len(),
            version: 0,
        };
        let usage_id = self.add_image_usage(
            RenderGraphImageUser::Node(create_node),
            version_id,
            RenderGraphImageUsageType::Create,
            preferred_layout,
            subresource_range,
            view_type,
            // access_flags,
            // stage_flags,
            // image_aspect_flags,
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
        subresource_range: RenderGraphImageSubresourceRange,
        view_type: dsc::ImageViewType,
        preferred_layout: dsc::ImageLayout,
        // access_flags: vk::AccessFlags,
        // stage_flags: vk::PipelineStageFlags,
        // image_aspect_flags: vk::ImageAspectFlags,
    ) -> RenderGraphImageUsageId {
        let version_id = self.image_usages[image.0].version;

        let usage_id = self.add_image_usage(
            RenderGraphImageUser::Node(read_node),
            version_id,
            RenderGraphImageUsageType::Read,
            preferred_layout,
            subresource_range,
            view_type,
            // access_flags,
            // stage_flags,
            // image_aspect_flags,
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
        subresource_range: RenderGraphImageSubresourceRange,
        view_type: dsc::ImageViewType,
        preferred_layout: dsc::ImageLayout,
        // read_access_flags: vk::AccessFlags,
        // read_stage_flags: vk::PipelineStageFlags,
        // read_image_aspect_flags: vk::ImageAspectFlags,
        // write_access_flags: vk::AccessFlags,
        // write_stage_flags: vk::PipelineStageFlags,
        // write_image_aspect_flags: vk::ImageAspectFlags,
    ) -> (RenderGraphImageUsageId, RenderGraphImageUsageId) {
        let read_version_id = self.image_usages[image.0].version;

        let read_usage_id = self.add_image_usage(
            RenderGraphImageUser::Node(modify_node),
            read_version_id,
            RenderGraphImageUsageType::ModifyRead,
            preferred_layout,
            subresource_range.clone(),
            view_type.clone(),
            // read_access_flags,
            // read_stage_flags,
            // read_image_aspect_flags,
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
            preferred_layout,
            subresource_range,
            view_type,
            // write_access_flags,
            // write_stage_flags,
            // write_image_aspect_flags,
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
        view_type: dsc::ImageViewType,
    ) -> RenderGraphImageUsageId {
        self.add_image_create(
            create_node,
            constraint,
            dsc::ImageLayout::Undefined,
            RenderGraphImageSubresourceRange::AllMipsAllLayers,
            view_type,
            // vk::AccessFlags::empty(),
            // vk::PipelineStageFlags::empty(),
            // vk::ImageAspectFlags::empty(),
        )
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

        // Add the read to the graph
        let create_image = self.add_image_create(
            node,
            constraint,
            dsc::ImageLayout::ColorAttachmentOptimal,
            RenderGraphImageSubresourceRange::NoMipsNoLayers,
            dsc::ImageViewType::Type2D,
            // vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            // vk::ImageAspectFlags::COLOR,
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

        // Add the read to the graph
        let create_image = self.add_image_create(
            node,
            constraint,
            dsc::ImageLayout::DepthAttachmentOptimal,
            RenderGraphImageSubresourceRange::NoMipsNoLayers,
            dsc::ImageViewType::Type2D,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH,
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

        // Add the read to the graph
        let create_image = self.add_image_create(
            node,
            constraint,
            dsc::ImageLayout::DepthStencilAttachmentOptimal,
            RenderGraphImageSubresourceRange::NoMipsNoLayers,
            dsc::ImageViewType::Type2D,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
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

        let create_image = self.add_image_create(
            node,
            constraint,
            dsc::ImageLayout::ColorAttachmentOptimal,
            RenderGraphImageSubresourceRange::NoMipsNoLayers,
            dsc::ImageViewType::Type2D,
            // vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            // vk::ImageAspectFlags::COLOR,
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
        subresource_range: RenderGraphImageSubresourceRange,
    ) {
        constraint.aspect_flags |= vk::ImageAspectFlags::COLOR;
        constraint.usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;

        // Add the read to the graph
        let read_image = self.add_image_read(
            node,
            image,
            constraint,
            subresource_range,
            dsc::ImageViewType::Type2D,
            dsc::ImageLayout::ColorAttachmentOptimal,
            // vk::AccessFlags::COLOR_ATTACHMENT_READ,
            // vk::PipelineStageFlags::FRAGMENT_SHADER,
            // vk::ImageAspectFlags::COLOR,
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
        subresource_range: RenderGraphImageSubresourceRange,
    ) {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;

        // Add the read to the graph
        let read_image = self.add_image_read(
            node,
            image,
            constraint,
            subresource_range,
            dsc::ImageViewType::Type2D,
            dsc::ImageLayout::DepthAttachmentOptimal,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH,
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
        subresource_range: RenderGraphImageSubresourceRange,
    ) {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;

        // Add the read to the graph
        let read_image = self.add_image_read(
            node,
            image,
            constraint,
            subresource_range,
            dsc::ImageViewType::Type2D,
            dsc::ImageLayout::DepthStencilAttachmentOptimal,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
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
        clear_color_value: Option<vk::ClearColorValue>,
        mut constraint: RenderGraphImageConstraint,
        subresource_range: RenderGraphImageSubresourceRange,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::COLOR;
        constraint.usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;

        // Add the read to the graph
        let (read_image, write_image) = self.add_image_modify(
            node,
            image,
            constraint,
            subresource_range,
            dsc::ImageViewType::Type2D,
            dsc::ImageLayout::ColorAttachmentOptimal,
            // vk::AccessFlags::COLOR_ATTACHMENT_READ,
            // vk::PipelineStageFlags::FRAGMENT_SHADER,
            // vk::ImageAspectFlags::COLOR,
            // vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            // vk::ImageAspectFlags::COLOR,
        );

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
        clear_depth_stencil_value: Option<vk::ClearDepthStencilValue>,
        mut constraint: RenderGraphImageConstraint,
        subresource_range: RenderGraphImageSubresourceRange,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;

        // Add the read to the graph
        let (read_image, write_image) = self.add_image_modify(
            node,
            image,
            constraint,
            subresource_range,
            dsc::ImageViewType::Type2D,
            dsc::ImageLayout::DepthAttachmentOptimal,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
            //     | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH,
        );

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
        clear_depth_stencil_value: Option<vk::ClearDepthStencilValue>,
        mut constraint: RenderGraphImageConstraint,
        subresource_range: RenderGraphImageSubresourceRange,
    ) -> RenderGraphImageUsageId {
        constraint.aspect_flags |= vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL;
        constraint.usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;

        // Add the read to the graph
        let (read_image, write_image) = self.add_image_modify(
            node,
            image,
            constraint,
            subresource_range,
            dsc::ImageViewType::Type2D,
            dsc::ImageLayout::DepthStencilAttachmentOptimal,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
            //     | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
            // vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            // vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            //     | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            // vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
        );

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
        subresource_range: RenderGraphImageSubresourceRange,
        view_type: dsc::ImageViewType,
    ) -> RenderGraphImageUsageId {
        // Don't assume color, we might sample a depth image
        //constraint.aspect_flags |= vk::ImageAspectFlags::COLOR;
        constraint.usage_flags |= vk::ImageUsageFlags::SAMPLED;

        // Add the read to the graph
        let usage = self.add_image_read(
            node,
            image,
            //RenderGraphAttachmentType::NotAttached,
            constraint,
            subresource_range,
            view_type,
            dsc::ImageLayout::ShaderReadOnlyOptimal,
            // vk::AccessFlags::SHADER_READ,
            // vk::PipelineStageFlags::FRAGMENT_SHADER,
            // vk::ImageAspectFlags::COLOR,
        );

        self.node_mut(node).sampled_images.push(usage);
        usage
    }

    pub fn set_output_image(
        &mut self,
        image_id: RenderGraphImageUsageId,
        dst_image: ResourceArc<ImageViewResource>,
        specification: RenderGraphImageSpecification,
        subresource_range: RenderGraphImageSubresourceRange,
        view_type: dsc::ImageViewType,
        layout: dsc::ImageLayout,
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
    ) -> RenderGraphOutputImageId {
        if specification.usage_flags == vk::ImageUsageFlags::empty() {
            panic!("An output image with empty ImageUsageFlags in the specification is almost certainly a mistake.");
        }

        if specification.aspect_flags == vk::ImageAspectFlags::empty() {
            panic!("An output image with empty ImageAspectFlags in the specification is almost certainly a mistake.");
        }

        let output_image_id = RenderGraphOutputImageId(self.output_images.len());

        let version_id = self.image_version_id(image_id);
        let usage_id = self.add_image_usage(
            RenderGraphImageUser::Output(output_image_id),
            version_id,
            RenderGraphImageUsageType::Output,
            layout,
            subresource_range,
            view_type,
            // access_flags,
            // stage_flags,
            // specification.aspect_flags,
        );

        let image_version = self.image_version_info_mut(image_id);
        image_version.read_usages.push(usage_id);

        let output_image = RenderGraphOutputImage {
            output_image_id,
            usage: usage_id,
            specification,
            dst_image,
            final_layout: layout,
            final_access_flags: access_flags,
            final_stage_flags: stage_flags,
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
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
    ) -> RenderGraphBufferUsageId {
        let usage_id = RenderGraphBufferUsageId(self.buffer_usages.len());
        self.buffer_usages.push(RenderGraphBufferUsage {
            user,
            usage_type,
            version,
            access_flags,
            stage_flags,
        });
        usage_id
    }

    // Add a buffer that can be used by nodes
    pub(super) fn add_buffer_create(
        &mut self,
        create_node: RenderGraphNodeId,
        constraint: RenderGraphBufferConstraint,
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
    ) -> RenderGraphBufferUsageId {
        let version_id = RenderGraphBufferVersionId {
            index: self.buffer_resources.len(),
            version: 0,
        };
        let usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Node(create_node),
            version_id,
            RenderGraphBufferUsageType::Create,
            access_flags,
            stage_flags,
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
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
    ) -> RenderGraphBufferUsageId {
        let version_id = self.buffer_usages[buffer.0].version;

        let usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Node(read_node),
            version_id,
            RenderGraphBufferUsageType::Read,
            access_flags,
            stage_flags,
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
        read_access_flags: vk::AccessFlags,
        read_stage_flags: vk::PipelineStageFlags,
        write_access_flags: vk::AccessFlags,
        write_stage_flags: vk::PipelineStageFlags,
    ) -> (RenderGraphBufferUsageId, RenderGraphBufferUsageId) {
        let read_version_id = self.buffer_usages[buffer.0].version;

        let read_usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Node(modify_node),
            read_version_id,
            RenderGraphBufferUsageType::ModifyRead,
            read_access_flags,
            read_stage_flags,
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
            write_access_flags,
            write_stage_flags,
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
        self.add_buffer_create(
            create_node,
            constraint,
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::empty(),
        )
    }

    pub fn read_vertex_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.usage_flags |= vk::BufferUsageFlags::VERTEX_BUFFER;

        self.add_buffer_read(
            read_node,
            buffer,
            constraint,
            vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            vk::PipelineStageFlags::VERTEX_INPUT,
        )
    }

    pub fn read_index_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.usage_flags |= vk::BufferUsageFlags::INDEX_BUFFER;

        self.add_buffer_read(
            read_node,
            buffer,
            constraint,
            vk::AccessFlags::INDEX_READ,
            vk::PipelineStageFlags::VERTEX_INPUT,
        )
    }

    pub fn read_indirect_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.usage_flags |= vk::BufferUsageFlags::INDIRECT_BUFFER;

        self.add_buffer_read(
            read_node,
            buffer,
            constraint,
            vk::AccessFlags::INDIRECT_COMMAND_READ,
            vk::PipelineStageFlags::DRAW_INDIRECT,
        )
    }

    pub fn read_uniform_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.usage_flags |= vk::BufferUsageFlags::UNIFORM_BUFFER;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        self.add_buffer_read(
            read_node,
            buffer,
            constraint,
            vk::AccessFlags::UNIFORM_READ,
            vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
        )
    }

    pub fn create_storage_buffer(
        &mut self,
        create_node: RenderGraphNodeId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.usage_flags |= vk::BufferUsageFlags::STORAGE_BUFFER;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        self.add_buffer_create(
            create_node,
            constraint,
            vk::AccessFlags::SHADER_WRITE,
            vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
        )
    }

    pub fn read_storage_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.usage_flags |= vk::BufferUsageFlags::STORAGE_BUFFER;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        self.add_buffer_read(
            read_node,
            buffer,
            constraint,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
        )
    }

    pub fn modify_storage_buffer(
        &mut self,
        read_node: RenderGraphNodeId,
        buffer: RenderGraphBufferUsageId,
        mut constraint: RenderGraphBufferConstraint,
    ) -> RenderGraphBufferUsageId {
        constraint.usage_flags |= vk::BufferUsageFlags::STORAGE_BUFFER;

        //TODO: In the future could consider options for determining stage flags to be compute or
        // fragment. Check node queue? Check if attachments exist? Explicit?
        let (_read_buffer, write_buffer) = self.add_buffer_modify(
            read_node,
            buffer,
            constraint,
            vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
            vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::AccessFlags::SHADER_WRITE,
            vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
        );

        write_buffer
    }

    pub fn set_output_buffer(
        &mut self,
        buffer_id: RenderGraphBufferUsageId,
        dst_buffer: ResourceArc<BufferResource>,
        specification: RenderGraphBufferSpecification,
        access_flags: vk::AccessFlags,
        stage_flags: vk::PipelineStageFlags,
    ) -> RenderGraphOutputBufferId {
        if specification.usage_flags == vk::BufferUsageFlags::empty() {
            panic!("An output buffer with empty BufferUsageFlags in the specification is almost certainly a mistake.");
        }

        let output_buffer_id = RenderGraphOutputBufferId(self.output_buffers.len());

        let version_id = self.buffer_version_id(buffer_id);
        let usage_id = self.add_buffer_usage(
            RenderGraphBufferUser::Output(output_buffer_id),
            version_id,
            RenderGraphBufferUsageType::Output,
            access_flags,
            stage_flags,
        );

        let buffer_version = self.buffer_version_info_mut(buffer_id);
        buffer_version.read_usages.push(usage_id);

        let output_buffer = RenderGraphOutputBuffer {
            output_buffer_id,
            usage: usage_id,
            specification,
            dst_buffer,
            final_access_flags: access_flags,
            final_stage_flags: stage_flags,
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
        let version = self.buffer_usages[usage_id.0].version;
        &self.buffer_resources[version.index]
    }

    pub(super) fn buffer_resource_mut(
        &mut self,
        usage_id: RenderGraphBufferUsageId,
    ) -> &mut RenderGraphBufferResource {
        let version = self.buffer_usages[usage_id.0].version;
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
        let version = self.buffer_usages[usage_id.0].version;
        &self.buffer_resources[version.index].versions[version.version]
    }

    pub(super) fn buffer_version_info_mut(
        &mut self,
        usage_id: RenderGraphBufferUsageId,
    ) -> &mut RenderGraphBufferResourceVersionInfo {
        let version = self.buffer_usages[usage_id.0].version;
        &mut self.buffer_resources[version.index].versions[version.version]
    }

    pub(super) fn buffer_version_id(
        &self,
        usage_id: RenderGraphBufferUsageId,
    ) -> RenderGraphBufferVersionId {
        self.buffer_usages[usage_id.0].version
    }

    pub(super) fn buffer_version_create_usage(
        &self,
        usage: RenderGraphBufferUsageId,
    ) -> RenderGraphBufferUsageId {
        let version = self.buffer_usages[usage.0].version;
        self.buffer_resources[version.index].versions[version.version].create_usage
    }

    pub fn build_plan(
        self,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> RenderGraphPlan {
        profiling::scope!("Build Plan");
        RenderGraphPlan::new(self, swapchain_surface_info)
    }
}
