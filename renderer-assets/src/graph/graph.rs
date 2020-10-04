use super::*;
use fnv::{FnvHashSet, FnvHashMap};
use crate::vk_description as dsc;

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
pub struct AssignPhysicalImagesResult {
    map_image_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId>,
    physical_image_usages: FnvHashMap<PhysicalImageId, Vec<RenderGraphImageUsageId>>,
}

#[derive(Debug)]
pub struct RenderGraphInputImage {
    pub usage: RenderGraphImageUsageId,
    pub specification: RenderGraphImageSpecification
}

#[derive(Debug)]
pub struct RenderGraphOutputImage {
    pub usage: RenderGraphImageUsageId,
    pub specification: RenderGraphImageSpecification
}

//
// A collection of nodes and resources. Nodes represent an event or process that will occur at
// a certain time. Resources represent images and buffers that may be read/written by nodes.
//
#[derive(Default, Debug)]
pub struct RenderGraph {
    // Nodes that have been registered in the graph
    nodes: Vec<RenderGraphNode>,

    // Image resources that have been registered in the graph. These resources are "virtual" until
    // the graph is scheduled. In other words, we don't necessarily allocate an image for every
    // resource as some resources can share the same image internally if their lifetime don't
    // overlap. Additionally, a resource can be bound to input and output images. If this is the
    // case, we will try to use those images rather than creating new ones.
    image_resources: Vec<RenderGraphImageResource>,

    image_usages: Vec<RenderGraphImageUsage>,

    pub(super) input_images: Vec<RenderGraphInputImage>,
    pub(super) output_images: Vec<RenderGraphOutputImage>,
}

impl RenderGraph {
    pub(super) fn add_usage(
        &mut self,
        version: RenderGraphImageVersionId,
        usage_type: RenderGraphImageUsageType,
    ) -> RenderGraphImageUsageId {
        let usage_id = RenderGraphImageUsageId(self.image_usages.len());
        self.image_usages.push(RenderGraphImageUsage {
            usage_type,
            version,
        });
        usage_id
    }

