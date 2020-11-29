use super::*;
use super::{RenderGraphImageSpecification, RenderGraphOutputImageId};
use crate::graph::graph_image::{PhysicalImageId, RenderGraphImageUser, VirtualImageId};
use crate::graph::graph_node::RenderGraphNodeId;
use crate::graph::{RenderGraphBuilder, RenderGraphImageConstraint, RenderGraphImageUsageId};
use crate::vk_description as dsc;
use crate::vk_description::SwapchainSurfaceInfo;
use crate::{ImageViewResource, ResourceArc};
use ash::vk;
use fnv::{FnvHashMap, FnvHashSet};
use std::sync::Arc;

/// The specification for the image by image usage
pub struct DetermineImageConstraintsResult {
    images: FnvHashMap<RenderGraphImageUsageId, RenderGraphImageSpecification>,
}

impl DetermineImageConstraintsResult {
    pub fn specification(
        &self,
        image: RenderGraphImageUsageId,
    ) -> Option<&RenderGraphImageSpecification> {
        self.images.get(&image)
    }
}

/// Assignment of usages to actual images. This allows a single image to be passed through a
/// sequence of reads and writes
#[derive(Debug)]
pub struct AssignVirtualImagesResult {
    usage_to_virtual: FnvHashMap<RenderGraphImageUsageId, VirtualImageId>,
}

#[derive(Default)]
struct VirtualImageIdAllocator {
    next_id: usize,
}

impl VirtualImageIdAllocator {
    fn allocate(&mut self) -> VirtualImageId {
        let id = VirtualImageId(self.next_id);
        self.next_id += 1;
        id
    }
}