    // Add an image that can be used by nodes
    pub(super) fn add_image_create(
        &mut self,
        create_node: RenderGraphNodeId,
        attachment_type: RenderGraphAttachmentType,
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        let version_id = RenderGraphImageVersionId {
            index: self.image_resources.len(),
            version: 0,
        };
        let usage_id = self.add_usage(version_id, RenderGraphImageUsageType::Create);

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
    ) -> RenderGraphImageUsageId {
        let version_id = self.image_usages[image.0].version;

        let usage_id = self.add_usage(version_id, RenderGraphImageUsageType::Read);

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
    ) -> (RenderGraphImageUsageId, RenderGraphImageUsageId) {
        let read_version_id = self.image_usages[image.0].version;

        let read_usage_id = self.add_usage(
            read_version_id,
            RenderGraphImageUsageType::ModifyRead,
        );

        self.image_resources[read_version_id.index].versions[read_version_id.version]
            .add_read_usage(read_usage_id);

        // Create a new version and add it to the image
        let version = self.image_resources[read_version_id.index].versions.len();

        let write_version_id = RenderGraphImageVersionId {
            index: read_version_id.index,
            version,
        };
        let write_usage_id =
            self.add_usage(write_version_id, RenderGraphImageUsageType::ModifyWrite);

        let mut version_info = RenderGraphImageResourceVersionInfo::new(modify_node, write_usage_id);
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
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        let attachment_type = RenderGraphAttachmentType::Color(color_attachment_index);

        // Add the read to the graph
        let create_image = self.add_image_create(node, attachment_type, constraint);

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
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        let attachment_type = RenderGraphAttachmentType::Depth;

        // Add the read to the graph
        let create_image = self.add_image_create(node, attachment_type, constraint);

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Create,
                clear_depth_stencil_value,
                read_image: None,
                write_image: Some(create_image),
            },
        );

        create_image
    }

    pub fn create_resolve_attachment(
        &mut self,
        node: RenderGraphNodeId,
        resolve_attachment_index: usize,
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        let attachment_type = RenderGraphAttachmentType::Resolve(resolve_attachment_index);

        let create_image = self.add_image_create(node, attachment_type, constraint);

        self.set_resolve_attachment(
            node,
            resolve_attachment_index,
            RenderGraphPassResolveAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Create,
                write_image: Some(create_image),
            },
        );

        create_image
    }

    pub fn read_color_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        color_attachment_index: usize,
        constraint: RenderGraphImageConstraint,
    ) {
        let attachment_type = RenderGraphAttachmentType::Color(color_attachment_index);

        // Add the read to the graph
        let read_image = self.add_image_read(node, image, attachment_type, constraint);

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
        constraint: RenderGraphImageConstraint,
    ) {
        let attachment_type = RenderGraphAttachmentType::Depth;

        // Add the read to the graph
        let read_image = self.add_image_read(node, image, attachment_type, constraint);

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Read,
                clear_depth_stencil_value: None,
                read_image: Some(read_image),
                write_image: None,
            },
        );
    }

    pub fn modify_color_attachment(
        &mut self,
        node: RenderGraphNodeId,
        image: RenderGraphImageUsageId,
        color_attachment_index: usize,
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        let attachment_type = RenderGraphAttachmentType::Color(color_attachment_index);

        // Add the read to the graph
        let (read_image, write_image) =
            self.add_image_modify(node, image, attachment_type, constraint);

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
        constraint: RenderGraphImageConstraint,
    ) -> RenderGraphImageUsageId {
        let attachment_type = RenderGraphAttachmentType::Depth;

        // Add the read to the graph
        let (read_image, write_image) =
            self.add_image_modify(node, image, attachment_type, constraint);

        self.set_depth_attachment(
            node,
            RenderGraphPassDepthAttachmentInfo {
                attachment_type: RenderGraphPassAttachmentType::Modify,
                clear_depth_stencil_value: None,
                read_image: Some(read_image),
                write_image: Some(write_image),
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

    // https://en.wikipedia.org/wiki/Topological_sorting#Depth-first_search
    // We
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
            println!("Found cycle in graph");
            println!("{:?}", self.node(node_id));
            for v in visiting_stack.iter().rev() {
                println!("{:?}", self.node(*v));
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
        //println!("  Begin visit {:?}", node_id);
        let node = self.node(node_id);

        //TODO: Does the order of visiting here matter? If we consider merging subpasses, trying to
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
        // by some sort of flagging/priority system to influence this logic. An end-user could
        // also use arbitrary dependencies the do nothing but influence the ordering here.
        //
        // As a first pass implementation, just ensure any merge dependencies are visited last so
        // that we will be more likely to be able to merge passes

        //TODO: Could we recurse to generate a bitset that is returned indicating all dependencies
        // and then O(n^2) try to dequeue them all in priority order?

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
        //println!("  End visit {:?}", node_id);
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

        // Iterate all the images we need to output. This will visit all the nodes we to execute,
        // potentially leaving out nodes we can cull.
        for output_image_id in &self.output_images {
            // Find the node that creates the output image
            let output_node = self.image_version_info(output_image_id.usage).creator_node;
            println!(
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

    fn merge_passes(
        &mut self,
        node_execution_order: &[RenderGraphNodeId],
    ) {
        for i in 0..node_execution_order.len() - 1 {
            let prev = RenderGraphNodeId(i);
            let next = RenderGraphNodeId(i + 1);

            if self.can_passes_merge(prev, next) {
                // merge - mainly push next's ID onto previous's merge list
            }
        }
    }

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

    /*
    fn determine_images_in_use(
        &self,
        node_execution_order: &[RenderGraphNodeId],
    ) -> FnvHashSet<RenderGraphImageResourceId> {
        let mut image_versions_in_use = FnvHashSet::default();
        //let mut images_in_use = vec![false; self.image_resources.len()];
        for node in node_execution_order {
            let node = self.node(*node);
            for create in &node.image_creates {
                image_versions_in_use.insert(create.image);
                //images_in_use[create.image.index] = true;
            }

            for read in &node.image_reads {
                image_versions_in_use.insert(read.image);
                //images_in_use[read.image.index] = true;
            }

            for modify in &node.image_modifies {
                image_versions_in_use.insert(modify.input);
                //images_in_use[modify.input.index] = true;

                image_versions_in_use.insert(modify.output);
                //images_in_use[modify.output.index] = true;
            }
        }

        //TODO: Should I add output images too?
        //TODO: This is per usage

        // println!("Images in use:");
        // for (image_index, _) in images_in_use.iter().enumerate() {
        //     println!("  Image {:?}", self.image_resources[image_index]);
        // }

        image_versions_in_use
    }
    */

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

        println!("Propagating image constraints");

        println!("  Set up input images");

        //
        // Propagate input image state specifications into images. Inputs are fully specified and
        // their constraints will never be overwritten
        //
        for input_image in &self.input_images {
            println!(
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

        println!("  Propagate image constraints FORWARD");

        //
        // Iterate forward through nodes to determine what states images need to be in. We only need
        // to handle operations that produce a new version of a resource. These operations do not
        // need to fully specify image info, but whatever they do specify will be carried forward
        // and not overwritten
        //
        for node_id in node_execution_order.iter() {
            let node = self.node(*node_id);
            println!("    node {:?} {:?}", node_id, node.name());

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

                println!(
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

                println!(
                    "        Forward propagate constraints {:?} {:?}",
                    image_create.image, version_state
                );

                // Don't bother setting usage constraint for 0
            }

            // We don't need to propagate anything forward on reads

            //
            // Propagate constraints forward for images being modified.
            //
            for image_modify in &node.image_modifies {
                println!(
                    "      Modify image {:?} {:?} -> {:?} {:?}",
                    image_modify.input,
                    self.image_resource(image_modify.input).name,
                    image_modify.output,
                    self.image_resource(image_modify.output).name
                );

                let image = self.image_version_info(image_modify.input);
                //println!("  Modify image {:?} {:?}", image_modify.input, self.image_resource(image_modify.input).name);
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
                    println!("        *** Found required fixup: {:?}", required_fixup);
                    println!("            {:?}", input_state.constraint);
                    println!("            {:?}", image_modify_constraint);
                    required_fixups.push(required_fixup);
                    */
                    //println!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", input_state.combined_constraints, image_modify.constraint);
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
                    println!("        *** Found required fixup {:?}", required_fixup);
                    println!("            {:?}", image_modify_constraint);
                    println!("            {:?}", output_state.constraint);
                    required_fixups.push(required_fixup);
                    */
                    //println!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", output_state.constraint, input_state.constraint);
                }

                println!("        Forward propagate constraints {:?}", output_state);
            }
        }

        println!("  Set up output images");

        //
        // Propagate output image state specifications into images
        //
        for output_image in &self.output_images {
            println!(
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
                println!("      *** Found required OUTPUT fixup");
                println!(
                    "          {:?}",
                    output_image_version_state //.combined_constraints
                );
                println!("          {:?}", output_image.specification);
                //println!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", output_image_version_state.constraint, output_specification);
            }

            image_version_states.insert(output_image.usage, output_image.specification.clone().into());
        }

        println!("  Propagate image constraints BACKWARD");

        //
        // Iterate backwards through nodes, determining the state the image must be in at every
        // step
        //
        for node_id in node_execution_order.iter().rev() {
            let node = self.node(*node_id);
            println!("    node {:?} {:?}", node_id, node.name());

            // Don't need to worry about creates, we back propagate to them when reading/modifying
            // // Propagate backwards into creates (in case they weren't fully specified)
            // for image_create in &node.image_creates {
            //     let image = self.image_version_info(image_create.image);
            //     println!("  Create image {:?} {:?}", image_create.image, self.image_resource(image_create.image).name);
            //     let mut version_state = &mut image_version_states[image_create.image.index][image_create.image.version];
            //     if !version_state.constraint.partial_merge(&image_create.constraint) {
            //         // Note this to handle later?
            //         panic!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", version_state.constraint, image_create.constraint);
            //     }
            // }

            //
            // Propagate backwards from reads
            //
            for image_read in &node.image_reads {
                println!(
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
                    println!("        *** Found required READ fixup");
                    println!(
                        "            {:?}",
                        version_state /*.combined_constraints*/
                    );
                    println!("            {:?}", image_read.constraint);
                    //println!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", version_state.constraint, image_read.constraint);
                }

                // If this is an image read with no output, it's possible the constraint on the read is incomplete.
                // So we need to merge the image state that may have information forward-propagated
                // into it with the constraints on the read. (Conceptually it's like we're forward
                // propagating here because the main forward propagate pass does not handle reads.
                // TODO: We could consider moving this to the forward pass
                let mut image_read_constraint = image_read.constraint.clone();
                image_read_constraint.partial_merge(&version_state /*.combined_constraints*/);
                println!(
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
                println!(
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
                    println!("        *** Found required MODIFY fixup");
                    println!(
                        "            {:?}",
                        input_state /*.combined_constraints*/
                    );
                    println!("            {:?}", image_modify.constraint);
                    //println!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", input_state.constraint, image_modify.constraint);
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
        println!("Insert resolves in graph where necessary");
        for node_id in node_execution_order {
            let mut resolves_to_add = Vec::default();

            let node = self.node(*node_id);
            println!("  node {:?}", node_id);
            // Iterate through all color attachments
            for (color_attachment_index, color_attachment) in
                node.color_attachments.iter().enumerate()
            {
                if let Some(color_attachment) = color_attachment {
                    println!("    color attachment {}", color_attachment_index);
                    // If this color attachment outputs an image
                    if let Some(write_image) = color_attachment.write_image {
                        let write_version = self.image_usages[write_image.0].version;
                        // Skip if it's not an MSAA image
                        let write_spec = image_constraint_results.specification(write_image);
                        if write_spec.samples == vk::SampleCountFlags::TYPE_1 {
                            println!("      already non-MSAA");
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
                            println!("      usage {}", usage_index);

                            let read_spec = image_constraint_results.specification(*read_usage);
                            if resolve_spec == *read_spec {
                                usages_to_move.push(*read_usage);
                                break;
                            } else {
                                println!("        incompatibility cannot be fixed via renderpass resolve");
                                println!("{:?}", resolve_spec);
                                println!("{:?}", read_spec);
                            }
                        }

                        if !usages_to_move.is_empty() {
                            resolves_to_add.push((color_attachment_index, resolve_spec, usages_to_move));
                        }
                    }
                }
            }

            for (resolve_attachment_index, resolve_spec, usages_to_move) in resolves_to_add {
                println!(
                    "ADDING RESOLVE FOR NODE {:?} ATTACHMENT {}",
                    node_id, resolve_attachment_index
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
                    //let from = self.image_usages[usage.0].version;
                    //self.image_version_info_mut(from).
                    self.move_read_usage_to_image(
                        usage,
                        self.image_usages[usage.0].version,
                        self.image_usages[image.0].version
                    )
                }
            }
        }
    }

    fn move_read_usage_to_image(
        &mut self,
        usage: RenderGraphImageUsageId,
        from: RenderGraphImageVersionId,
        to: RenderGraphImageVersionId
    ) {
        println!("MOVE USAGE {:?} from {:?} to {:?}", usage, from, to);
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
        let mut physical_image_usages: FnvHashMap<PhysicalImageId, Vec<RenderGraphImageUsageId>> =
            FnvHashMap::default();
        let mut image_allocator = PhysicalImageAllocator::default();
        //TODO: Associate input images here? We can wait until we decide which images are shared
        println!("Associate images written by nodes with physical images");
        for node in node_execution_order.iter() {
            let node = self.node(*node);
            println!("  node {:?} {:?}", node.id().0, node.name());

            // A list of all images we write to from this node. We will try to share the images
            // being written forward into the nodes of downstream reads. This can chain such that
            // the same image is shared by many nodes
            let mut written_images = vec![];

            for create in &node.image_creates {
                // An image that's created always allocates an image (we can try to alias/pool these later)
                let physical_image =
                    image_allocator.allocate(&image_constraint_results.specification(create.image));
                println!(
                    "    Create {:?} will use image {:?}",
                    create.image, physical_image
                );
                map_image_to_physical.insert(create.image, physical_image);
                physical_image_usages
                    .entry(physical_image)
                    .or_default()
                    .push(create.image);

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
                println!(
                    "    Modify {:?} will pass through image {:?}",
                    modify.output, physical_image
                );
                map_image_to_physical.insert(modify.output, physical_image);
                physical_image_usages
                    .entry(physical_image)
                    .or_default()
                    .push(modify.output);

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
                    let specifications_match = image_constraint_results
                        .specification(written_image)
                        == image_constraint_results.specification(*usage_resource_id);

                    // We can't share images unless it's a read or it's an exclusive write
                    let is_read_or_exclusive_write =
                        (read_count > 0 && self.image_usages[usage_resource_id.0].usage_type.is_read_only()) || write_count <= 1;

                    let read_type =
                        self.image_usages[usage_resource_id.0].usage_type;
                    if specifications_match && is_read_or_exclusive_write {
                        // it's a shared read or an exclusive write
                        println!(
                            "    Usage {:?} will share an image with {:?} ({:?} -> {:?})",
                            written_image, usage_resource_id, write_type, read_type
                        );
                        let overwritten_image =
                            map_image_to_physical.insert(*usage_resource_id, write_physical_image);
                        physical_image_usages
                            .entry(write_physical_image)
                            .or_default()
                            .push(*usage_resource_id);
                        assert!(overwritten_image.is_none());
                    } else {
                        // allocate new image
                        let specification = image_constraint_results.specification(written_image);
                        let physical_image = image_allocator.allocate(&specification);
                        println!(
                            "    Allocate image {:?} for {:?} ({:?} -> {:?})",
                            physical_image, usage_resource_id, write_type, read_type
                        );
                        let overwritten_image =
                            map_image_to_physical.insert(*usage_resource_id, physical_image);
                        physical_image_usages
                            .entry(physical_image)
                            .or_default()
                            .push(*usage_resource_id);
                        assert!(overwritten_image.is_none());
                    }
                }
            }
        }

        AssignPhysicalImagesResult {
            physical_image_usages,
            map_image_to_physical,
        }
    }

    fn collect_read_requirements(
        &self,
        node_execution_order: &[RenderGraphNodeId],
        image_constraints: &DetermineImageConstraintsResult,
        physical_images: &AssignPhysicalImagesResult,
    ) {
    }

    fn record_nodes(
        &self,
        node_execution_order: &[RenderGraphNodeId],
        image_constraints: &DetermineImageConstraintsResult,
        physical_images: &AssignPhysicalImagesResult,
    ) {
        for node_id in node_execution_order {
            let node = self.node(*node_id);
            println!("record {:?}", node);

            for (index, attachment_info) in node.color_attachments.iter().enumerate() {
                let attachment_info = attachment_info.as_ref().unwrap();

                // Modifies have two images, but the spec will be the same for both of them. The
                // algorithm that determines spec must ensure this because while we have distinct
                // read/write IDs for the single modify, in the end it must be on a single physical
                // image.
                let image = attachment_info
                    .read_image
                    .or(attachment_info.write_image)
                    .unwrap();
                let physical_image = physical_images.map_image_to_physical.get(&image);
                let specification = image_constraints.specification(image);

                let attachment = RenderGraph::create_attachment_description(specification, attachment_info.attachment_type, attachment_info.clear_color_value.is_some());

                println!("  Color Attachment {}: {:?}", index, attachment);
            }

            for (index, attachment_info) in node.resolve_attachments.iter().enumerate() {
                let attachment_info = attachment_info.as_ref().unwrap();

                // Modifies have two images, but the spec will be the same for both of them. The
                // algorithm that determines spec must ensure this because while we have distinct
                // read/write IDs for the single modify, in the end it must be on a single physical
                // image.
                let image = attachment_info.write_image.unwrap();
                let physical_image = physical_images.map_image_to_physical.get(&image);
                let specification = image_constraints.specification(image);

                let attachment = RenderGraph::create_attachment_description(specification, attachment_info.attachment_type, false);

                println!("  Resolve Attachment {}: {:?}", index, attachment);
            }

            if let Some(attachment_info) = &node.depth_attachment {
                // Modifies have two images, but the spec will be the same for both of them. The
                // algorithm that determines spec must ensure this because while we have distinct
                // read/write IDs for the single modify, in the end it must be on a single physical
                // image.
                let image = attachment_info
                    .read_image
                    .or(attachment_info.write_image)
                    .unwrap();
                let physical_image = physical_images.map_image_to_physical.get(&image);
                let specification = image_constraints.specification(image);

                let attachment = RenderGraph::create_attachment_description(specification, attachment_info.attachment_type, attachment_info.clear_depth_stencil_value.is_some());

                println!("  Depth Attachment: {:?}", attachment);
            }
        }
    }

    fn create_attachment_description(
        //attachment_info: &RenderGraphPassColorAttachmentInfo,
        specification: &RenderGraphImageSpecification,
        attachment_type: RenderGraphPassAttachmentType,
        has_clear_color: bool
    ) -> dsc::AttachmentDescription {
        let flags = dsc::AttachmentDescriptionFlags::None;
        // TODO: Look up if aliasing
        let format = specification.format;
        let samples = specification.samples;
        let (load_op, store_op) = match attachment_type {
            RenderGraphPassAttachmentType::Create => {
                if has_clear_color {
                    (dsc::AttachmentLoadOp::Clear, dsc::AttachmentStoreOp::Store)
                } else {
                    (
                        dsc::AttachmentLoadOp::DontCare,
                        dsc::AttachmentStoreOp::Store,
                    )
                }
            }
            RenderGraphPassAttachmentType::Read => (
                dsc::AttachmentLoadOp::Load,
                dsc::AttachmentStoreOp::DontCare,
            ),
            RenderGraphPassAttachmentType::Modify => {
                (dsc::AttachmentLoadOp::Load, dsc::AttachmentStoreOp::Store)
            }
        };
        let stencil_load_op = dsc::AttachmentLoadOp::DontCare;
        let stencil_store_op = dsc::AttachmentStoreOp::DontCare;
        //TODO: Can we set DONT_CARE on an output if the downstream readers are culled?
        let attachment = dsc::AttachmentDescription {
            flags: dsc::AttachmentDescriptionFlags::None,
            format: dsc::AttachmentFormat::Format(format.into()),
            samples: dsc::SampleCountFlags::from_vk_sample_count_flags(samples).unwrap(),
            load_op,
            store_op,
            stencil_load_op,
            stencil_store_op,
            initial_layout: dsc::ImageLayout::ColorAttachmentOptimal,
            final_layout: dsc::ImageLayout::ColorAttachmentOptimal,
            // pub flags: AttachmentDescriptionFlags,
            // pub format: AttachmentFormat,
            // pub samples: SampleCountFlags,
            // pub load_op: AttachmentLoadOp,
            // pub store_op: AttachmentStoreOp,
            // pub stencil_load_op: AttachmentLoadOp,
            // pub stencil_store_op: AttachmentStoreOp,
            // pub initial_layout: ImageLayout,
            // pub final_layout: ImageLayout,
        };
        attachment
    }

    pub fn prepare(&mut self) {
        //
        // Walk backwards through the DAG, starting from the output images, through all the upstream
        // dependencies of those images. We are doing a depth first search. Nodes that make no
        // direct or indirect contribution to an output image will not be included
        //
        let node_execution_order = self.determine_node_order();

        // Print out the execution order
        println!("Execution order of unculled nodes:");
        for node in &node_execution_order {
            println!("  Node {:?} {:?}", node, self.node(*node).name());
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
        // Pull all the node actions out of nodes and prepare them. This allows them to look ahead
        // at what they will be writing to and potentially adjust how they output. If nodes are not
        // able to adjust to accomodate future nodes we will have to insert operations between node
        // actions.
        //
        // The following would be supported here:
        // - Renderpasses can add a resolve as the end to switch sample count
        // - Renderpasses can detect the next node is a compatible renderpass and merge it
        //

        //TODO: We must adjust the graph here to convert modifies to read/create when resolving
        //TODO: Need to add in functionality to splice in new nodes and replace images, not just so
        // we can change the graph here but also for downstream code to non-intrusively modify a
        // graph
        self.insert_resolves(&node_execution_order, &mut image_constraint_results);

        //self.merge_passes(&node_execution_order);

        // Print the cases where we can't reuse images
        self.print_image_compatibility(&image_constraint_results);

        //graph.replace_image(before, after)

        //
        // Assign logical images to physical images. This should give us a minimal number of images.
        // This does not include aliasing images during graph execution. We handle this later.
        //
        let assign_physical_images_result =
            self.assign_physical_images(&node_execution_order, &mut image_constraint_results);
        println!("Physical image usage:");
        for (physical_image_id, logical_image_id_list) in
            &assign_physical_images_result.physical_image_usages
        {
            println!("  Physical image: {:?}", physical_image_id);
            for logical_image in logical_image_id_list {
                println!("    {:?}", logical_image);
            }
        }

        let read_requirements = self.collect_read_requirements(
            &node_execution_order,
            &image_constraint_results,
            &assign_physical_images_result,
        );

        // At some point need to walk through nodes to place barriers and see about aliasing/pooling images

        self.record_nodes(
            &node_execution_order,
            &image_constraint_results,
            &assign_physical_images_result,
        );

        //
        // Execute the graph
        //
        println!("-------------- EXECUTE --------------");

        for (node_index, node) in self.nodes.iter().enumerate() {
            println!("Record node {:?} {:?}", node.id(), node.name());
            // if let Some(action) = node_actions[node_index].take() {
            //     action.record(self, &image_constraint_results);
            // }

            // Any output may need to be adjusted
            for create in &node.image_creates {
                println!("Process create {:?}", create);
                let reader_count = self.image_version_info(create.image).read_usages.len();
                println!("  read count: {}", reader_count);
            }

            // Any output may need to be adjusted
            for modify in &node.image_modifies {
                println!("Process modify {:?}", modify);
                let reader_count = self.image_version_info(modify.output).read_usages.len();
                println!("  read count: {}", reader_count);
            }
        }

        // * Traverse graphs and figure out all read/write requirements
        // * See if writers are able to change output to match their readers
        // * See if writers are able to merge with readers (must be single writer to single reader)
        //   - Don't bother with it right now
        // * Try to merge image usage?
        // *
        // * Allocate/reuse frame buffers?

        //
        // Find the lifetimes and sequence of events for all image resources
        //

        //
        // Merge passes? Make sure that if we merge A, B, C, that A ends up with all the reads/writes
        // from B and C. Or.. we can reorder to ensure mergable passes are next to each other
        //

        //
        // Optimize ordering to delay nodes that have dependencies that are nearby in the ordering.
        //

        //
        // Calculate physical images.
        //
        //TODO: If we are binding images to input/output names, we could have the same image bound
        // for both input and output. They need to have the same physical index. The other way
        // is we can use CLEAR/LOAD/STORE ops to know read/write and we bind via attachment name

        //
        // Merge render passes if not already done
        //

        //
        // Get rid of images that are only used between subpasses
        //

        //
        // Trace backwards to determine what states images should be in?
        //
        // Insert barriers? Modify passes? Alias images?
    }

    fn print_image_constraints(
        &self,
        image_constraint_results: &mut DetermineImageConstraintsResult,
    ) {
        println!("Image constraints:");
        for (image_index, image_resource) in self.image_resources.iter().enumerate() {
            println!("  Image {:?} {:?}", image_index, image_resource.name);
            for (version_index, version) in image_resource.versions.iter().enumerate() {
                println!("    Version {}", version_index);

                println!(
                    "      Writen as: {:?}",
                    image_constraint_results.specification(version.create_usage)
                );

                for (usage_index, usage) in version.read_usages.iter().enumerate() {
                    println!(
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
        println!("Image Compatibility Report:");
        for (image_index, image_resource) in self.image_resources.iter().enumerate() {
            println!("  Image {:?} {:?}", image_index, image_resource.name);
            for (version_index, version) in image_resource.versions.iter().enumerate() {
                let write_specification =
                    image_constraint_results.specification(version.create_usage);

                println!("    Version {}: {:?}", version_index, version);
                for (usage_index, usage) in version.read_usages.iter().enumerate() {
                    let read_specification = image_constraint_results.specification(*usage);

                    // TODO: Skip images we don't use?

                    if write_specification == read_specification {
                        println!("      read usage {} matches", usage_index);
                    } else {
                        println!("      read usage {} does not match", usage_index);
                        println!("        produced: {:?}", write_specification);
                        println!("        required: {:?}", read_specification);
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct PhysicalImageId(usize);

struct PhysicalImageInfo {
    specification: RenderGraphImageSpecification,
}

#[derive(Default)]
struct PhysicalImageAllocator {
    unused_images: FnvHashMap<RenderGraphImageSpecification, Vec<PhysicalImageId>>,
    allocated_images: Vec<PhysicalImageInfo>,
}

impl PhysicalImageAllocator {
    fn allocate(
        &mut self,
        specification: &RenderGraphImageSpecification,
    ) -> PhysicalImageId {
        if let Some(image) = self
            .unused_images
            .entry(specification.clone())
            .or_default()
            .pop()
        {
            image
        } else {
            let id = PhysicalImageId(self.allocated_images.len());
            self.allocated_images.push(PhysicalImageInfo {
                specification: specification.clone(),
            });
            id
        }
    }
}