// Recursively called to topologically sort the nodes to determine execution order
// https://en.wikipedia.org/wiki/Topological_sorting#Depth-first_search
fn visit_node(
    graph: &RenderGraphBuilder,
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
        log::trace!("Found cycle in graph");
        log::trace!("{:?}", graph.node(node_id));
        for v in visiting_stack.iter().rev() {
            log::trace!("{:?}", graph.node(*v));
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
    //log::trace!("  Begin visit {:?}", node_id);
    let node = graph.node(node_id);

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
            let upstream_node = graph.image_version_info(read_image).creator_node;

            // This might be too expensive to check
            if can_passes_merge(graph, upstream_node, node.id()) {
                merge_candidates.insert(upstream_node);
            }
        }
    }

    for color_attachment in &node.color_attachments {
        // If this is an attachment we are reading, then the node that created it is a merge candidate
        if let Some(read_image) = color_attachment.as_ref().and_then(|x| x.read_image) {
            let upstream_node = graph.image_version_info(read_image).creator_node;

            // This might be too expensive to check
            if can_passes_merge(graph, upstream_node, node.id()) {
                merge_candidates.insert(upstream_node);
            }
        }
    }

    //
    // Visit all the nodes we aren't delaying
    //
    for read in &node.image_reads {
        let upstream_node = graph.image_version_info(read.image).creator_node;
        if !merge_candidates.contains(&upstream_node) {
            visit_node(
                graph,
                upstream_node,
                visited,
                visiting,
                visiting_stack,
                ordered_list,
            );
        }
    }

    for modify in &node.image_modifies {
        let upstream_node = graph.image_version_info(modify.input).creator_node;
        if !merge_candidates.contains(&upstream_node) {
            visit_node(
                graph,
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
        visit_node(
            graph,
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
    //log::trace!("  End visit {:?}", node_id);
    visiting_stack.pop();
    visiting[node_id.0] = false;
}

#[profiling::function]
fn determine_node_order(graph: &RenderGraphBuilder) -> Vec<RenderGraphNodeId> {
    // As we depth-first traverse nodes, mark them as visiting and push them onto this stack.
    // We will use this to detect and print out cycles
    let mut visiting = vec![false; graph.nodes.len()];
    let mut visiting_stack = Vec::default();

    // The order of nodes, upstream to downstream. As we depth-first traverse nodes, push nodes
    // with no unvisited dependencies onto this list and mark them as visited
    let mut visited = vec![false; graph.nodes.len()];
    let mut ordered_list = Vec::default();

    // Iterate all the images we need to output. This will visit all the nodes we need to execute,
    // potentially leaving out nodes we can cull.
    for output_image_id in &graph.output_images {
        // Find the node that creates the output image
        let output_node = graph.image_version_info(output_image_id.usage).creator_node;
        log::trace!(
            "Traversing dependencies of output image created by node {:?} {:?}",
            output_node,
            graph.node(output_node).name()
        );

        visit_node(
            graph,
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
    _graph: &RenderGraphBuilder,
    _prev: RenderGraphNodeId,
    _next: RenderGraphNodeId,
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

#[profiling::function]
fn determine_image_constraints(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
) -> DetermineImageConstraintsResult {
    let mut image_version_states: FnvHashMap<RenderGraphImageUsageId, RenderGraphImageConstraint> =
        Default::default();

    log::trace!("Propagating image constraints");

    log::trace!("  Set up input images");

    //
    // Propagate input image state specifications into images. Inputs are fully specified and
    // their constraints will never be overwritten
    //
    for input_image in &graph.input_images {
        log::trace!(
            "    Image {:?} {:?}",
            input_image,
            graph.image_resource(input_image.usage).name
        );
        image_version_states
            .entry(graph.get_create_usage(input_image.usage))
            .or_default()
            .set(&input_image.specification);

        // Don't bother setting usage constraint for 0
    }

    log::trace!("  Propagate image constraints FORWARD");

    //
    // Iterate forward through nodes to determine what states images need to be in. We only need
    // to handle operations that produce a new version of a resource. These operations do not
    // need to fully specify image info, but whatever they do specify will be carried forward
    // and not overwritten
    //
    for node_id in node_execution_order.iter() {
        let node = graph.node(*node_id);
        log::trace!("    node {:?} {:?}", node_id, node.name());

        //
        // Propagate constraints into images this node creates.
        //
        for image_create in &node.image_creates {
            // An image cannot be created within the graph and imported externally at the same
            // time. (The code here assumes no input and will not produce correct results if there
            // is an input image)
            //TODO: Input images are broken, we don't properly represent an image being created
            // OR receiving an input. We probably need to make creator in
            // RenderGraphImageResourceVersionInfo Option or an enum with input/create options
            //assert!(graph.image_version_info(image_create.image).input_image.is_none());

            log::trace!(
                "      Create image {:?} {:?}",
                image_create.image,
                graph.image_resource(image_create.image).name
            );

            let version_state = image_version_states
                .entry(graph.get_create_usage(image_create.image))
                .or_default();

            if !version_state.try_merge(&image_create.constraint) {
                // Should not happen as this should be our first visit to this image
                panic!("Unexpected constraints on image being created");
            }

            log::trace!(
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
            log::trace!(
                "      Modify image {:?} {:?} -> {:?} {:?}",
                image_modify.input,
                graph.image_resource(image_modify.input).name,
                image_modify.output,
                graph.image_resource(image_modify.output).name
            );

            //let image = graph.image_version_info(image_modify.input);
            //log::trace!("  Modify image {:?} {:?}", image_modify.input, graph.image_resource(image_modify.input).name);
            let input_state = image_version_states
                .entry(graph.get_create_usage(image_modify.input))
                .or_default();
            let mut image_modify_constraint = image_modify.constraint.clone();

            // Merge the input image constraints with this node's constraints
            if !image_modify_constraint.partial_merge(&input_state /*.combined_constraints*/) {
                // This would need to be resolved by inserting some sort of fixup

                // We will detect this on the backward pass, no need to do anything here
                /*
                let required_fixup = ImageConstraintRequiredFixup::Modify(node.id(), image_modify.clone());
                log::trace!("        *** Found required fixup: {:?}", required_fixup);
                log::trace!("            {:?}", input_state.constraint);
                log::trace!("            {:?}", image_modify_constraint);
                required_fixups.push(required_fixup);
                */
                //log::trace!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", input_state.combined_constraints, image_modify.constraint);
            }

            //TODO: Should we set the usage constraint here? For now will wait until backward propagation

            let output_state = image_version_states
                .entry(graph.get_create_usage(image_modify.output))
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
                log::trace!("        *** Found required fixup {:?}", required_fixup);
                log::trace!("            {:?}", image_modify_constraint);
                log::trace!("            {:?}", output_state.constraint);
                required_fixups.push(required_fixup);
                */
                //log::trace!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", output_state.constraint, input_state.constraint);
            }

            log::trace!("        Forward propagate constraints {:?}", output_state);
        }
    }

    log::trace!("  Set up output images");

    //
    // Propagate output image state specifications into images
    //
    for output_image in &graph.output_images {
        log::trace!(
            "    Image {:?} {:?}",
            output_image,
            graph.image_resource(output_image.usage).name
        );
        let output_image_version_state = image_version_states
            .entry(graph.get_create_usage(output_image.usage))
            .or_default();
        let output_constraint = output_image.specification.clone().into();
        if !output_image_version_state.partial_merge(&output_constraint) {
            // This would need to be resolved by inserting some sort of fixup
            /*
            log::trace!("      *** Found required OUTPUT fixup");
            log::trace!(
                "          {:?}",
                output_image_version_state //.combined_constraints
            );
            log::trace!("          {:?}", output_image.specification);
            */
            //log::trace!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", output_image_version_state.constraint, output_specification);
        }

        image_version_states.insert(
            output_image.usage,
            output_image.specification.clone().into(),
        );
    }

    log::trace!("  Propagate image constraints BACKWARD");

    //
    // Iterate backwards through nodes, determining the state the image must be in at every
    // step
    //
    for node_id in node_execution_order.iter().rev() {
        let node = graph.node(*node_id);
        log::trace!("    node {:?} {:?}", node_id, node.name());

        // Don't need to worry about creates, we back propagate to them when reading/modifying

        //
        // Propagate backwards from reads
        //
        for image_read in &node.image_reads {
            log::trace!(
                "      Read image {:?} {:?}",
                image_read.image,
                graph.image_resource(image_read.image).name
            );

            let version_state = image_version_states
                .entry(graph.get_create_usage(image_read.image))
                .or_default();
            if !version_state
                //.combined_constraints
                .partial_merge(&image_read.constraint)
            {
                // This would need to be resolved by inserting some sort of fixup
                /*
                log::trace!("        *** Found required READ fixup");
                log::trace!(
                    "            {:?}",
                    version_state /*.combined_constraints*/
                );
                log::trace!("            {:?}", image_read.constraint);
                */
                //log::trace!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", version_state.constraint, image_read.constraint);
            }

            // If this is an image read with no output, it's possible the constraint on the read is incomplete.
            // So we need to merge the image state that may have information forward-propagated
            // into it with the constraints on the read. (Conceptually it's like we're forward
            // propagating here because the main forward propagate pass does not handle reads.
            // TODO: We could consider moving this to the forward pass
            let mut image_read_constraint = image_read.constraint.clone();
            image_read_constraint.partial_merge(&version_state /*.combined_constraints*/);
            log::trace!(
                "        Read constraints will be {:?}",
                image_read_constraint
            );
            if let Some(spec) = image_read_constraint.try_convert_to_specification() {
                image_version_states.insert(image_read.image, spec.into());
            } else {
                panic!(
                    "Not enough information in the graph to determine the specification for image {:?} {:?} being read by node {:?} {:?}. Constraints are: {:?}",
                    image_read.image,
                    graph.image_resource(image_read.image).name,
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
            log::trace!(
                "      Modify image {:?} {:?} <- {:?} {:?}",
                image_modify.input,
                graph.image_resource(image_modify.input).name,
                image_modify.output,
                graph.image_resource(image_modify.output).name
            );
            // The output image constraint already takes image_modify.constraint into account from
            // when we propagated image constraints forward
            let output_image_constraint = image_version_states
                .entry(graph.get_create_usage(image_modify.output))
                .or_default()
                .clone();
            let input_state = image_version_states
                .entry(graph.get_create_usage(image_modify.input))
                .or_default();
            if !input_state.partial_merge(&output_image_constraint) {
                // This would need to be resolved by inserting some sort of fixup
                /*
                log::trace!("        *** Found required MODIFY fixup");
                log::trace!(
                    "            {:?}",
                    input_state /*.combined_constraints*/
                );
                log::trace!("            {:?}", image_modify.constraint);
                */
                //log::trace!("Image cannot be placed into a form that satisfies all constraints:\n{:#?}\n{:#?}", input_state.constraint, image_modify.constraint);
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

#[profiling::function]
fn insert_resolves(
    graph: &mut RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    image_constraint_results: &mut DetermineImageConstraintsResult,
) {
    log::trace!("Insert resolves in graph where necessary");
    for node_id in node_execution_order {
        let mut resolves_to_add = Vec::default();

        let node = graph.node(*node_id);
        log::trace!("  node {:?}", node_id);
        // Iterate through all color attachments
        for (color_attachment_index, color_attachment) in node.color_attachments.iter().enumerate()
        {
            if let Some(color_attachment) = color_attachment {
                log::trace!("    color attachment {}", color_attachment_index);
                // If this color attachment outputs an image
                if let Some(write_image) = color_attachment.write_image {
                    //let write_version = graph.image_usages[write_image.0].version;
                    // Skip if it's not an MSAA image
                    let write_spec = image_constraint_results.specification(write_image).unwrap();
                    if write_spec.samples == vk::SampleCountFlags::TYPE_1 {
                        log::trace!("      already non-MSAA");
                        continue;
                    }

                    // Calculate the spec that we would have after the resolve
                    let mut resolve_spec = write_spec.clone();
                    resolve_spec.samples = vk::SampleCountFlags::TYPE_1;

                    let mut usages_to_move = vec![];

                    // Look for any usages we need to fix
                    for (usage_index, read_usage) in graph
                        .image_version_info(write_image)
                        .read_usages
                        .iter()
                        .enumerate()
                    {
                        log::trace!(
                            "      usage {}, {:?}",
                            usage_index,
                            graph.image_usages[read_usage.0].usage_type
                        );
                        let read_spec =
                            image_constraint_results.specification(*read_usage).unwrap();
                        if *read_spec == *write_spec {
                            continue;
                        } else if *read_spec == resolve_spec {
                            usages_to_move.push(*read_usage);
                            break;
                        } else {
                            log::trace!(
                                "        incompatibility cannot be fixed via renderpass resolve"
                            );
                            log::trace!("          resolve: {:?}", resolve_spec);
                            log::trace!("          read   : {:?}", read_spec);
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
            log::trace!(
                "        ADDING RESOLVE FOR NODE {:?} ATTACHMENT {}",
                node_id,
                resolve_attachment_index
            );
            let image = graph.create_resolve_attachment(
                *node_id,
                resolve_attachment_index,
                resolve_spec.clone().into(),
            );
            image_constraint_results.images.insert(image, resolve_spec);

            for usage in usages_to_move {
                let from = graph.image_usages[usage.0].version;
                let to = graph.image_usages[image.0].version;
                log::trace!(
                    "          MOVE USAGE {:?} from {:?} to {:?}",
                    usage,
                    from,
                    to
                );
                graph.move_read_usage_to_image(usage, from, to)
            }
        }
    }
}

#[profiling::function]
fn assign_virtual_images(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    image_constraint_results: &mut DetermineImageConstraintsResult,
) -> AssignVirtualImagesResult {
    let mut usage_to_virtual: FnvHashMap<RenderGraphImageUsageId, VirtualImageId> =
        FnvHashMap::default();

    let mut virtual_image_id_allocator = VirtualImageIdAllocator::default();
    //TODO: Associate input images here? We can wait until we decide which images are shared
    log::trace!("Associate images written by nodes with virtual images");
    for node in node_execution_order.iter() {
        let node = graph.node(*node);
        log::trace!("  node {:?} {:?}", node.id().0, node.name());

        // A list of all images we write to from this node. We will try to share the images
        // being written forward into the nodes of downstream reads. This can chain such that
        // the same image is shared by many nodes
        let mut written_images = vec![];

        for create in &node.image_creates {
            // An image that's created always allocates an image (we reuse these if they are compatible
            // and lifetimes don't overlap)
            let virtual_image = virtual_image_id_allocator.allocate();
            log::trace!(
                "    Create {:?} will use image {:?}",
                create.image,
                virtual_image
            );
            usage_to_virtual.insert(create.image, virtual_image);
            // Queue this image write to try to share the image forward
            written_images.push(create.image);
        }

        for modify in &node.image_modifies {
            // The virtual image in the read portion of a modify must also be the write image.
            // The format of the input/output is guaranteed to match
            assert_eq!(
                image_constraint_results.specification(modify.input),
                image_constraint_results.specification(modify.output)
            );

            // Assign the image
            let virtual_image = *usage_to_virtual.get(&modify.input).unwrap();
            log::trace!(
                "    Modify {:?} will pass through image {:?}",
                modify.output,
                virtual_image
            );
            usage_to_virtual.insert(modify.output, virtual_image);

            // Queue this image write to try to share the image forward
            written_images.push(modify.output);
        }

        for written_image in written_images {
            // Count the downstream users of this image based on if they need read-only access
            // or write access. We need this information to determine which usages we can share
            // the output data with.
            //TODO: This could be smarter to handle the case of a resource being read/written
            // in different lifetimes
            let written_image_version_info = graph.image_version_info(written_image);
            let mut read_count = 0;
            let mut write_count = 0;
            for usage in &written_image_version_info.read_usages {
                if graph.image_usages[usage.0].usage_type.is_read_only() {
                    read_count += 1;
                } else {
                    write_count += 1;
                }
            }

            // If we don't already have an image
            let write_virtual_image = *usage_to_virtual.get(&written_image).unwrap();
            let write_type = graph.image_usages[written_image.0].usage_type;

            for usage_resource_id in &written_image_version_info.read_usages {
                // We can't share images if they aren't the same format
                let written_spec = image_constraint_results
                    .specification(written_image)
                    .unwrap();
                let usage_spec = match image_constraint_results.specification(*usage_resource_id) {
                    Some(usage_spec) => usage_spec,
                    // If the reader of this image was culled, we may not have determined a spec.
                    // If so, skip this usage
                    None => continue,
                };

                let specifications_match = *written_spec == *usage_spec;

                // We can't share images unless it's a read or it's an exclusive write
                let is_read_or_exclusive_write = (read_count > 0
                    && graph.image_usages[usage_resource_id.0]
                        .usage_type
                        .is_read_only())
                    || write_count <= 1;

                let read_type = graph.image_usages[usage_resource_id.0].usage_type;
                if specifications_match && is_read_or_exclusive_write {
                    // it's a shared read or an exclusive write
                    log::trace!(
                        "    Usage {:?} will share an image with {:?} ({:?} -> {:?})",
                        written_image,
                        usage_resource_id,
                        write_type,
                        read_type
                    );
                    let overwritten_image =
                        usage_to_virtual.insert(*usage_resource_id, write_virtual_image);

                    assert!(overwritten_image.is_none());
                } else {
                    // allocate new image
                    let virtual_image = virtual_image_id_allocator.allocate();
                    log::trace!(
                        "    Allocate image {:?} for {:?} ({:?} -> {:?})  (specifications_match match: {} is_read_or_exclusive_write: {})",
                        virtual_image,
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
                        usage_to_virtual.insert(*usage_resource_id, virtual_image);

                    assert!(overwritten_image.is_none());

                    //TODO: One issue (aside from not doing any blits right now) is that images created in this way
                    // aren't included in the assign_physical_images logic
                    panic!("Render graph does not currently support blit from one image to another to fix image compatibility");
                }
            }
        }
    }

    // vulkan image layouts: https://github.com/nannou-org/nannou/issues/271#issuecomment-465876622
    AssignVirtualImagesResult {
        //physical_image_usages,
        usage_to_virtual,
        //physical_image_versions,
        //physical_image_infos,
    }
}

//TODO: Redundant with can_passes_merge
fn can_merge_nodes(
    graph: &RenderGraphBuilder,
    before_node_id: RenderGraphNodeId,
    after_node_id: RenderGraphNodeId,
    _image_constraints: &DetermineImageConstraintsResult,
    _virtual_images: &AssignVirtualImagesResult,
) -> bool {
    let _before_node = graph.node(before_node_id);
    let _after_node = graph.node(after_node_id);

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

#[profiling::function]
fn build_physical_passes(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    image_constraints: &DetermineImageConstraintsResult,
    virtual_images: &AssignVirtualImagesResult,
    //determine_image_layouts_result: &DetermineImageLayoutsResult
) -> Vec<RenderGraphPass> {
    let mut pass_node_sets = Vec::default();

    let mut subpass_nodes = Vec::default();
    for node_id in node_execution_order {
        let mut add_to_current = true;
        for subpass_node in &subpass_nodes {
            if !can_merge_nodes(
                graph,
                *subpass_node,
                *node_id,
                image_constraints,
                virtual_images,
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

    log::trace!("gather pass info");
    let mut passes = Vec::default();
    for pass_node_set in pass_node_sets {
        log::trace!("  nodes in pass: {:?}", pass_node_set);
        fn find_or_insert_attachment(
            attachments: &mut Vec<RenderGraphPassAttachment>,
            virtual_image: VirtualImageId,
        ) -> (usize, bool) {
            if let Some(position) = attachments
                .iter()
                .position(|x| x.virtual_image == virtual_image)
            {
                (position, false)
            } else {
                attachments.push(RenderGraphPassAttachment {
                    virtual_image,
                    image: None,
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
            pre_pass_barrier: Default::default(),
        };

        for node_id in pass_node_set {
            log::trace!("    subpass node: {:?}", node_id);
            let mut subpass = RenderGraphSubpass {
                node: node_id,
                color_attachments: Default::default(),
                resolve_attachments: Default::default(),
                depth_attachment: Default::default(),
            };

            let subpass_node = graph.node(node_id);

            for (color_attachment_index, color_attachment) in
                subpass_node.color_attachments.iter().enumerate()
            {
                if let Some(color_attachment) = color_attachment {
                    let read_or_write_usage = color_attachment
                        .read_image
                        .or(color_attachment.write_image)
                        .unwrap();
                    let virtual_image = virtual_images
                        .usage_to_virtual
                        .get(&read_or_write_usage)
                        .unwrap();
                    //let version_id = graph.image_version_id(read_or_write_usage);
                    let specification = image_constraints.images.get(&read_or_write_usage).unwrap();
                    log::trace!("      virtual attachment (color): {:?}", virtual_image);

                    let (pass_attachment_index, is_first_usage) =
                        find_or_insert_attachment(&mut pass.attachments, *virtual_image);
                    subpass.color_attachments[color_attachment_index] = Some(pass_attachment_index);

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
                        if !graph.image_version_info(write_image).read_usages.is_empty() {
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
                    let virtual_image = virtual_images.usage_to_virtual.get(&write_image).unwrap();
                    //let version_id = graph.image_version_id(write_image);
                    let specification = image_constraints.images.get(&write_image).unwrap();
                    log::trace!("      virtual attachment (resolve): {:?}", virtual_image);

                    let (pass_attachment_index, is_first_usage) =
                        find_or_insert_attachment(&mut pass.attachments, *virtual_image);
                    subpass.resolve_attachments[resolve_attachment_index] =
                        Some(pass_attachment_index);

                    assert!(is_first_usage); // Not sure if this assert is valid
                    let mut attachment = &mut pass.attachments[pass_attachment_index];
                    attachment.format = specification.format;
                    attachment.samples = specification.samples;

                    //TODO: Should we skip resolving if there is no reader?
                    let store_op = if !graph.image_version_info(write_image).read_usages.is_empty()
                    {
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
                let virtual_image = virtual_images
                    .usage_to_virtual
                    .get(&read_or_write_usage)
                    .unwrap();
                //let version_id = graph.image_version_id(read_or_write_usage);
                let specification = image_constraints.images.get(&read_or_write_usage).unwrap();
                log::trace!("      virtaul attachment (depth): {:?}", virtual_image);

                let (pass_attachment_index, is_first_usage) =
                    find_or_insert_attachment(&mut pass.attachments, *virtual_image);
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
                    if !graph.image_version_info(write_image).read_usages.is_empty() {
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

#[derive(Debug)]
struct AssignPhysicalImagesResult {
    usage_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId>,
    virtual_to_physical: FnvHashMap<VirtualImageId, PhysicalImageId>,
    specifications: Vec<RenderGraphImageSpecification>,
}

#[profiling::function]
fn assign_physical_images(
    graph: &RenderGraphBuilder,
    image_constraints: &DetermineImageConstraintsResult,
    virtual_images: &AssignVirtualImagesResult,
    passes: &mut [RenderGraphPass],
) -> AssignPhysicalImagesResult {
    log::trace!("-- Assign physical images --");
    struct PhysicalImageReuseRequirements {
        virtual_id: VirtualImageId,
        specification: RenderGraphImageSpecification,
        first_node_pass_index: usize,
        last_node_pass_index: usize,
    }

    fn add_or_modify_reuse_image_requirements(
        virtual_images: &AssignVirtualImagesResult,
        image_constraints: &DetermineImageConstraintsResult,
        pass_index: usize,
        usage: RenderGraphImageUsageId,
        reuse_requirements: &mut Vec<PhysicalImageReuseRequirements>,
        reuse_requirements_lookup: &mut FnvHashMap<VirtualImageId, usize>,
    ) {
        // Get physical ID from usage
        let virtual_id = virtual_images.usage_to_virtual[&usage];

        // Find requirements for this image if they exist, or create new requirements. This is a
        // lookup for an index so that the requirements will be stored sorted by
        // first_node_pass_index for iteration later
        let reused_image_requirements_index = *reuse_requirements_lookup
            .entry(virtual_id)
            .or_insert_with(|| {
                let reused_image_requirements_index = reuse_requirements.len();
                let specification = &image_constraints.images[&usage];
                reuse_requirements.push(PhysicalImageReuseRequirements {
                    virtual_id,
                    first_node_pass_index: pass_index,
                    last_node_pass_index: pass_index,
                    specification: specification.clone(),
                });

                log::trace!("  Add requirement {:?} {:?}", virtual_id, specification);
                reused_image_requirements_index
            });

        // Update the last pass index
        reuse_requirements[reused_image_requirements_index].last_node_pass_index = pass_index;
    }

    let mut reuse_requirements = Vec::<PhysicalImageReuseRequirements>::default();
    let mut reuse_requirements_lookup = FnvHashMap::<VirtualImageId, usize>::default();

    for (pass_index, pass) in passes.iter().enumerate() {
        for subpass in &pass.subpasses {
            let node = graph.node(subpass.node);

            for modify in &node.image_modifies {
                add_or_modify_reuse_image_requirements(
                    virtual_images,
                    image_constraints,
                    pass_index,
                    modify.input,
                    &mut reuse_requirements,
                    &mut reuse_requirements_lookup,
                );
                add_or_modify_reuse_image_requirements(
                    virtual_images,
                    image_constraints,
                    pass_index,
                    modify.output,
                    &mut reuse_requirements,
                    &mut reuse_requirements_lookup,
                );
            }

            for read in &node.image_reads {
                add_or_modify_reuse_image_requirements(
                    virtual_images,
                    image_constraints,
                    pass_index,
                    read.image,
                    &mut reuse_requirements,
                    &mut reuse_requirements_lookup,
                );
            }

            for create in &node.image_creates {
                add_or_modify_reuse_image_requirements(
                    virtual_images,
                    image_constraints,
                    pass_index,
                    create.image,
                    &mut reuse_requirements,
                    &mut reuse_requirements_lookup,
                );
            }
        }
    }

    //TODO: Find transients
    //TODO: Mark input images as non-reuse?
    //TODO: Stay in same queue?

    struct PhysicalImage {
        specification: RenderGraphImageSpecification,
        last_node_pass_index: usize,
        can_be_reused: bool,
    }

    let mut physical_images = Vec::<PhysicalImage>::default();
    let mut virtual_to_physical = FnvHashMap::<VirtualImageId, PhysicalImageId>::default();

    // First allocate physical IDs for all output images
    for output_image in &graph.output_images {
        let physical_image_id = PhysicalImageId(physical_images.len());
        physical_images.push(PhysicalImage {
            specification: output_image.specification.clone(),
            last_node_pass_index: passes.len() - 1,
            can_be_reused: false, // Should be safe to allow reuse? But last_node_pass_index effectively makes this never reuse
        });

        let virtual_id = virtual_images.usage_to_virtual[&output_image.usage];
        let old = virtual_to_physical.insert(virtual_id, physical_image_id);
        log::trace!(
            "  Output Image {:?} -> {:?} Used in passes [{}:{}]",
            virtual_id,
            physical_image_id,
            0,
            passes.len() - 1
        );
        assert!(old.is_none());
    }

    // Images are sorted by first usage (because we register them in order of the passes that first use them)
    for reuse_requirements in &reuse_requirements {
        if virtual_to_physical.contains_key(&reuse_requirements.virtual_id) {
            // May already have been registered by output image
            continue;
        }

        // See if we can reuse with an existing physical image
        let mut physical_image_id = None;
        for (physical_image_index, physical_image) in physical_images.iter_mut().enumerate() {
            if physical_image.last_node_pass_index < reuse_requirements.first_node_pass_index
                && physical_image.can_be_reused
            {
                if physical_image
                    .specification
                    .try_merge(&reuse_requirements.specification)
                {
                    physical_image.last_node_pass_index = reuse_requirements.last_node_pass_index;
                    physical_image_id = Some(PhysicalImageId(physical_image_index));
                    log::trace!(
                        "  Intermediate Image (Reuse) {:?} -> {:?} Used in passes [{}:{}]",
                        reuse_requirements.virtual_id,
                        physical_image_id,
                        reuse_requirements.first_node_pass_index,
                        reuse_requirements.last_node_pass_index
                    );
                    break;
                }
            }
        }

        // If the existing physical images are not compatible, make a new one
        let physical_image_id = physical_image_id.unwrap_or_else(|| {
            let physical_image_id = PhysicalImageId(physical_images.len());
            physical_images.push(PhysicalImage {
                specification: reuse_requirements.specification.clone(),
                last_node_pass_index: reuse_requirements.last_node_pass_index,
                can_be_reused: true,
            });

            log::trace!(
                "  Intermediate Image (Create new) {:?} -> {:?} Used in passes [{}:{}]",
                reuse_requirements.virtual_id,
                physical_image_id,
                reuse_requirements.first_node_pass_index,
                reuse_requirements.last_node_pass_index
            );
            physical_image_id
        });

        virtual_to_physical.insert(reuse_requirements.virtual_id, physical_image_id);
    }

    let mut map_image_to_physical = FnvHashMap::default();
    for (&usage, virtual_image) in &virtual_images.usage_to_virtual {
        //TODO: This was breaking in a test because an output image had no usage flags and we
        // never assigned the output image a physical ID since it wasn't in a pass
        map_image_to_physical.insert(usage, virtual_to_physical[virtual_image]);
    }

    let physical_image_specifications: Vec<_> = physical_images
        .into_iter()
        .map(|x| x.specification)
        .collect();

    for pass in passes {
        for attachment in &mut pass.attachments {
            attachment.image = Some(virtual_to_physical[&attachment.virtual_image]);
        }
    }

    AssignPhysicalImagesResult {
        virtual_to_physical,
        usage_to_physical: map_image_to_physical,
        specifications: physical_image_specifications,
    }
}

#[profiling::function]
fn build_node_barriers(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    _image_constraints: &DetermineImageConstraintsResult,
    physical_images: &AssignPhysicalImagesResult,
    //determine_image_layouts_result: &DetermineImageLayoutsResult,
) -> FnvHashMap<RenderGraphNodeId, RenderGraphNodeImageBarriers> {
    let mut barriers = FnvHashMap::<RenderGraphNodeId, RenderGraphNodeImageBarriers>::default();

    for node_id in node_execution_order {
        let node = graph.node(*node_id);
        //let mut invalidate_barriers = FnvHashMap<PhysicalImageId, RenderGraphImageBarrier>::default();
        //let mut flush_barriers = FnvHashMap<PhysicalImageId, RenderGraphImageBarrier>::default();
        let mut node_barriers: FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers> =
            Default::default();

        for color_attachment in &node.color_attachments {
            if let Some(color_attachment) = color_attachment {
                let read_or_write_usage = color_attachment
                    .read_image
                    .or(color_attachment.write_image)
                    .unwrap();
                let physical_image = physical_images
                    .usage_to_physical
                    .get(&read_or_write_usage)
                    .unwrap();
                //let version_id = graph.image_version_id(read_or_write_usage);

                let barrier = node_barriers.entry(*physical_image).or_insert_with(|| {
                    RenderGraphPassImageBarriers::new(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                });

                barrier.used_by_attachment |= true;

                if color_attachment.read_image.is_some() {
                    barrier.invalidate.access_flags |= vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                        | vk::AccessFlags::COLOR_ATTACHMENT_READ;
                    barrier.invalidate.stage_flags |=
                        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
                    //barrier.invalidate.layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
                    //invalidate_barrier.layout = determine_image_layouts_result.image_layouts[&version_id].read_layout.into();
                }

                if color_attachment.write_image.is_some() {
                    barrier.flush.access_flags |= vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
                    barrier.flush.stage_flags |= vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
                    //barrier.flush.layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
                    //flush_barrier.layout = determine_image_layouts_result.image_layouts[&version_id].write_layout.into();
                }
            }
        }

        for resolve_attachment in &node.resolve_attachments {
            if let Some(resolve_attachment) = resolve_attachment {
                let physical_image = physical_images
                    .usage_to_physical
                    .get(&resolve_attachment.write_image)
                    .unwrap();
                //let version_id = graph.image_version_id(resolve_attachment.write_image);

                let barrier = node_barriers.entry(*physical_image).or_insert_with(|| {
                    RenderGraphPassImageBarriers::new(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                });

                barrier.used_by_attachment |= true;

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
                .usage_to_physical
                .get(&read_or_write_usage)
                .unwrap();
            //let version_id = graph.image_version_id(read_or_write_usage);

            let barrier = node_barriers.entry(*physical_image).or_insert_with(|| {
                RenderGraphPassImageBarriers::new(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            });

            barrier.used_by_attachment |= true;

            if depth_attachment.read_image.is_some() && depth_attachment.write_image.is_some() {
                //barrier.invalidate.layout = vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
                //barrier.invalidate.layout = determine_image_layouts_result.image_layouts[&version_id].read_layout.into();
                barrier.invalidate.access_flags |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
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
                barrier.invalidate.access_flags |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ;
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

        for sampled_image in &node.sampled_images {
            let physical_image = physical_images
                .usage_to_physical
                .get(sampled_image)
                .unwrap();

            let barrier = node_barriers.entry(*physical_image).or_insert_with(|| {
                RenderGraphPassImageBarriers::new(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            });

            barrier.used_by_sampling |= true;

            barrier.invalidate.access_flags |= vk::AccessFlags::SHADER_READ;
            barrier.invalidate.stage_flags |= vk::PipelineStageFlags::FRAGMENT_SHADER;
        }

        // barriers.push(RenderGraphNodeImageBarriers {
        //     invalidates: invalidate_barriers,
        //     flushes: flush_barriers
        // });
        barriers.insert(
            *node_id,
            RenderGraphNodeImageBarriers {
                barriers: node_barriers,
            },
        );
    }

    barriers
}

// * At this point we know format, samples, load_op, stencil_load_op, and initial_layout. We also
//   know what needs to be flushed/invalidated
// * We want to determine store_op, stencil_store_op, final_layout. And the validates/flushes
//   we actually need to insert
#[profiling::function]
fn build_pass_barriers(
    graph: &RenderGraphBuilder,
    _node_execution_order: &[RenderGraphNodeId],
    _image_constraints: &DetermineImageConstraintsResult,
    physical_images: &AssignPhysicalImagesResult,
    node_barriers: &FnvHashMap<RenderGraphNodeId, RenderGraphNodeImageBarriers>,
    passes: &mut [RenderGraphPass],
) -> Vec<Vec<dsc::SubpassDependency>> {
    log::trace!("-- build_pass_barriers --");
    const MAX_PIPELINE_FLAG_BITS: usize = 15;
    // #[allow(non_snake_case)]
    // let ALL_GRAPHICS: vk::PipelineStageFlags =
    //     vk::PipelineStageFlags::from_raw(0b111_1111_1110);

    //
    // We will walk through all nodes keeping track of memory access as we go
    //
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

    //TODO: to support subpass, probably need image states for each previous subpass
    let mut image_states: Vec<ImageState> =
        Vec::with_capacity(physical_images.specifications.len());
    image_states.resize_with(physical_images.specifications.len(), || Default::default());

    // dependencies for all renderpasses
    let mut pass_dependencies = Vec::default();

    for (pass_index, pass) in passes.iter_mut().enumerate() {
        log::trace!("pass {}", pass_index);

        // Dependencies for this renderpass
        let mut subpass_dependencies = Vec::default();

        // Initial layout for all attachments at the start of the renderpass
        let mut attachment_initial_layout: Vec<Option<dsc::ImageLayout>> = Default::default();
        attachment_initial_layout.resize_with(pass.attachments.len(), || None);

        //TODO: This does not support multipass
        assert_eq!(pass.subpasses.len(), 1);
        for (subpass_index, subpass) in pass.subpasses.iter_mut().enumerate() {
            log::trace!("  subpass {}", subpass_index);
            let node_barriers = &node_barriers[&subpass.node];

            // Accumulate the invalidates for this subpass here
            let mut invalidate_src_access_flags = vk::AccessFlags::empty();
            let mut invalidate_src_pipeline_stage_flags = vk::PipelineStageFlags::empty();
            let mut invalidate_dst_access_flags = vk::AccessFlags::empty();
            let mut invalidate_dst_pipeline_stage_flags = vk::PipelineStageFlags::empty();

            // See if we can rely on an external dependency on the subpass to do layout transitions.
            // Common case where this is not possible is having any image that's not an attachment
            // being used via sampling.
            let mut use_external_dependency_for_pass_initial_layout_transition = true;
            for (physical_image_id, image_barrier) in &node_barriers.barriers {
                if image_barrier.used_by_sampling
                    && image_states[physical_image_id.0].layout != image_barrier.layout
                {
                    log::trace!("    will emit separate barrier for layout transitions");
                    use_external_dependency_for_pass_initial_layout_transition = false;
                    break;
                }
            }

            let mut image_transitions = Vec::default();
            // Look at all the images we read and determine what invalidates we need
            for (physical_image_id, image_barrier) in &node_barriers.barriers {
                log::trace!("    image {:?}", physical_image_id);
                let image_state = &mut image_states[physical_image_id.0];

                // Include the previous writer's stage/access flags, if there were any
                invalidate_src_access_flags |= image_state.pending_flush_access_flags;
                invalidate_src_pipeline_stage_flags |=
                    image_state.pending_flush_pipeline_stage_flags;

                // layout changes are write operations and can cause hazards. We need to
                // block on any stages before that are reading or writing
                let layout_change = image_state.layout != image_barrier.layout;
                if layout_change {
                    log::trace!(
                        "      layout change! {:?} -> {:?}",
                        image_state.layout,
                        image_barrier.layout
                    );
                    for i in 0..MAX_PIPELINE_FLAG_BITS {
                        if image_state.invalidated[i] != vk::AccessFlags::empty() {
                            // Add an execution barrier if we are transitioning the layout
                            // of something that is already being read from
                            let pipeline_stage = vk::PipelineStageFlags::from_raw(1 << i);
                            log::trace!(
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
                    log::trace!(
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
                            log::trace!(
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
                    log::trace!("      no invalidation required, clearing access flags");
                    image_invalidate_access_flags = vk::AccessFlags::empty();
                }

                log::trace!("      Access Flags: {:?}", image_invalidate_access_flags);
                log::trace!(
                    "      Pipeline Stage Flags: {:?}",
                    image_invalidate_pipeline_stage_flags
                );

                // OR the requirements in
                invalidate_dst_access_flags |= image_invalidate_access_flags;
                invalidate_dst_pipeline_stage_flags |= image_invalidate_pipeline_stage_flags;

                // Set the initial layout for the attachment, but only if it's the first time we've seen it
                //TODO: This is bad and does not properly handle an image being used in multiple ways requiring
                // multiple layouts
                for (attachment_index, attachment) in &mut pass.attachments.iter_mut().enumerate() {
                    //log::trace!("      attachment {:?}", attachment.image);
                    if attachment.image.unwrap() == *physical_image_id {
                        if attachment_initial_layout[attachment_index].is_none() {
                            //log::trace!("        initial layout {:?}", image_barrier.layout);
                            attachment_initial_layout[attachment_index] =
                                Some(image_state.layout.into());

                            if use_external_dependency_for_pass_initial_layout_transition {
                                // Use an external dependency on the renderpass to do the image
                                // transition
                                attachment.initial_layout = image_state.layout.into();
                            } else {
                                // Use an image barrier before the pass to transition the layout,
                                // so we will already be in the correct layout before starting the
                                // pass.
                                attachment.initial_layout = image_barrier.layout.into();
                            }
                        }

                        attachment.final_layout = image_barrier.layout.into();
                        break;
                    }
                }

                if layout_change && !use_external_dependency_for_pass_initial_layout_transition {
                    image_transitions.push((
                        physical_image_id,
                        image_state.layout,
                        image_barrier.layout,
                    ));
                }

                image_state.layout = image_barrier.layout;
            }

            //
            // for (physical_image_id, image_barrier) in &node_barriers.flushes {
            //     log::trace!("    flush");
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
            //         log::trace!("      attachment {:?}", attachment.image);
            //         if attachment.image == *physical_image_id {
            //             log::trace!("        final layout {:?}", image_barrier.layout);
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

            if use_external_dependency_for_pass_initial_layout_transition {
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
            } else {
                let image_barriers = image_transitions
                    .into_iter()
                    .map(|(&image, old_layout, new_layout)| PrepassImageBarrier {
                        image,
                        old_layout,
                        new_layout,
                        src_access: invalidate_src_access_flags,
                        dst_access: invalidate_dst_access_flags,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    })
                    .collect();

                let barrier = PrepassBarrier {
                    src_stage: invalidate_src_pipeline_stage_flags,
                    dst_stage: invalidate_dst_pipeline_stage_flags,
                    image_barriers,
                };

                pass.pre_pass_barrier = Some(barrier);
            }

            // Handle the flush synchronization
            for (physical_image_id, image_barrier) in &node_barriers.barriers {
                let image_state = &mut image_states[physical_image_id.0];

                // Queue up flushes to happen later based on what this pass writes
                image_state.pending_flush_pipeline_stage_flags |= image_barrier.flush.stage_flags;
                image_state.pending_flush_access_flags |= image_barrier.flush.access_flags;

                // If we write something, mark it as no longer invalidated
                //TODO: Not sure if we invalidate specific stages or all stages
                //TODO: Can we invalidate specific access instead of all access?
                for i in 0..MAX_PIPELINE_FLAG_BITS {
                    image_state.invalidated[i] = vk::AccessFlags::empty();
                }

                // If we add code to change final layout, ensure that we set up a dependency
            }

            // Do not put unstored images into UNDEFINED layout, per vulkan spec

            // TODO: Figure out how to handle output images
            // TODO: This only works if no one else reads it?
            log::trace!("Check for output images");
            for (output_image_index, output_image) in graph.output_images.iter().enumerate() {
                if graph.image_version_info(output_image.usage).creator_node == subpass.node {
                    //output_image.
                    //graph.image_usages[output_image.usage]

                    let output_physical_image =
                        physical_images.usage_to_physical[&output_image.usage];
                    log::trace!(
                        "Output image {} usage {:?} created by node {:?} physical image {:?}",
                        output_image_index,
                        output_image.usage,
                        subpass.node,
                        output_physical_image
                    );

                    for (attachment_index, attachment) in
                        &mut pass.attachments.iter_mut().enumerate()
                    {
                        if attachment.image.unwrap() == output_physical_image {
                            log::trace!("  attachment {}", attachment_index);
                            attachment.final_layout = output_image.final_layout;
                        }
                    }
                    //TODO: Need a 0 -> EXTERNAL dependency here?
                }
            }

            //TODO: Need to do a dependency? Maybe by adding a flush?
        }

        pass_dependencies.push(subpass_dependencies);
    }

    pass_dependencies
}

#[profiling::function]
fn create_output_passes(
    passes: Vec<RenderGraphPass>,
    node_barriers: FnvHashMap<RenderGraphNodeId, RenderGraphNodeImageBarriers>,
    subpass_dependencies: &Vec<Vec<dsc::SubpassDependency>>,
    swapchain_info: &SwapchainSurfaceInfo,
) -> Vec<RenderGraphOutputPass> {
    let mut renderpasses = Vec::with_capacity(passes.len());
    for (index, pass) in passes.into_iter().enumerate() {
        let mut renderpass_desc = dsc::RenderPass::default();
        let mut subpass_nodes = Vec::with_capacity(pass.subpasses.len());

        renderpass_desc.attachments.reserve(pass.attachments.len());
        for attachment in &pass.attachments {
            renderpass_desc
                .attachments
                .push(attachment.create_attachment_description());
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

            for (color_index, attachment_index) in subpass.color_attachments.iter().enumerate() {
                if let Some(attachment_index) = attachment_index {
                    let physical_image = pass.attachments[*attachment_index].image.unwrap();
                    set_attachment_reference(
                        &mut subpass_description.color_attachments,
                        color_index,
                        dsc::AttachmentReference {
                            attachment: dsc::AttachmentIndex::Index(*attachment_index as u32),
                            layout: node_barriers[&subpass.node].barriers[&physical_image]
                                .layout
                                .into(),
                        },
                    );
                }
            }

            for (resolve_index, attachment_index) in subpass.resolve_attachments.iter().enumerate()
            {
                if let Some(attachment_index) = attachment_index {
                    let physical_image = pass.attachments[*attachment_index].image.unwrap();
                    set_attachment_reference(
                        &mut subpass_description.resolve_attachments,
                        resolve_index,
                        dsc::AttachmentReference {
                            attachment: dsc::AttachmentIndex::Index(*attachment_index as u32),
                            layout: node_barriers[&subpass.node].barriers[&physical_image]
                                .layout
                                .into(),
                        },
                    );
                }
            }

            if let Some(attachment_index) = subpass.depth_attachment {
                let physical_image = pass.attachments[attachment_index].image.unwrap();
                subpass_description.depth_stencil_attachment = Some(dsc::AttachmentReference {
                    attachment: dsc::AttachmentIndex::Index(attachment_index as u32),
                    layout: node_barriers[&subpass.node].barriers[&physical_image]
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
            renderpass_desc.dependencies.push(dependency.clone());
        }

        let attachment_images = pass
            .attachments
            .iter()
            .map(|attachment| attachment.image.unwrap())
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
            description: Arc::new(renderpass_desc),
            extents: swapchain_info.extents,
            attachment_images,
            clear_values,
            pre_pass_barrier: pass.pre_pass_barrier,
        };

        renderpasses.push(output_pass);
    }

    renderpasses
}

#[allow(dead_code)]
fn print_image_constraints(
    graph: &RenderGraphBuilder,
    image_constraint_results: &mut DetermineImageConstraintsResult,
) {
    log::trace!("Image constraints:");
    for (image_index, image_resource) in graph.image_resources.iter().enumerate() {
        log::trace!("  Image {:?} {:?}", image_index, image_resource.name);
        for (version_index, version) in image_resource.versions.iter().enumerate() {
            log::trace!("    Version {}", version_index);

            log::trace!(
                "      Writen as: {:?}",
                image_constraint_results.specification(version.create_usage)
            );

            for (usage_index, usage) in version.read_usages.iter().enumerate() {
                log::trace!(
                    "      Read Usage {}: {:?}",
                    usage_index,
                    image_constraint_results.specification(*usage)
                );
            }
        }
    }
}

#[allow(dead_code)]
fn print_image_compatibility(
    graph: &RenderGraphBuilder,
    image_constraint_results: &DetermineImageConstraintsResult,
) {
    log::trace!("Image Compatibility Report:");
    for (image_index, image_resource) in graph.image_resources.iter().enumerate() {
        log::trace!("  Image {:?} {:?}", image_index, image_resource.name);
        for (version_index, version) in image_resource.versions.iter().enumerate() {
            let write_specification = image_constraint_results.specification(version.create_usage);

            log::trace!("    Version {}: {:?}", version_index, version);
            for (usage_index, usage) in version.read_usages.iter().enumerate() {
                let read_specification = image_constraint_results.specification(*usage);

                // TODO: Skip images we don't use?

                if write_specification == read_specification {
                    log::trace!("      read usage {} matches", usage_index);
                } else {
                    log::trace!("      read usage {} does not match", usage_index);
                    log::trace!("        produced: {:?}", write_specification);
                    log::trace!("        required: {:?}", read_specification);
                }
            }
        }
    }
}

#[allow(dead_code)]
fn print_node_barriers(
    node_barriers: &FnvHashMap<RenderGraphNodeId, RenderGraphNodeImageBarriers>
) {
    log::trace!("Barriers:");
    for (node_id, barriers) in node_barriers.iter() {
        log::trace!("  pass {:?}", node_id);
        log::trace!("    invalidates");
        for (physical_id, barriers) in &barriers.barriers {
            log::trace!("      {:?}: {:?}", physical_id, barriers.invalidate);
        }
        log::trace!("    flushes");
        for (physical_id, barriers) in &barriers.barriers {
            log::trace!("      {:?}: {:?}", physical_id, barriers.flush);
        }
    }
}

#[allow(dead_code)]
fn verify_unculled_image_usages_specifications_exist(
    graph: &RenderGraphBuilder,
    node_execution_order: &Vec<RenderGraphNodeId>,
    image_constraint_results: &DetermineImageConstraintsResult,
) {
    for (_image_index, image_resource) in graph.image_resources.iter().enumerate() {
        //log::trace!("  Image {:?} {:?}", image_index, image_resource.name);
        for (_version_index, version) in image_resource.versions.iter().enumerate() {
            // Check the write usage for this version
            if node_execution_order.contains(&version.creator_node)
                && image_constraint_results
                    .images
                    .get(&version.create_usage)
                    .is_none()
            {
                let usage_info = &graph.image_usages[version.create_usage.0];
                panic!(
                    "Could not determine specification for image {:?} use by {:?} for {:?}",
                    version.create_usage, usage_info.user, usage_info.usage_type
                );
            }

            // Check the read usages for this version
            for (_, usage) in version.read_usages.iter().enumerate() {
                let usage_info = &graph.image_usages[usage.0];
                let is_scheduled = match &usage_info.user {
                    RenderGraphImageUser::Node(node_id) => node_execution_order.contains(node_id),
                    RenderGraphImageUser::Output(_) => true,
                };

                if is_scheduled && image_constraint_results.images.get(usage).is_none() {
                    panic!(
                        "Could not determine specification for image {:?} used by {:?} for {:?}",
                        usage, usage_info.user, usage_info.usage_type
                    );
                }
            }
        }
    }
}

#[allow(dead_code)]
fn print_final_images(
    output_images: &FnvHashMap<PhysicalImageId, RenderGraphPlanOutputImage>,
    intermediate_images: &FnvHashMap<PhysicalImageId, RenderGraphImageSpecification>,
) {
    log::trace!("-- IMAGES --");
    for (physical_id, intermediate_image_spec) in intermediate_images {
        log::trace!(
            "Intermediate Image: {:?} {:?}",
            physical_id,
            intermediate_image_spec
        );
    }
    for (physical_id, output_image) in output_images {
        log::trace!("Output Image: {:?} {:?}", physical_id, output_image);
    }
}

#[allow(dead_code)]
fn print_final_image_usage(
    graph: &RenderGraphBuilder,
    assign_physical_images_result: &AssignPhysicalImagesResult,
    image_constraint_results: &DetermineImageConstraintsResult,
    renderpasses: &Vec<RenderGraphOutputPass>,
) {
    log::debug!("-- IMAGE USAGE --");
    for (pass_index, pass) in renderpasses.iter().enumerate() {
        log::debug!("pass {}", pass_index);
        for (subpass_index, _subpass) in pass.description.subpasses.iter().enumerate() {
            let node_id = pass.subpass_nodes[subpass_index];
            let node = graph.node(node_id);
            log::debug!("  subpass {} {:?} {:?}", subpass_index, node_id, node.name);

            for (color_attachment_index, color_attachment) in
                node.color_attachments.iter().enumerate()
            {
                if let Some(color_attachment) = color_attachment {
                    let read_or_write = color_attachment
                        .read_image
                        .or_else(|| color_attachment.write_image)
                        .unwrap();
                    let physical_image =
                        assign_physical_images_result.usage_to_physical[&read_or_write];
                    let write_name = color_attachment
                        .write_image
                        .map(|x| graph.image_resource(x).name)
                        .flatten();
                    log::debug!(
                        "    Color Attachment {}: {:?} Name: {:?} Constraints: {:?}",
                        color_attachment_index,
                        physical_image,
                        write_name,
                        image_constraint_results.images[&read_or_write]
                    );
                }
            }

            for (resolve_attachment_index, resolve_attachment) in
                node.resolve_attachments.iter().enumerate()
            {
                if let Some(resolve_attachment) = resolve_attachment {
                    let physical_image = assign_physical_images_result.usage_to_physical
                        [&resolve_attachment.write_image];
                    let write_name = graph.image_resource(resolve_attachment.write_image).name;
                    log::debug!(
                        "    Resolve Attachment {}: {:?} Name: {:?} Constraints: {:?}",
                        resolve_attachment_index,
                        physical_image,
                        write_name,
                        image_constraint_results.images[&resolve_attachment.write_image]
                    );
                }
            }

            if let Some(depth_attachment) = &node.depth_attachment {
                let read_or_write = depth_attachment
                    .read_image
                    .or_else(|| depth_attachment.write_image)
                    .unwrap();
                let physical_image =
                    assign_physical_images_result.usage_to_physical[&read_or_write];
                let write_name = depth_attachment
                    .write_image
                    .map(|x| graph.image_resource(x).name)
                    .flatten();
                log::debug!(
                    "    Depth Attachment: {:?} Name: {:?} Constraints: {:?}",
                    physical_image,
                    write_name,
                    image_constraint_results.images[&read_or_write]
                );
            }

            for sampled_image in &node.sampled_images {
                let physical_image = assign_physical_images_result.usage_to_physical[sampled_image];
                let write_name = graph.image_resource(*sampled_image).name;
                log::debug!(
                    "    Sampled: {:?} Name: {:?} Constraints: {:?}",
                    physical_image,
                    write_name,
                    image_constraint_results.images[sampled_image]
                );
            }
        }
    }
    for output_image in &graph.output_images {
        let physical_image = assign_physical_images_result.usage_to_physical[&output_image.usage];
        let write_name = graph.image_resource(output_image.usage).name;
        log::debug!(
            "    Output Image {:?} Name: {:?} Constraints: {:?}",
            physical_image,
            write_name,
            image_constraint_results.images[&output_image.usage]
        );
    }
}

#[derive(Debug)]
pub struct RenderGraphPlanOutputImage {
    pub output_id: RenderGraphOutputImageId,
    pub dst_image: ResourceArc<ImageViewResource>,
}

/// The final output of a render graph, which will be consumed by PreparedRenderGraph. This just
/// includes the computed metadata and does not allocate resources.
#[derive(Debug)]
pub struct RenderGraphPlan {
    pub passes: Vec<RenderGraphOutputPass>,
    pub output_images: FnvHashMap<PhysicalImageId, RenderGraphPlanOutputImage>,
    pub intermediate_images: FnvHashMap<PhysicalImageId, RenderGraphImageSpecification>,
    pub node_to_renderpass_index: FnvHashMap<RenderGraphNodeId, usize>,
    pub image_usage_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId>,
}

impl RenderGraphPlan {
    #[profiling::function]
    pub(super) fn new(
        mut graph: RenderGraphBuilder,
        swapchain_info: &SwapchainSurfaceInfo,
    ) -> RenderGraphPlan {
        log::trace!("-- Create render graph plan --");

        //
        // Walk backwards through the DAG, starting from the output images, through all the upstream
        // dependencies of those images. We are doing a depth first search. Nodes that make no
        // direct or indirect contribution to an output image will not be included. As an
        // an implementation detail, we try to put renderpass merge candidates adjacent to each
        // other in this list
        //
        let node_execution_order = determine_node_order(&graph);

        // Print out the execution order
        log::trace!("Execution order of unculled nodes:");
        for node in &node_execution_order {
            log::trace!("  Node {:?} {:?}", node, graph.node(*node).name());
        }

        //
        // Traverse the graph to determine specifications for all images that will be used. This
        // iterates forwards and backwards through the node graph. This allows us to specify
        // attributes about images (like format, sample count) in key areas and infer it elsewhere.
        // If there is not enough information to infer then the render graph cannot be used and
        // building it will panic.
        //
        let mut image_constraint_results =
            determine_image_constraints(&graph, &node_execution_order);

        // Look at all image versions and ensure a constraint exists for usages where the node was
        // not culled
        //RenderGraphPlan::verify_unculled_image_usages_specifications_exist(&graph, &node_execution_order, &image_constraint_results);

        // Print out the constraints assigned to images
        //print_image_constraints(&graph, &mut image_constraint_results);

        //
        // Add resolves to the graph - this will occur when a renderpass outputs a multisample image
        // to a renderpass that is expecting a non-multisampled image.
        //
        insert_resolves(
            &mut graph,
            &node_execution_order,
            &mut image_constraint_results,
        );

        // Print the cases where we can't reuse images
        //print_image_compatibility(&graph, &image_constraint_results);

        //
        // Assign logical images to physical images. This should give us a minimal number of images
        // if we are not reusing or aliasing. (We reuse when we assign physical indexes)
        //
        let assign_virtual_images_result =
            assign_virtual_images(&graph, &node_execution_order, &mut image_constraint_results);

        //
        // Combine nodes into passes where possible
        //
        let mut passes = build_physical_passes(
            &graph,
            &node_execution_order,
            &image_constraint_results,
            &assign_virtual_images_result, /*, &determine_image_layouts_result*/
        );

        //
        // Find virtual images with matching specification and non-overlapping lifetimes. Assign
        // the same physical index to them so that we reuse a single allocation
        //
        let assign_physical_images_result = assign_physical_images(
            &graph,
            &image_constraint_results,
            &assign_virtual_images_result,
            &mut passes,
        );

        // log::trace!("Merged Renderpasses:");
        // for (index, pass) in passes.iter().enumerate() {
        //     log::trace!("  pass {}", index);
        //     log::trace!("    attachments:");
        //     for attachment in &pass.attachments {
        //         log::trace!("      {:?}", attachment);
        //     }
        //     log::trace!("    subpasses:");
        //     for subpass in &pass.subpasses {
        //         log::trace!("      {:?}", subpass);
        //     }
        // }

        //
        // Determine read/write barriers for each node based on the data the produce/consume
        //
        let node_barriers = build_node_barriers(
            &graph,
            &node_execution_order,
            &image_constraint_results,
            &assign_physical_images_result, /*, &determine_image_layouts_result*/
        );

        print_node_barriers(&node_barriers);

        //TODO: Figure out in/out layouts for passes? Maybe insert some other fixes? Drop transient
        // images?

        //
        // Combine the node barriers to produce the dependencies for subpasses and determine/handle
        // image layout transitions
        //
        let subpass_dependencies = build_pass_barriers(
            &graph,
            &node_execution_order,
            &image_constraint_results,
            &assign_physical_images_result,
            &node_barriers,
            &mut passes,
        );

        // log::trace!("Merged Renderpasses:");
        // for (index, pass) in passes.iter().enumerate() {
        //     log::trace!("  pass {}", index);
        //     log::trace!("    attachments:");
        //     for attachment in &pass.attachments {
        //         log::trace!("      {:?}", attachment);
        //     }
        //     log::trace!("    subpasses:");
        //     for subpass in &pass.subpasses {
        //         log::trace!("      {:?}", subpass);
        //     }
        //     log::trace!("    dependencies:");
        //     for subpass in &subpass_dependencies[index] {
        //         log::trace!("      {:?}", subpass);
        //     }
        // }

        //TODO: Cull images that only exist within the lifetime of a single pass? (just passed among
        // subpasses)

        //TODO: Allocation of images
        // alias_images(
        //     &graph,
        //     &node_execution_order,
        //     &image_constraint_results,
        //     &assign_physical_images_result,
        //     &node_barriers,
        //     &passes,
        // );

        //
        // Produce the final output data. This mainly includes a descriptor object that can be
        // passed into the resource system to create the renderpass but also includes other metadata
        // required to push them through the command queue
        //
        let renderpasses =
            create_output_passes(passes, node_barriers, &subpass_dependencies, swapchain_info);

        //
        // Separate the output images from the intermediate images (the rendergraph will be
        // responsible for allocating the intermediate images)
        //
        let mut output_images: FnvHashMap<PhysicalImageId, RenderGraphPlanOutputImage> =
            Default::default();
        for output_image in &graph.output_images {
            let output_image_physical_id =
                assign_physical_images_result.usage_to_physical[&output_image.usage];

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
        for (index, specification) in assign_physical_images_result
            .specifications
            .iter()
            .enumerate()
        {
            let physical_image = PhysicalImageId(index);
            if output_images.contains_key(&physical_image) {
                continue;
            }

            intermediate_images.insert(physical_image, specification.clone());
        }

        // log::trace!("-- RENDERPASS {} --", renderpass_index);
        // for (renderpass_index, renderpass) in renderpasses.iter().enumerate() {
        //     log::trace!("-- RENDERPASS {} --", renderpass_index);
        //     log::trace!("{:#?}", renderpass);
        // }

        print_final_images(&output_images, &intermediate_images);

        print_final_image_usage(
            &graph,
            &assign_physical_images_result,
            &image_constraint_results,
            &renderpasses,
        );

        //
        // Create a lookup from node_id to renderpass. Nodes are culled and renderpasses may include
        // subpasses from multiple nodes.
        //
        let mut node_to_renderpass_index = FnvHashMap::default();
        for (renderpass_index, renderpass) in renderpasses.iter().enumerate() {
            for &node in &renderpass.subpass_nodes {
                node_to_renderpass_index.insert(node, renderpass_index);
            }
        }

        RenderGraphPlan {
            passes: renderpasses,
            output_images,
            intermediate_images,
            node_to_renderpass_index,
            image_usage_to_physical: assign_physical_images_result.usage_to_physical,
        }
    }
}
