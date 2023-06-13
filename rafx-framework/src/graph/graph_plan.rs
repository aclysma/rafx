use super::*;
use super::{RenderGraphExternalImageId, RenderGraphImageSpecification};
use crate::graph::graph_image::{PhysicalImageId, RenderGraphImageUser, VirtualImageId};
use crate::graph::graph_node::RenderGraphNodeId;
use crate::graph::{RenderGraphBuilder, RenderGraphImageConstraint, RenderGraphImageUsageId};
use crate::render_features::RenderPhaseIndex;
use crate::{BufferResource, GraphicsPipelineRenderTargetMeta};
use crate::{ImageViewResource, ResourceArc};
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::{RafxFormat, RafxLoadOp, RafxResourceState, RafxSampleCount, RafxStoreOp};

// Recursively called to topologically sort the nodes to determine execution order. See
// determine_node_order which kicks this off.
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
        log::warn!("Found cycle in graph");
        log::warn!("{:?}", graph.node(node_id));
        for v in visiting_stack.iter().rev() {
            log::warn!("{:?}", graph.node(*v));
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

    //
    // Visit all the nodes we aren't delaying
    //
    for read in &node.image_reads {
        let upstream_node = graph.image_version_info(read.image).creator_node;
        visit_node(
            graph,
            upstream_node,
            visited,
            visiting,
            visiting_stack,
            ordered_list,
        );
    }

    for modify in &node.image_modifies {
        let upstream_node = graph.image_version_info(modify.input).creator_node;
        visit_node(
            graph,
            upstream_node,
            visited,
            visiting,
            visiting_stack,
            ordered_list,
        );
    }

    for read in &node.buffer_reads {
        let upstream_node = graph.buffer_version_info(read.buffer).creator_node;
        visit_node(
            graph,
            upstream_node,
            visited,
            visiting,
            visiting_stack,
            ordered_list,
        );
    }

    for modify in &node.buffer_modifies {
        let upstream_node = graph.buffer_version_info(modify.input).creator_node;
        visit_node(
            graph,
            upstream_node,
            visited,
            visiting,
            visiting_stack,
            ordered_list,
        );
    }

    for &explicit_dependency in &node.explicit_dependencies {
        visit_node(
            graph,
            explicit_dependency,
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

//
// The purpose of this function is to determine the order that nodes should execute in. We do this
// by following the graph from the outputs backwards.
//
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
    for external_image in &graph.external_images {
        if let Some(output_usage) = external_image.output_usage {
            // Find the node that creates the output image
            let output_node = graph.image_version_info(output_usage).creator_node;
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
    }

    // Iterate all the buffers we need to output. This will visit all the nodes we need to execute,
    // potentially leaving out nodes we can cull.
    for external_buffer in &graph.external_buffers {
        if let Some(output_usage) = external_buffer.output_usage {
            // Find the node that creates the output buffer
            let output_node = graph.buffer_version_info(output_usage).creator_node;
            log::trace!(
                "Traversing dependencies of output buffer created by node {:?} {:?}",
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
    }

    for node in &graph.nodes {
        if !node.can_be_culled {
            visit_node(
                graph,
                node.id(),
                &mut visited,
                &mut visiting,
                &mut visiting_stack,
                &mut ordered_list,
            );
        }
    }

    ordered_list
}

/// The specification for the image by image usage
pub struct DetermineConstraintsResult {
    images: FnvHashMap<RenderGraphImageUsageId, RenderGraphImageSpecification>,
    buffers: FnvHashMap<RenderGraphBufferUsageId, RenderGraphBufferSpecification>,
}

impl DetermineConstraintsResult {
    pub fn image_specification(
        &self,
        image: RenderGraphImageUsageId,
    ) -> Option<&RenderGraphImageSpecification> {
        self.images.get(&image)
    }

    pub fn buffer_specification(
        &self,
        buffer: RenderGraphBufferUsageId,
    ) -> Option<&RenderGraphBufferSpecification> {
        self.buffers.get(&buffer)
    }
}

//
// This function determines the specifications of all images. This is done by looking at the
// constraints on the image at every point it is used. This information and how the image is used
// will determine the image specification. (The specification is all the information needed to
// create the image. Conflicting constraints or incomplete constraints will result in an error.
//
// The general algorithm here is to start from the beginning of the graph, walk forward to the end,
// propagating the constraints. Then walk backwards from the end to the beginning.
//
#[profiling::function]
fn determine_constraints(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    swapchain_surface_info: &SwapchainSurfaceInfo,
) -> DetermineConstraintsResult {
    let mut image_version_states: FnvHashMap<RenderGraphImageUsageId, RenderGraphImageConstraint> =
        Default::default();

    let mut buffer_version_states: FnvHashMap<
        RenderGraphBufferUsageId,
        RenderGraphBufferConstraint,
    > = Default::default();

    log::trace!("Propagating constraints");

    log::trace!("  Set up input images");

    //
    // Propagate input image state specifications into images. Inputs are fully specified and
    // their constraints will never be overwritten
    //
    for external_image in &graph.external_images {
        if let Some(input_usage) = external_image.input_usage {
            log::trace!(
                "    Image {:?} {:?}",
                input_usage,
                graph.image_resource(input_usage).name
            );
            debug_assert_eq!(graph.image_version_create_usage(input_usage), input_usage);
            image_version_states
                .entry(input_usage)
                .or_default()
                .set(&external_image.specification);

            // Don't bother setting usage constraint for 0
        }
    }

    log::trace!("  Set up input buffers");

    //
    // Propagate input buffer state specifications into buffers. Inputs are fully specified and
    // their constraints will never be overwritten
    //
    for external_buffer in &graph.external_buffers {
        if let Some(input_usage) = external_buffer.input_usage {
            log::trace!(
                "    Buffer {:?} {:?}",
                input_usage,
                graph.buffer_resource(input_usage).name
            );
            debug_assert_eq!(graph.buffer_version_create_usage(input_usage), input_usage);
            buffer_version_states
                .entry(input_usage)
                .or_default()
                .set(&external_buffer.specification);

            // Don't bother setting usage constraint for 0
        }
    }

    log::trace!("  Propagate constraints FORWARD");

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
            // vs. receiving an input. We probably need to make creator in
            // RenderGraphImageResourceVersionInfo Option or an enum with input/create options
            //assert!(graph.image_version_info(image_create.image).input_image.is_none());

            log::trace!(
                "      Create image {:?} {:?}",
                image_create.image,
                graph.image_resource(image_create.image).name
            );

            let version_state = image_version_states
                .entry(graph.image_version_create_usage(image_create.image))
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

        //
        // Propagate constraints into buffers this node creates.
        //
        for buffer_create in &node.buffer_creates {
            // A buffer cannot be created within the graph and imported externally at the same
            // time. (The code here assumes no input and will not produce correct results if there
            // is an input buffer)
            //TODO: Input buffers are broken, we don't properly represent a buffer being created
            // vs. receiving an input. We probably need to make creator in
            // RenderGraphImageResourceVersionInfo Option or an enum with input/create options
            //assert!(graph.buffer_version_info(buffer_create.buffer).input_buffer.is_none());

            log::trace!(
                "      Create buffer {:?} {:?}",
                buffer_create.buffer,
                graph.buffer_resource(buffer_create.buffer).name
            );

            let version_state = buffer_version_states
                .entry(graph.buffer_version_create_usage(buffer_create.buffer))
                .or_default();

            if !version_state.try_merge(&buffer_create.constraint) {
                // Should not happen as this should be our first visit to this buffer
                panic!("Unexpected constraints on buffer being created");
            }

            log::trace!(
                "        Forward propagate constraints {:?} {:?}",
                buffer_create.buffer,
                version_state
            );

            // Don't bother setting usage constraint for 0
        }

        // We don't need to propagate anything forward on reads

        fn propagate_image_constraints_forward(
            graph: &RenderGraphBuilder,
            image_version_states: &mut FnvHashMap<
                RenderGraphImageUsageId,
                RenderGraphImageConstraint,
            >,
            input: RenderGraphImageUsageId,
            output: RenderGraphImageUsageId,
            constraint: &RenderGraphImageConstraint,
            operation_name: &str,
        ) {
            log::trace!(
                "      {} image {:?} {:?} -> {:?} {:?}",
                operation_name,
                input,
                graph.image_resource(input).name,
                output,
                graph.image_resource(output).name
            );

            //let image = graph.image_version_info(input);
            //log::trace!("  Modify image {:?} {:?}", input, graph.image_resource(input).name);
            let input_state = image_version_states
                .entry(graph.image_version_create_usage(input))
                .or_default();
            let mut image_modify_constraint = constraint.clone();

            // Merge the input image constraints with this node's constraints
            image_modify_constraint.partial_merge(&input_state);

            let output_state = image_version_states
                .entry(graph.image_version_create_usage(output))
                .or_default();

            // Now propagate forward to the image version we write
            output_state.partial_merge(&image_modify_constraint);

            log::trace!("        Forward propagate constraints {:?}", output_state);
        }

        fn propagate_buffer_constraints_forward(
            graph: &RenderGraphBuilder,
            buffer_version_states: &mut FnvHashMap<
                RenderGraphBufferUsageId,
                RenderGraphBufferConstraint,
            >,
            input: RenderGraphBufferUsageId,
            output: RenderGraphBufferUsageId,
            constraint: &RenderGraphBufferConstraint,
            operation_name: &str,
        ) {
            log::trace!(
                "      {} buffer {:?} {:?} -> {:?} {:?}",
                operation_name,
                input,
                graph.buffer_resource(input).name,
                output,
                graph.buffer_resource(output).name
            );

            //let buffer = graph.buffer_version_info(input);
            //log::trace!("  Modify buffer {:?} {:?}", input, graph.buffer_resource(input).name);
            let input_state = buffer_version_states
                .entry(graph.buffer_version_create_usage(input))
                .or_default();
            let mut buffer_modify_constraint = constraint.clone();

            // Merge the input buffer constraints with this node's constraints
            buffer_modify_constraint.partial_merge(&input_state);

            let output_state = buffer_version_states
                .entry(graph.buffer_version_create_usage(output))
                .or_default();

            // Now propagate forward to the buffer version we write
            output_state.partial_merge(&buffer_modify_constraint);

            log::trace!("        Forward propagate constraints {:?}", output_state);
        }

        //
        // Propagate constraints forward for images being modified.
        //
        for image_modify in &node.image_modifies {
            propagate_image_constraints_forward(
                graph,
                &mut image_version_states,
                image_modify.input,
                image_modify.output,
                &image_modify.constraint,
                "Modify",
            );
        }

        for image_modify in &node.image_copies {
            propagate_image_constraints_forward(
                graph,
                &mut image_version_states,
                image_modify.input,
                image_modify.output,
                &image_modify.constraint,
                "Modify",
            );
        }

        //
        // Propagate constraints forward for buffers being modified.
        //
        for buffer_modify in &node.buffer_modifies {
            propagate_buffer_constraints_forward(
                graph,
                &mut buffer_version_states,
                buffer_modify.input,
                buffer_modify.output,
                &buffer_modify.constraint,
                "Modify",
            );
        }

        for buffer_modify in &node.buffer_copies {
            propagate_buffer_constraints_forward(
                graph,
                &mut buffer_version_states,
                buffer_modify.input,
                buffer_modify.output,
                &buffer_modify.constraint,
                "Modify",
            );
        }
    }

    log::trace!("  Set up output images");

    //
    // Propagate output image state specifications into images
    //
    for external_image in &graph.external_images {
        if let Some(output_usage) = external_image.output_usage {
            log::trace!(
                "    Image {:?} {:?}",
                output_usage,
                graph.image_resource(output_usage).name
            );
            let output_image_version_state = image_version_states
                .entry(graph.image_version_create_usage(output_usage))
                .or_default();
            let output_constraint = external_image.specification.clone().into();
            output_image_version_state.partial_merge(&output_constraint);

            image_version_states.insert(output_usage, external_image.specification.clone().into());
        }
    }

    //
    // Propagate output buffer state specifications into buffers
    //
    for external_buffer in &graph.external_buffers {
        if let Some(output_usage) = external_buffer.output_usage {
            log::trace!(
                "    Buffer {:?} {:?}",
                output_usage,
                graph.buffer_resource(output_usage).name
            );
            let output_buffer_version_state = buffer_version_states
                .entry(graph.buffer_version_create_usage(output_usage))
                .or_default();
            let output_constraint = external_buffer.specification.clone().into();
            output_buffer_version_state.partial_merge(&output_constraint);

            buffer_version_states
                .insert(output_usage, external_buffer.specification.clone().into());
        }
    }

    log::trace!("  Propagate constraints BACKWARD");

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
                .entry(graph.image_version_create_usage(image_read.image))
                .or_default();
            version_state.partial_merge(&image_read.constraint);

            // If this is an image read with no output, it's possible the constraint on the read is incomplete.
            // So we need to merge the image state that may have information forward-propagated
            // into it with the constraints on the read. (Conceptually it's like we're forward
            // propagating here because the main forward propagate pass does not handle reads.
            // TODO: We could consider moving this to the forward pass
            let mut image_read_constraint = image_read.constraint.clone();
            image_read_constraint.partial_merge(&version_state);
            log::trace!(
                "        Read constraints will be {:?}",
                image_read_constraint
            );
            if let Some(spec) =
                image_read_constraint.try_convert_to_specification(swapchain_surface_info)
            {
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
        // Propagate backwards from reads
        //
        for buffer_read in &node.buffer_reads {
            log::trace!(
                "      Read buffer {:?} {:?}",
                buffer_read.buffer,
                graph.buffer_resource(buffer_read.buffer).name
            );

            let version_state = buffer_version_states
                .entry(graph.buffer_version_create_usage(buffer_read.buffer))
                .or_default();
            version_state.partial_merge(&buffer_read.constraint);

            // If this is a buffer read with no output, it's possible the constraint on the read is incomplete.
            // So we need to merge the buffer state that may have information forward-propagated
            // into it with the constraints on the read. (Conceptually it's like we're forward
            // propagating here because the main forward propagate pass does not handle reads.
            // TODO: We could consider moving this to the forward pass
            let mut buffer_read_constraint = buffer_read.constraint.clone();
            buffer_read_constraint.partial_merge(&version_state);
            log::trace!(
                "        Read constraints will be {:?}",
                buffer_read_constraint
            );
            if let Some(spec) = buffer_read_constraint.try_convert_to_specification() {
                buffer_version_states.insert(buffer_read.buffer, spec.into());
            } else {
                panic!(
                    "Not enough information in the graph to determine the specification for buffer {:?} {:?} being read by node {:?} {:?}. Constraints are: {:?}",
                    buffer_read.buffer,
                    graph.buffer_resource(buffer_read.buffer).name,
                    node.id(),
                    node.name(),
                    buffer_version_states.get(&buffer_read.buffer)
                );
            }
        }

        fn propagate_image_constraints_backward(
            graph: &RenderGraphBuilder,
            image_version_states: &mut FnvHashMap<
                RenderGraphImageUsageId,
                RenderGraphImageConstraint,
            >,
            input: RenderGraphImageUsageId,
            output: RenderGraphImageUsageId,
            operation_name: &str,
        ) {
            log::trace!(
                "      {} image {:?} {:?} <- {:?} {:?}",
                operation_name,
                input,
                graph.image_resource(input).name,
                output,
                graph.image_resource(output).name
            );
            // The output image constraint already takes constraint into account from
            // when we propagated image constraints forward
            let output_image_constraint = image_version_states
                .entry(graph.image_version_create_usage(output))
                .or_default()
                .clone();
            let input_state = image_version_states
                .entry(graph.image_version_create_usage(input))
                .or_default();
            input_state.partial_merge(&output_image_constraint);

            image_version_states.insert(input, output_image_constraint.clone());
        }

        fn propagate_buffer_constraints_backward(
            graph: &RenderGraphBuilder,
            buffer_version_states: &mut FnvHashMap<
                RenderGraphBufferUsageId,
                RenderGraphBufferConstraint,
            >,
            input: RenderGraphBufferUsageId,
            output: RenderGraphBufferUsageId,
            operation_name: &str,
        ) {
            log::trace!(
                "      {} buffer {:?} {:?} <- {:?} {:?}",
                operation_name,
                input,
                graph.buffer_resource(input).name,
                output,
                graph.buffer_resource(output).name
            );
            // The output buffer constraint already takes constraint into account from
            // when we propagated buffer constraints forward
            let output_buffer_constraint = buffer_version_states
                .entry(graph.buffer_version_create_usage(output))
                .or_default()
                .clone();
            let input_state = buffer_version_states
                .entry(graph.buffer_version_create_usage(input))
                .or_default();
            input_state.partial_merge(&output_buffer_constraint);

            buffer_version_states.insert(input, output_buffer_constraint.clone());
        }

        //
        // Propagate backwards from modifies
        //
        for image_modify in &node.image_modifies {
            propagate_image_constraints_backward(
                graph,
                &mut image_version_states,
                image_modify.input,
                image_modify.output,
                "Modify",
            );
        }

        for image_copy in &node.image_copies {
            propagate_image_constraints_backward(
                graph,
                &mut image_version_states,
                image_copy.input,
                image_copy.output,
                "Copy",
            );
        }

        for buffer_modify in &node.buffer_modifies {
            propagate_buffer_constraints_backward(
                graph,
                &mut buffer_version_states,
                buffer_modify.input,
                buffer_modify.output,
                "Modify",
            );
        }

        for buffer_copy in &node.buffer_copies {
            propagate_buffer_constraints_backward(
                graph,
                &mut buffer_version_states,
                buffer_copy.input,
                buffer_copy.output,
                "Copy",
            );
        }
    }

    let mut image_specs = FnvHashMap::default();
    for (k, v) in image_version_states {
        image_specs.insert(
            k,
            v.try_convert_to_specification(swapchain_surface_info)
                .unwrap(),
        );
    }

    let mut buffer_specs = FnvHashMap::default();
    for (k, v) in buffer_version_states {
        buffer_specs.insert(k, v.try_convert_to_specification().unwrap());
    }

    DetermineConstraintsResult {
        images: image_specs,
        buffers: buffer_specs,
    }
}

//
// This function finds places where an image needs to transition from multisampled to non-multisampled.
// This can be done efficiently by adding a resolve attachment to the pass. These resolves are
// automatically inserted. This only works for color attachments (limitation of vulkan)
//
#[profiling::function]
fn insert_resolves(
    graph: &mut RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    constraint_results: &mut DetermineConstraintsResult,
) {
    log::trace!("Insert resolves in graph where necessary");
    for node_id in node_execution_order {
        let mut resolves_to_add = Vec::default();

        let node = graph.node(*node_id);
        log::trace!("  node {:?} {:?}", node_id, node.name);
        // Iterate through all color attachments
        for (color_attachment_index, color_attachment) in node.color_attachments.iter().enumerate()
        {
            if let Some(color_attachment) = color_attachment {
                log::trace!("    color attachment {}", color_attachment_index);
                // If this color attachment outputs an image
                if let Some(write_image) = color_attachment.write_image {
                    //let write_version = graph.image_usages[write_image.0].version;
                    // Skip if it's not an MSAA image
                    let write_spec = constraint_results.image_specification(write_image).unwrap();
                    if write_spec.samples == RafxSampleCount::SampleCount1 {
                        log::trace!("      already non-MSAA");
                        continue;
                    }

                    // Calculate the spec that we would have after the resolve
                    let mut resolve_spec = write_spec.clone();
                    resolve_spec.samples = RafxSampleCount::SampleCount1;

                    let mut usages_to_move = vec![];

                    // Look for any usages we need to fix
                    for (usage_index, read_usage) in graph
                        .image_version_info(write_image)
                        .read_usages
                        .iter()
                        .enumerate()
                    {
                        log::trace!(
                            "      usage {}, {:?} {:?}",
                            usage_index,
                            read_usage,
                            graph.image_usages[read_usage.0].usage_type
                        );
                        let read_spec =
                            constraint_results.image_specification(*read_usage).unwrap();
                        if *read_spec == *write_spec {
                            continue;
                        } else if *read_spec == resolve_spec {
                            usages_to_move.push(*read_usage);
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
                Default::default(),
            );
            constraint_results.images.insert(image, resolve_spec);

            for usage in usages_to_move {
                let from = graph.image_usages[usage.0].version;
                let to = graph.image_usages[image.0].version;
                log::trace!(
                    "          MOVE USAGE {:?} from {:?} to {:?}",
                    usage,
                    from,
                    to
                );
                graph.redirect_image_usage(usage, from, to)
            }
        }
    }
}

/// Assignment of usages to actual images. This allows a single image to be passed through a
/// sequence of reads and writes
#[derive(Debug)]
pub struct AssignVirtualResourcesResult {
    image_usage_to_virtual: FnvHashMap<RenderGraphImageUsageId, VirtualImageId>,
    buffer_usage_to_virtual: FnvHashMap<RenderGraphBufferUsageId, VirtualBufferId>,
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

#[derive(Default)]
struct VirtualBufferIdAllocator {
    next_id: usize,
}

impl VirtualBufferIdAllocator {
    fn allocate(&mut self) -> VirtualBufferId {
        let id = VirtualBufferId(self.next_id);
        self.next_id += 1;
        id
    }
}

//
// The graph is built with the assumption that every image is immutable. However in most cases we
// can easily pass the same image through multiple passes saving memory and the need to copy data.
// This function finds places where we can trivially forward an image from one pass to another. In
// the future, cases where this is not possible might be handled by copying the image. (Needed if
// there are multiple downstream consumers modifying the image or if the format needs to change.
//
#[profiling::function]
fn assign_virtual_resources(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    constraint_results: &mut DetermineConstraintsResult,
) -> AssignVirtualResourcesResult {
    let mut image_usage_to_virtual: FnvHashMap<RenderGraphImageUsageId, VirtualImageId> =
        FnvHashMap::default();
    let mut buffer_usage_to_virtual: FnvHashMap<RenderGraphBufferUsageId, VirtualBufferId> =
        FnvHashMap::default();

    let mut virtual_image_id_allocator = VirtualImageIdAllocator::default();
    let mut virtual_buffer_id_allocator = VirtualBufferIdAllocator::default();

    log::trace!("Associate input images with virtual images");
    for external_image in &graph.external_images {
        if let Some(input_usage) = external_image.input_usage {
            // Assign the image
            let virtual_image = virtual_image_id_allocator.allocate();
            log::trace!(
                "    External image {:?} used as input will use image {:?}",
                input_usage,
                virtual_image
            );
            image_usage_to_virtual.insert(input_usage, virtual_image);

            // Try to share the image forward to downstream consumers
            propagate_virtual_image_id(
                graph,
                constraint_results,
                &mut image_usage_to_virtual,
                &mut virtual_image_id_allocator,
                input_usage,
            );
        }
    }

    log::trace!("Associate input buffers with virtual buffers");
    for external_buffer in &graph.external_buffers {
        if let Some(input_usage) = external_buffer.input_usage {
            // Assign the buffer
            let virtual_buffer = virtual_buffer_id_allocator.allocate();
            log::trace!(
                "    External buffer {:?} used as input will use buffer {:?}",
                external_buffer.external_buffer_id,
                virtual_buffer
            );
            buffer_usage_to_virtual.insert(input_usage, virtual_buffer);

            // Try to share the buffer forward to downstream consumers
            propagate_virtual_buffer_id(
                graph,
                constraint_results,
                &mut buffer_usage_to_virtual,
                &mut virtual_buffer_id_allocator,
                input_usage,
            );
        }
    }

    //TODO: Associate input images here? We can wait until we decide which images are shared
    log::trace!("Associate images written by nodes with virtual images");
    for node in node_execution_order.iter() {
        let node = graph.node(*node);
        log::trace!("  node {:?} {:?}", node.id().0, node.name());

        // A list of all images we write to from this node. We will try to share the images
        // being written forward into the nodes of downstream reads. This can chain such that
        // the same image is shared by many nodes

        //
        // Handle images created by this node
        //
        for image_create in &node.image_creates {
            // An image that's created always allocates an image (we reuse these if they are compatible
            // and lifetimes don't overlap)
            let virtual_image = virtual_image_id_allocator.allocate();
            log::trace!(
                "    Create {:?} will use image {:?}",
                image_create.image,
                virtual_image
            );
            image_usage_to_virtual.insert(image_create.image, virtual_image);
            // Queue this image write to try to share the image forward
            propagate_virtual_image_id(
                graph,
                constraint_results,
                &mut image_usage_to_virtual,
                &mut virtual_image_id_allocator,
                image_create.image,
            );
        }

        //
        // Handle buffers created by this node
        //
        for buffer_create in &node.buffer_creates {
            // A buffer that's created always allocates a buffer (we reuse these if they are compatible
            // and lifetimes don't overlap)
            let virtual_buffer = virtual_buffer_id_allocator.allocate();
            log::trace!(
                "    Create {:?} will use buffer {:?}",
                buffer_create.buffer,
                virtual_buffer
            );
            buffer_usage_to_virtual.insert(buffer_create.buffer, virtual_buffer);
            // Try to share the buffer forward to downstream consumers
            propagate_virtual_buffer_id(
                graph,
                constraint_results,
                &mut buffer_usage_to_virtual,
                &mut virtual_buffer_id_allocator,
                buffer_create.buffer,
            );
        }

        //
        // Handle images modified by this node
        //
        for image_modify in &node.image_modifies {
            // The virtual image in the read portion of a image_modify must also be the write image.
            // The format of the input/output is guaranteed to match
            assert_eq!(
                constraint_results.image_specification(image_modify.input),
                constraint_results.image_specification(image_modify.output)
            );

            // Assign the image
            let virtual_image = *image_usage_to_virtual.get(&image_modify.input).unwrap();
            log::trace!(
                "    Modify {:?} will pass through image {:?}",
                image_modify.output,
                virtual_image
            );
            image_usage_to_virtual.insert(image_modify.output, virtual_image);

            // Queue this image write to try to share the image forward
            propagate_virtual_image_id(
                graph,
                constraint_results,
                &mut image_usage_to_virtual,
                &mut virtual_image_id_allocator,
                image_modify.output,
            );
        }

        //
        // Handle buffers modified by this node
        //
        for buffer_modify in &node.buffer_modifies {
            // The virtual buffer in the read portion of a buffer_modify must also be the write buffer.
            // The format of the input/output is guaranteed to match
            assert_eq!(
                constraint_results.buffer_specification(buffer_modify.input),
                constraint_results.buffer_specification(buffer_modify.output)
            );

            // Assign the buffer
            let virtual_buffer = *buffer_usage_to_virtual.get(&buffer_modify.input).unwrap();
            log::trace!(
                "    Modify {:?} will pass through buffer {:?}",
                buffer_modify.output,
                virtual_buffer
            );
            buffer_usage_to_virtual.insert(buffer_modify.output, virtual_buffer);

            // Try to share the buffer forward to downstream consumers
            propagate_virtual_buffer_id(
                graph,
                constraint_results,
                &mut buffer_usage_to_virtual,
                &mut virtual_buffer_id_allocator,
                buffer_modify.output,
            );
        }
    }

    // vulkan image layouts: https://github.com/nannou-org/nannou/issues/271#issuecomment-465876622
    AssignVirtualResourcesResult {
        image_usage_to_virtual,
        buffer_usage_to_virtual,
    }
}

fn propagate_virtual_image_id(
    graph: &RenderGraphBuilder,
    constraint_results: &DetermineConstraintsResult,
    image_usage_to_virtual: &mut FnvHashMap<RenderGraphImageUsageId, VirtualImageId>,
    virtual_image_id_allocator: &mut VirtualImageIdAllocator,
    written_image: RenderGraphImageUsageId,
) {
    // Count the downstream users of this image based on if they need read-only access
    // or write access. We need this information to determine which usages we can share
    // the output data with.
    //
    // I'm not sure if this works as written. I was thinking we might have trouble with
    // multiple readers, and then they pass to a writer, but now that I think of it, readers
    // don't "output" anything.
    //
    // That said, this doesn't understand multiple writers of different subresources right
    // now.
    //
    //TODO: This could be smarter to handle the case of a resource being read and then
    // later written
    //TODO: Could handle non-overlapping subresource ranges being written
    let written_image_version_info = graph.image_version_info(written_image);
    let mut read_count = 0;
    //let mut read_ranges = vec![];
    let mut write_count = 0;
    //let mut write_ranges = vec![];
    for usage in &written_image_version_info.read_usages {
        if graph.image_usages[usage.0].usage_type.is_read_only() {
            read_count += 1;
            //read_ranges.push(graph.image_usages[usage.0].subresource_range.clone());
        } else {
            write_count += 1;
            //write_ranges.push(graph.image_usages[usage.0].subresource_range.clone());
        }
    }

    // let mut has_overlapping_write = false;
    // for i in 0..write_ranges.len() {
    //     for j in 0..i {
    //
    //     }
    // }

    let write_virtual_image = *image_usage_to_virtual.get(&written_image).unwrap();
    let write_type = graph.image_usages[written_image.0].usage_type;

    let written_spec = constraint_results
        .image_specification(written_image)
        .unwrap();

    for usage_resource_id in &written_image_version_info.read_usages {
        let usage_spec = match constraint_results.image_specification(*usage_resource_id) {
            Some(usage_spec) => usage_spec,
            // If the reader of this image was culled, we may not have determined a spec.
            // If so, skip this usage
            None => continue,
        };

        // We can't share images if they aren't the same format
        let specifications_compatible =
            RenderGraphImageSpecification::specifications_are_compatible(written_spec, usage_spec);

        // We can't share images unless it's a read or it's an exclusive write
        let is_read_or_exclusive_write = (read_count > 0
            && graph.image_usages[usage_resource_id.0]
                .usage_type
                .is_read_only())
            || write_count <= 1;

        let read_type = graph.image_usages[usage_resource_id.0].usage_type;
        if specifications_compatible && is_read_or_exclusive_write {
            // it's a shared read or an exclusive write
            log::trace!(
                "    Usage {:?} will share an image with {:?} ({:?} -> {:?})",
                written_image,
                usage_resource_id,
                write_type,
                read_type
            );
            let overwritten_image =
                image_usage_to_virtual.insert(*usage_resource_id, write_virtual_image);

            assert!(overwritten_image.is_none());
        } else {
            // allocate new image
            let virtual_image = virtual_image_id_allocator.allocate();
            log::info!(
                "    Allocate image {:?} for {:?} ({:?} -> {:?})  (specifications_compatible match: {} is_read_or_exclusive_write: {})",
                virtual_image,
                usage_resource_id,
                write_type,
                read_type,
                specifications_compatible,
                is_read_or_exclusive_write
            );
            let overwritten_image =
                image_usage_to_virtual.insert(*usage_resource_id, virtual_image);

            assert!(overwritten_image.is_none());

            //TODO: One issue (aside from not doing any blits right now) is that images created in this way
            // aren't included in the assign_physical_images logic

            log::info!(
                "      writer     : {}",
                graph.debug_user_name_of_image_usage(written_image)
            );
            log::info!(
                "      reader     : {}",
                graph.debug_user_name_of_image_usage(*usage_resource_id)
            );

            if !specifications_compatible {
                log::info!("      writer spec: {:?}", written_spec);
                log::info!("      reader spec: {:?}", usage_spec);
            }
            log::info!("      --- All Usages ---");
            log::info!(
                "      Creator: {} spec: {:?}",
                graph.debug_user_name_of_image_usage(written_image),
                constraint_results.image_specification(written_image)
            );
            for &read_usage in &written_image_version_info.read_usages {
                log::info!(
                    "        Reader: {} spec: {:?}",
                    graph.debug_user_name_of_image_usage(read_usage),
                    constraint_results.image_specification(read_usage)
                );
            }

            panic!("The render graph contains an image conflict that cannot be automatically resolved.");
        }
    }
}

fn propagate_virtual_buffer_id(
    graph: &RenderGraphBuilder,
    constraint_results: &DetermineConstraintsResult,
    buffer_usage_to_virtual: &mut FnvHashMap<RenderGraphBufferUsageId, VirtualBufferId>,
    virtual_buffer_id_allocator: &mut VirtualBufferIdAllocator,
    written_buffer: RenderGraphBufferUsageId,
) {
    // Count the downstream users of this image based on if they need read-only access
    // or write access. We need this information to determine which usages we can share
    // the output data with.
    //
    // I'm not sure if this works as written. I was thinking we might have trouble with
    // multiple readers, and then they pass to a writer, but now that I think of it, readers
    // don't "output" anything.
    //TODO: This could be smarter to handle the case of a resource being read and then
    // later written

    let written_buffer_version_info = graph.buffer_version_info(written_buffer);
    let mut read_count = 0;
    let mut write_count = 0;
    for usage in &written_buffer_version_info.read_usages {
        if graph.buffer_usages[usage.0].usage_type.is_read_only() {
            read_count += 1;
        } else {
            write_count += 1;
        }
    }

    let write_virtual_buffer = *buffer_usage_to_virtual.get(&written_buffer).unwrap();
    let write_type = graph.buffer_usages[written_buffer.0].usage_type;

    let written_spec = constraint_results
        .buffer_specification(written_buffer)
        .unwrap();

    for usage_resource_id in &written_buffer_version_info.read_usages {
        let usage_spec = match constraint_results.buffer_specification(*usage_resource_id) {
            Some(usage_spec) => usage_spec,
            // If the reader of this buffer was culled, we may not have determined a spec.
            // If so, skip this usage
            None => continue,
        };

        // We can't share buffers if they aren't the same format
        let specifications_compatible =
            RenderGraphBufferSpecification::specifications_are_compatible(written_spec, usage_spec);

        // We can't share buffers unless it's a read or it's an exclusive write
        let is_read_or_exclusive_write = (read_count > 0
            && graph.buffer_usages[usage_resource_id.0]
                .usage_type
                .is_read_only())
            || write_count <= 1;

        let read_type = graph.buffer_usages[usage_resource_id.0].usage_type;
        if specifications_compatible && is_read_or_exclusive_write {
            // it's a shared read or an exclusive write
            log::trace!(
                "    Usage {:?} will share a buffer with {:?} ({:?} -> {:?})",
                written_buffer,
                usage_resource_id,
                write_type,
                read_type
            );
            let overwritten_buffer =
                buffer_usage_to_virtual.insert(*usage_resource_id, write_virtual_buffer);

            assert!(overwritten_buffer.is_none());
        } else {
            // allocate new buffer
            let virtual_buffer = virtual_buffer_id_allocator.allocate();
            log::info!(
                "    Allocate buffer {:?} for {:?} ({:?} -> {:?})  (specifications_compatible match: {} is_read_or_exclusive_write: {})",
                virtual_buffer,
                usage_resource_id,
                write_type,
                read_type,
                specifications_compatible,
                is_read_or_exclusive_write
            );

            let overwritten_buffer =
                buffer_usage_to_virtual.insert(*usage_resource_id, virtual_buffer);

            assert!(overwritten_buffer.is_none());

            //TODO: One issue (aside from not doing any copies right now) is that buffers created in this way
            // aren't included in the assign_physical_buffers logic

            log::info!(
                "      writer     : {}",
                graph.debug_user_name_of_buffer_usage(written_buffer)
            );
            log::info!(
                "      reader     : {}",
                graph.debug_user_name_of_buffer_usage(*usage_resource_id)
            );

            if !specifications_compatible {
                log::info!("      writer spec: {:?}", written_spec);
                log::info!("      reader spec: {:?}", usage_spec);
            }
            log::info!("      --- All Usages ---");
            log::info!(
                "      Creator: {} spec {:?}",
                graph.debug_user_name_of_buffer_usage(written_buffer),
                constraint_results.buffer_specification(written_buffer)
            );
            for &read_usage in &written_buffer_version_info.read_usages {
                log::info!(
                    "        Reader: {} spec {:?}",
                    graph.debug_user_name_of_buffer_usage(read_usage),
                    constraint_results.buffer_specification(read_usage)
                );
            }

            panic!("The render graph contains a buffer conflict that cannot be automatically resolved.");
        }
    }
}

//
// This walks through the nodes and creates passes/subpasses. Most of the info to create them is
// determined here along with stage/access/queue family barrier info. (The barrier info is used
// later.. some of the invalidates/flushes can be merged.)
//
#[profiling::function]
fn build_physical_passes(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    constraints: &DetermineConstraintsResult,
    virtual_resources: &AssignVirtualResourcesResult,
) -> Vec<RenderGraphPass> {
    #[derive(Debug)]
    enum PassNode {
        RenderpassNode(RenderGraphNodeId),
        CallbackNode(RenderGraphNodeId),
    }

    // All passes
    let mut pass_nodes = Vec::default();

    for node_id in node_execution_order {
        let pass_node = match graph.node(*node_id).kind {
            RenderGraphNodeKind::Renderpass => PassNode::RenderpassNode(*node_id),
            RenderGraphNodeKind::Callback => PassNode::CallbackNode(*node_id),
        };
        pass_nodes.push(pass_node);
    }

    log::trace!("gather pass info");
    let mut passes = Vec::default();
    for pass_node in pass_nodes {
        log::trace!("  nodes in pass: {:?}", pass_node);

        fn find_or_insert_attachment(
            attachments: &mut Vec<RenderGraphPassAttachment>,
            usage: RenderGraphImageUsageId,
            virtual_image: VirtualImageId,
        ) -> (usize, bool) {
            if let Some(position) = attachments
                .iter()
                .position(|x| x.virtual_image == virtual_image)
            {
                (position, false)
            } else {
                attachments.push(RenderGraphPassAttachment {
                    usage,
                    virtual_image,

                    //NOTE: These get assigned later in assign_physical_images
                    image: None,
                    image_view: None,

                    load_op: RafxLoadOp::DontCare,
                    stencil_load_op: RafxLoadOp::DontCare,
                    store_op: RafxStoreOp::DontCare,
                    stencil_store_op: RafxStoreOp::DontCare,
                    clear_color: None,
                    format: RafxFormat::UNDEFINED,
                    samples: RafxSampleCount::SampleCount1,

                    // NOTE: These get assigned later in build_pass_barriers
                    initial_state: RafxResourceState::UNDEFINED,
                    final_state: RafxResourceState::UNDEFINED,
                });
                (attachments.len() - 1, true)
            }
        }

        match pass_node {
            PassNode::CallbackNode(compute_node) => {
                passes.push(RenderGraphPass::Callback(RenderGraphCallbackPass {
                    node: compute_node,
                    pre_pass_barrier: Default::default(),
                }));
            }
            PassNode::RenderpassNode(renderpass_node) => {
                let mut renderpass_attachments = Vec::default();

                log::trace!("    subpass node: {:?}", renderpass_node);
                let subpass_node = graph.node(renderpass_node);

                // Don't create a subpass if there are no attachments
                if subpass_node.color_attachments.is_empty()
                    && subpass_node.depth_attachment.is_none()
                {
                    assert!(subpass_node.resolve_attachments.is_empty());
                    log::trace!("      Not generating a subpass - no attachments");
                    continue;
                }

                let mut pass_color_attachments: [Option<usize>; MAX_COLOR_ATTACHMENTS] =
                    Default::default();
                let mut pass_resolve_attachments: [Option<usize>; MAX_COLOR_ATTACHMENTS] =
                    Default::default();
                let mut pass_depth_attachment = Default::default();

                for (color_attachment_index, color_attachment) in
                    subpass_node.color_attachments.iter().enumerate()
                {
                    if let Some(color_attachment) = color_attachment {
                        let read_or_write_usage = color_attachment
                            .read_image
                            .or(color_attachment.write_image)
                            .unwrap();
                        let virtual_image = virtual_resources
                            .image_usage_to_virtual
                            .get(&read_or_write_usage)
                            .unwrap();

                        let specification = constraints.images.get(&read_or_write_usage).unwrap();
                        log::trace!("      virtual attachment (color): {:?}", virtual_image);

                        let (pass_attachment_index, is_first_usage) = find_or_insert_attachment(
                            &mut renderpass_attachments,
                            read_or_write_usage,
                            *virtual_image, /*, subresource_range*/
                        );
                        pass_color_attachments[color_attachment_index] =
                            Some(pass_attachment_index);

                        let attachment = &mut renderpass_attachments[pass_attachment_index];
                        if is_first_usage {
                            // Check if we load or clear
                            if color_attachment.clear_color_value.is_some() {
                                attachment.load_op = RafxLoadOp::Clear;
                                attachment.clear_color = Some(AttachmentClearValue::Color(
                                    color_attachment.clear_color_value.unwrap(),
                                ))
                            } else if color_attachment.read_image.is_some() {
                                attachment.load_op = RafxLoadOp::Load;
                            }

                            attachment.format = specification.format.into();
                            attachment.samples = specification.samples.into();
                        };

                        let store_op = if let Some(write_image) = color_attachment.write_image {
                            if !graph.image_version_info(write_image).read_usages.is_empty() {
                                RafxStoreOp::Store
                            } else {
                                RafxStoreOp::DontCare
                            }
                        } else {
                            RafxStoreOp::DontCare
                        };

                        attachment.store_op = store_op;
                        attachment.stencil_store_op = RafxStoreOp::DontCare;
                    }
                }

                for (resolve_attachment_index, resolve_attachment) in
                    subpass_node.resolve_attachments.iter().enumerate()
                {
                    if let Some(resolve_attachment) = resolve_attachment {
                        let write_image = resolve_attachment.write_image;
                        let virtual_image = virtual_resources
                            .image_usage_to_virtual
                            .get(&write_image)
                            .unwrap();
                        //let version_id = graph.image_version_id(write_image);
                        let specification = constraints.images.get(&write_image).unwrap();
                        log::trace!("      virtual attachment (resolve): {:?}", virtual_image);

                        let (pass_attachment_index, is_first_usage) = find_or_insert_attachment(
                            &mut renderpass_attachments,
                            write_image,
                            *virtual_image, /*, subresource_range*/
                        );
                        pass_resolve_attachments[resolve_attachment_index] =
                            Some(pass_attachment_index);

                        assert!(is_first_usage); // Not sure if this assert is valid
                        let attachment = &mut renderpass_attachments[pass_attachment_index];
                        attachment.format = specification.format.into();
                        attachment.samples = specification.samples.into();

                        //TODO: Should we skip resolving if there is no reader?
                        let store_op =
                            if !graph.image_version_info(write_image).read_usages.is_empty() {
                                RafxStoreOp::Store
                            } else {
                                RafxStoreOp::DontCare
                            };

                        attachment.store_op = store_op;
                        attachment.stencil_store_op = RafxStoreOp::DontCare;
                    }
                }

                if let Some(depth_attachment) = &subpass_node.depth_attachment {
                    let read_or_write_usage = depth_attachment
                        .read_image
                        .or(depth_attachment.write_image)
                        .unwrap();
                    let virtual_image = virtual_resources
                        .image_usage_to_virtual
                        .get(&read_or_write_usage)
                        .unwrap();
                    let specification = constraints.images.get(&read_or_write_usage).unwrap();
                    log::trace!("      virtual attachment (depth): {:?}", virtual_image);

                    let (pass_attachment_index, is_first_usage) = find_or_insert_attachment(
                        &mut renderpass_attachments,
                        read_or_write_usage,
                        *virtual_image, /*, subresource_range*/
                    );
                    pass_depth_attachment = Some(pass_attachment_index);

                    let attachment = &mut renderpass_attachments[pass_attachment_index];
                    if is_first_usage {
                        // Check if we load or clear
                        //TODO: Support load_op for stencil

                        if depth_attachment.clear_depth_stencil_value.is_some() {
                            if depth_attachment.has_depth {
                                attachment.load_op = RafxLoadOp::Clear;
                            }
                            if depth_attachment.has_stencil {
                                attachment.stencil_load_op = RafxLoadOp::Clear;
                            }
                            attachment.clear_color = Some(AttachmentClearValue::DepthStencil(
                                depth_attachment.clear_depth_stencil_value.unwrap(),
                            ));
                        } else if depth_attachment.read_image.is_some() {
                            if depth_attachment.has_depth {
                                attachment.load_op = RafxLoadOp::Load;
                            }

                            if depth_attachment.has_stencil {
                                attachment.stencil_load_op = RafxLoadOp::Load;
                            }
                        }

                        attachment.format = specification.format.into();
                        attachment.samples = specification.samples.into();
                    };

                    let store_op = if let Some(write_image) = depth_attachment.write_image {
                        if !graph.image_version_info(write_image).read_usages.is_empty() {
                            RafxStoreOp::Store
                        } else {
                            RafxStoreOp::DontCare
                        }
                    } else {
                        RafxStoreOp::DontCare
                    };

                    if depth_attachment.has_depth {
                        attachment.store_op = store_op;
                    }

                    if depth_attachment.has_stencil {
                        attachment.stencil_store_op = store_op;
                    }
                }

                passes.push(RenderGraphPass::Render(RenderGraphRenderPass {
                    node_id: renderpass_node,
                    attachments: renderpass_attachments,
                    color_attachments: pass_color_attachments,
                    depth_attachment: pass_depth_attachment,
                    resolve_attachments: pass_resolve_attachments,
                    pre_pass_barrier: None,
                }));
            }
        }
    }

    passes
}

#[derive(Debug)]
struct AssignPhysicalResourcesResult {
    image_usage_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId>,
    image_usage_to_image_view: FnvHashMap<RenderGraphImageUsageId, PhysicalImageViewId>,
    image_views: Vec<RenderGraphImageView>, // indexed by physical image view id
    #[allow(unused)]
    image_virtual_to_physical: FnvHashMap<VirtualImageId, PhysicalImageId>,
    image_specifications: Vec<RenderGraphImageSpecification>, // indexed by physical image id

    #[allow(unused)]
    buffer_usage_to_physical: FnvHashMap<RenderGraphBufferUsageId, PhysicalBufferId>,
    #[allow(unused)]
    buffer_virtual_to_physical: FnvHashMap<VirtualBufferId, PhysicalBufferId>,
    buffer_specifications: Vec<RenderGraphBufferSpecification>, // indexed by physical image id
}

//
// This function walks through all the passes and creates a minimal list of images/buffers, potentially
// reusing a resource for multiple purposes during the execution of the graph. (Only if the lifetimes
// of those usages don't overlap!) For example, if we do a series of blurs, we can collapse those
// image usages into ping-ponging back and forth between two images.
//
#[profiling::function]
fn assign_physical_resources(
    graph: &RenderGraphBuilder,
    constraints: &DetermineConstraintsResult,
    virtual_resources: &AssignVirtualResourcesResult,
    passes: &mut [RenderGraphPass],
) -> AssignPhysicalResourcesResult {
    log::trace!("-- Assign physical resources --");
    struct PhysicalImageReuseRequirements {
        virtual_id: VirtualImageId,
        specification: RenderGraphImageSpecification,
        first_node_pass_index: usize,
        last_node_pass_index: usize,
    }

    struct PhysicalBufferReuseRequirements {
        virtual_id: VirtualBufferId,
        specification: RenderGraphBufferSpecification,
        first_node_pass_index: usize,
        last_node_pass_index: usize,
    }

    //
    // This inner function is responsible for populating reuse_requirements and
    // reuse_requirements_lookup. The goal here is to determine the lifetimes of all virtual images
    //
    fn add_or_modify_reuse_image_requirements(
        virtual_resources: &AssignVirtualResourcesResult,
        constraints: &DetermineConstraintsResult,
        pass_index: usize,
        usage: RenderGraphImageUsageId,
        reuse_requirements: &mut Vec<PhysicalImageReuseRequirements>,
        reuse_requirements_lookup: &mut FnvHashMap<VirtualImageId, usize>,
    ) {
        // Get physical ID from usage
        let virtual_id = virtual_resources.image_usage_to_virtual[&usage];

        // Find requirements for this image if they exist, or create new requirements. This is a
        // lookup for an index so that the requirements will be stored sorted by
        // first_node_pass_index for iteration later
        let reused_image_requirements_index = *reuse_requirements_lookup
            .entry(virtual_id)
            .or_insert_with(|| {
                let reused_image_requirements_index = reuse_requirements.len();
                let specification = &constraints.images[&usage];
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

    fn add_or_modify_reuse_buffer_requirements(
        virtual_resources: &AssignVirtualResourcesResult,
        constraints: &DetermineConstraintsResult,
        pass_index: usize,
        usage: RenderGraphBufferUsageId,
        reuse_requirements: &mut Vec<PhysicalBufferReuseRequirements>,
        reuse_requirements_lookup: &mut FnvHashMap<VirtualBufferId, usize>,
    ) {
        // Get physical ID from usage
        let virtual_id = virtual_resources.buffer_usage_to_virtual[&usage];

        // Find requirements for this buffer if they exist, or create new requirements. This is a
        // lookup for an index so that the requirements will be stored sorted by
        // first_node_pass_index for iteration later
        let reused_buffer_requirements_index = *reuse_requirements_lookup
            .entry(virtual_id)
            .or_insert_with(|| {
                let reused_buffer_requirements_index = reuse_requirements.len();
                let specification = &constraints.buffers[&usage];
                reuse_requirements.push(PhysicalBufferReuseRequirements {
                    virtual_id,
                    first_node_pass_index: pass_index,
                    last_node_pass_index: pass_index,
                    specification: specification.clone(),
                });

                log::trace!("  Add requirement {:?} {:?}", virtual_id, specification);
                reused_buffer_requirements_index
            });

        // Update the last pass index
        reuse_requirements[reused_buffer_requirements_index].last_node_pass_index = pass_index;
    }

    let mut image_reuse_requirements = Vec::<PhysicalImageReuseRequirements>::default();
    let mut image_reuse_requirements_lookup = FnvHashMap::<VirtualImageId, usize>::default();
    let mut buffer_reuse_requirements = Vec::<PhysicalBufferReuseRequirements>::default();
    let mut buffer_reuse_requirements_lookup = FnvHashMap::<VirtualBufferId, usize>::default();

    //
    // Walk through all image/buffer usages to determine their lifetimes
    //
    for (pass_index, pass) in passes.iter().enumerate() {
        let subpass_node_id = pass.node();
        let node = graph.node(subpass_node_id);

        for image_modify in &node.image_modifies {
            add_or_modify_reuse_image_requirements(
                virtual_resources,
                constraints,
                pass_index,
                image_modify.input,
                &mut image_reuse_requirements,
                &mut image_reuse_requirements_lookup,
            );
            add_or_modify_reuse_image_requirements(
                virtual_resources,
                constraints,
                pass_index,
                image_modify.output,
                &mut image_reuse_requirements,
                &mut image_reuse_requirements_lookup,
            );
        }

        for image_read in &node.image_reads {
            add_or_modify_reuse_image_requirements(
                virtual_resources,
                constraints,
                pass_index,
                image_read.image,
                &mut image_reuse_requirements,
                &mut image_reuse_requirements_lookup,
            );
        }

        for image_create in &node.image_creates {
            add_or_modify_reuse_image_requirements(
                virtual_resources,
                constraints,
                pass_index,
                image_create.image,
                &mut image_reuse_requirements,
                &mut image_reuse_requirements_lookup,
            );
        }

        for image_sample in &node.sampled_images {
            add_or_modify_reuse_image_requirements(
                virtual_resources,
                constraints,
                pass_index,
                *image_sample,
                &mut image_reuse_requirements,
                &mut image_reuse_requirements_lookup,
            );
        }

        for buffer_modify in &node.buffer_modifies {
            add_or_modify_reuse_buffer_requirements(
                virtual_resources,
                constraints,
                pass_index,
                buffer_modify.input,
                &mut buffer_reuse_requirements,
                &mut buffer_reuse_requirements_lookup,
            );
            add_or_modify_reuse_buffer_requirements(
                virtual_resources,
                constraints,
                pass_index,
                buffer_modify.output,
                &mut buffer_reuse_requirements,
                &mut buffer_reuse_requirements_lookup,
            );
        }

        for buffer_read in &node.buffer_reads {
            add_or_modify_reuse_buffer_requirements(
                virtual_resources,
                constraints,
                pass_index,
                buffer_read.buffer,
                &mut buffer_reuse_requirements,
                &mut buffer_reuse_requirements_lookup,
            );
        }

        for buffer_create in &node.buffer_creates {
            add_or_modify_reuse_buffer_requirements(
                virtual_resources,
                constraints,
                pass_index,
                buffer_create.buffer,
                &mut buffer_reuse_requirements,
                &mut buffer_reuse_requirements_lookup,
            );
        }
    }

    //TODO: Find transients
    //TODO: Mark input images as non-reuse?
    //TODO: Stay in same queue?

    #[derive(Debug, PartialEq)]
    struct PhysicalImage {
        specification: RenderGraphImageSpecification,
        last_node_pass_index: usize,
        can_be_reused: bool,
    }

    #[derive(Debug, PartialEq)]
    struct PhysicalBuffer {
        specification: RenderGraphBufferSpecification,
        last_node_pass_index: usize,
        can_be_reused: bool,
    }

    let mut physical_images = Vec::<PhysicalImage>::default();
    let mut image_virtual_to_physical = FnvHashMap::<VirtualImageId, PhysicalImageId>::default();
    let mut physical_buffers = Vec::<PhysicalBuffer>::default();
    let mut buffer_virtual_to_physical = FnvHashMap::<VirtualBufferId, PhysicalBufferId>::default();

    //
    // Allocate physical IDs for all input images/buffers
    //
    for external_image in &graph.external_images {
        if let Some(input_usage) = external_image.input_usage {
            let virtual_id = virtual_resources.image_usage_to_virtual[&input_usage];
            let physical_image = PhysicalImage {
                specification: external_image.specification.clone(),
                last_node_pass_index: passes.len() - 1,
                can_be_reused: false, // Should be safe to allow reuse? But last_node_pass_index effectively makes this never reuse
            };

            let physical_image_id = PhysicalImageId(physical_images.len());
            physical_images.push(physical_image);
            let old = image_virtual_to_physical.insert(virtual_id, physical_image_id);
            assert!(old.is_none());

            log::trace!(
                "  Input Image {:?} -> {:?} Used in passes [{}:{}]",
                virtual_id,
                physical_image_id,
                0,
                passes.len() - 1
            );
        }
    }

    for external_buffer in &graph.external_buffers {
        if let Some(input_usage) = external_buffer.input_usage {
            let virtual_id = virtual_resources.buffer_usage_to_virtual[&input_usage];
            let physical_buffer = PhysicalBuffer {
                specification: external_buffer.specification.clone(),
                last_node_pass_index: passes.len() - 1,
                can_be_reused: false, // Should be safe to allow reuse? But last_node_pass_index effectively makes this never reuse
            };

            let physical_buffer_id = PhysicalBufferId(physical_buffers.len());
            physical_buffers.push(physical_buffer);
            let old = buffer_virtual_to_physical.insert(virtual_id, physical_buffer_id);
            assert!(old.is_none());

            log::trace!(
                "  Input Buffer {:?} -> {:?} Used in passes [{}:{}]",
                virtual_id,
                physical_buffer_id,
                0,
                passes.len() - 1
            );
        }
    }

    //
    // Allocate physical IDs for all output images/buffers
    //
    for external_image in &graph.external_images {
        if let Some(output_usage) = external_image.output_usage {
            let virtual_id = virtual_resources.image_usage_to_virtual[&output_usage];
            let physical_image = PhysicalImage {
                specification: external_image.specification.clone(),
                last_node_pass_index: passes.len() - 1,
                can_be_reused: false, // Should be safe to allow reuse? But last_node_pass_index effectively makes this never reuse
            };

            let physical_image_id = if let Some(existing_physical_image_id) =
                image_virtual_to_physical.get(&virtual_id)
            {
                // If an input image already exists, verify it created the same physical image entry
                let existing_physical_image = &physical_images[existing_physical_image_id.0];
                assert_eq!(physical_image, *existing_physical_image);
                *existing_physical_image_id
            } else {
                let physical_image_id = PhysicalImageId(physical_images.len());
                physical_images.push(physical_image);
                let old = image_virtual_to_physical.insert(virtual_id, physical_image_id);
                assert!(old.is_none());
                physical_image_id
            };

            log::trace!(
                "  Output Image {:?} -> {:?} Used in passes [{}:{}]",
                virtual_id,
                physical_image_id,
                0,
                passes.len() - 1
            );
        }
    }

    for external_buffer in &graph.external_buffers {
        if let Some(output_usage) = external_buffer.output_usage {
            let virtual_id = virtual_resources.buffer_usage_to_virtual[&output_usage];
            let physical_buffer = PhysicalBuffer {
                specification: external_buffer.specification.clone(),
                last_node_pass_index: passes.len() - 1,
                can_be_reused: false, // Should be safe to allow reuse? But last_node_pass_index effectively makes this never reuse
            };

            let physical_buffer_id = if let Some(existing_physical_buffer_id) =
                buffer_virtual_to_physical.get(&virtual_id)
            {
                // If an input buffer already exists, verify it created the same physical buffer entry
                let existing_physical_buffer = &physical_buffers[existing_physical_buffer_id.0];
                assert_eq!(physical_buffer, *existing_physical_buffer);
                *existing_physical_buffer_id
            } else {
                let physical_buffer_id = PhysicalBufferId(physical_buffers.len());
                physical_buffers.push(physical_buffer);
                let old = buffer_virtual_to_physical.insert(virtual_id, physical_buffer_id);
                assert!(old.is_none());
                physical_buffer_id
            };

            log::trace!(
                "  Output Buffer {:?} -> {:?} Used in passes [{}:{}]",
                virtual_id,
                physical_buffer_id,
                0,
                passes.len() - 1
            );
        }
    }

    //
    // Determine the minimal set of physical images needed to represent all our virtual images,
    // given that virtual images can use the same physical image if their lifetimes don't overlap
    //
    // Images are sorted by first usage (because we register them in order of the passes that first
    // use them)
    //
    for reuse_requirements in &image_reuse_requirements {
        if image_virtual_to_physical.contains_key(&reuse_requirements.virtual_id) {
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

        image_virtual_to_physical.insert(reuse_requirements.virtual_id, physical_image_id);
    }

    for reuse_requirements in &buffer_reuse_requirements {
        if buffer_virtual_to_physical.contains_key(&reuse_requirements.virtual_id) {
            // May already have been registered by output buffer
            continue;
        }

        // See if we can reuse with an existing physical buffer
        let mut physical_buffer_id = None;
        for (physical_buffer_index, physical_buffer) in physical_buffers.iter_mut().enumerate() {
            if physical_buffer.last_node_pass_index < reuse_requirements.first_node_pass_index
                && physical_buffer.can_be_reused
            {
                if physical_buffer
                    .specification
                    .try_merge(&reuse_requirements.specification)
                {
                    physical_buffer.last_node_pass_index = reuse_requirements.last_node_pass_index;
                    physical_buffer_id = Some(PhysicalBufferId(physical_buffer_index));
                    log::trace!(
                        "  Intermediate Buffer (Reuse) {:?} -> {:?} Used in passes [{}:{}]",
                        reuse_requirements.virtual_id,
                        physical_buffer_id,
                        reuse_requirements.first_node_pass_index,
                        reuse_requirements.last_node_pass_index
                    );
                    break;
                }
            }
        }

        // If the existing physical buffers are not compatible, make a new one
        let physical_buffer_id = physical_buffer_id.unwrap_or_else(|| {
            let physical_buffer_id = PhysicalBufferId(physical_buffers.len());
            physical_buffers.push(PhysicalBuffer {
                specification: reuse_requirements.specification.clone(),
                last_node_pass_index: reuse_requirements.last_node_pass_index,
                can_be_reused: true,
            });

            log::trace!(
                "  Intermediate Buffer (Create new) {:?} -> {:?} Used in passes [{}:{}]",
                reuse_requirements.virtual_id,
                physical_buffer_id,
                reuse_requirements.first_node_pass_index,
                reuse_requirements.last_node_pass_index
            );
            physical_buffer_id
        });

        buffer_virtual_to_physical.insert(reuse_requirements.virtual_id, physical_buffer_id);
    }

    //
    // Create a lookup to get physical image from usage
    //
    let mut image_usage_to_physical = FnvHashMap::default();
    for (&usage, virtual_image) in &virtual_resources.image_usage_to_virtual {
        //TODO: This was breaking in a test because an output image had no usage flags and we
        // never assigned the output image a physical ID since it wasn't in a pass
        image_usage_to_physical.insert(usage, image_virtual_to_physical[virtual_image]);
    }

    //
    // Create a lookup to get physical buffer from usage
    //
    let mut buffer_usage_to_physical = FnvHashMap::default();
    for (&usage, virtual_buffer) in &virtual_resources.buffer_usage_to_virtual {
        //TODO: This was breaking in a test because an output buffer had no usage flags and we
        // never assigned the output buffer a physical ID since it wasn't in a pass
        buffer_usage_to_physical.insert(usage, buffer_virtual_to_physical[virtual_buffer]);
    }

    //
    // Setup image views
    //

    // Temporary to build image view list/lookup
    let mut image_subresource_to_view = FnvHashMap::default();

    // Create a list of all views needed for the graph and associating the usage with the view
    let mut image_views = Vec::default();
    let mut image_usage_to_image_view = FnvHashMap::default();

    //
    // Create a list and lookup for all image views that are needed for the graph
    //
    for (&usage, &physical_image) in &image_usage_to_physical {
        let image_specification = constraints.image_specification(usage).unwrap();
        let image_view = RenderGraphImageView {
            physical_image,
            format: image_specification.format,
            view_options: graph.image_usage(usage).view_options.clone(),
        };

        // Get the ID that matches the view, or insert a new view, generating a new ID

        let image_view_id = *image_subresource_to_view
            .entry(image_view.clone())
            .or_insert_with(|| {
                let image_view_id = PhysicalImageViewId(image_views.len());
                image_views.push(image_view);
                image_view_id
            });

        let old = image_usage_to_image_view.insert(usage, image_view_id);
        assert!(old.is_none());
    }

    for pass in passes {
        if let RenderGraphPass::Render(renderpass) = pass {
            for attachment in &mut renderpass.attachments {
                let physical_image = image_virtual_to_physical[&attachment.virtual_image];
                let image_view_id = image_usage_to_image_view[&attachment.usage];
                attachment.image = Some(physical_image);
                attachment.image_view = Some(image_view_id);
            }
        }
    }

    //
    // Create a list of all images that need to be created
    //
    let image_specifications: Vec<_> = physical_images
        .into_iter()
        .map(|x| x.specification)
        .collect();

    //
    // Create a list of all buffers that need to be created
    //
    let buffer_specifications: Vec<_> = physical_buffers
        .into_iter()
        .map(|x| x.specification)
        .collect();

    AssignPhysicalResourcesResult {
        image_usage_to_physical,
        image_virtual_to_physical,
        image_usage_to_image_view,
        image_views,
        image_specifications,
        buffer_usage_to_physical,
        buffer_virtual_to_physical,
        buffer_specifications,
    }
}

fn add_image_barrier_for_node(
    physical_resources: &AssignPhysicalResourcesResult,
    image_node_barriers: &mut FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers>,
    image: RenderGraphImageUsageId,
    resource_state: RafxResourceState,
) {
    let physical_image = physical_resources
        .image_usage_to_physical
        .get(&image)
        .unwrap();

    // If this assert fires, the image was used in multiple ways during the same pass
    let old = image_node_barriers.insert(
        *physical_image,
        RenderGraphPassImageBarriers::new(resource_state),
    );
    assert!(old.is_none());
}

fn add_buffer_barrier_for_node(
    physical_resources: &AssignPhysicalResourcesResult,
    buffer_node_barriers: &mut FnvHashMap<PhysicalBufferId, RenderGraphPassBufferBarriers>,
    buffer: RenderGraphBufferUsageId,
    resource_state: RafxResourceState,
) {
    let physical_buffer = physical_resources
        .buffer_usage_to_physical
        .get(&buffer)
        .unwrap();

    // If this assert fires, the image was used in multiple ways during the same pass
    let old = buffer_node_barriers.insert(
        *physical_buffer,
        RenderGraphPassBufferBarriers::new(resource_state),
    );
    assert!(old.is_none());
}

#[profiling::function]
fn build_node_barriers(
    graph: &RenderGraphBuilder,
    node_execution_order: &[RenderGraphNodeId],
    _constraints: &DetermineConstraintsResult,
    physical_resources: &AssignPhysicalResourcesResult,
    builtin_final_node: RenderGraphNodeId,
) -> FnvHashMap<RenderGraphNodeId, RenderGraphNodeResourceBarriers> {
    let mut resource_barriers =
        FnvHashMap::<RenderGraphNodeId, RenderGraphNodeResourceBarriers>::default();

    for node_id in node_execution_order {
        // A special final node is added to every graph to handle transitioning any output
        // images/buffers to their final state, so break out here and handle logic for this below
        if *node_id == builtin_final_node {
            break;
        }

        let node = graph.node(*node_id);
        let mut image_node_barriers: FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers> =
            Default::default();
        let mut buffer_node_barriers: FnvHashMap<PhysicalBufferId, RenderGraphPassBufferBarriers> =
            Default::default();

        //
        // RenderPass attachments
        //
        for color_attachment in &node.color_attachments {
            if let Some(color_attachment) = color_attachment {
                let read_or_write_usage = color_attachment
                    .read_image
                    .or(color_attachment.write_image)
                    .unwrap();

                add_image_barrier_for_node(
                    physical_resources,
                    &mut image_node_barriers,
                    read_or_write_usage,
                    RafxResourceState::RENDER_TARGET,
                );
            }
        }

        for resolve_attachment in &node.resolve_attachments {
            if let Some(resolve_attachment) = resolve_attachment {
                add_image_barrier_for_node(
                    physical_resources,
                    &mut image_node_barriers,
                    resolve_attachment.write_image,
                    RafxResourceState::RENDER_TARGET,
                );
            }
        }

        if let Some(depth_attachment) = &node.depth_attachment {
            let read_or_write_usage = depth_attachment
                .read_image
                .or(depth_attachment.write_image)
                .unwrap();

            add_image_barrier_for_node(
                physical_resources,
                &mut image_node_barriers,
                read_or_write_usage,
                RafxResourceState::DEPTH_WRITE,
            );
        }

        //
        // Shader image resources
        //
        for &image in &node.sampled_images {
            add_image_barrier_for_node(
                physical_resources,
                &mut image_node_barriers,
                image,
                RafxResourceState::SHADER_RESOURCE,
            );
        }

        for &image in &node.storage_image_creates {
            add_image_barrier_for_node(
                physical_resources,
                &mut image_node_barriers,
                image,
                RafxResourceState::UNORDERED_ACCESS,
            );
        }

        for &image in &node.storage_image_reads {
            add_image_barrier_for_node(
                physical_resources,
                &mut image_node_barriers,
                image,
                RafxResourceState::UNORDERED_ACCESS,
            );
        }

        for &image in &node.storage_image_modifies {
            add_image_barrier_for_node(
                physical_resources,
                &mut image_node_barriers,
                image,
                RafxResourceState::UNORDERED_ACCESS,
            );
        }

        for &image in &node.copy_src_image_reads {
            add_image_barrier_for_node(
                physical_resources,
                &mut image_node_barriers,
                image,
                RafxResourceState::COPY_SRC,
            );
        }

        for &image in &node.copy_dst_image_writes {
            add_image_barrier_for_node(
                physical_resources,
                &mut image_node_barriers,
                image,
                RafxResourceState::COPY_DST,
            );
        }

        //
        // Shader buffer resources
        //
        for &buffer in &node.vertex_buffer_reads {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::VERTEX_AND_CONSTANT_BUFFER,
            );
        }

        for &buffer in &node.index_buffer_reads {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::INDEX_BUFFER,
            );
        }

        for &buffer in &node.indirect_buffer_reads {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::INDIRECT_ARGUMENT,
            );
        }

        for &buffer in &node.uniform_buffer_reads {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::VERTEX_AND_CONSTANT_BUFFER,
            );
        }

        for &buffer in &node.storage_buffer_creates {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::UNORDERED_ACCESS,
            );
        }

        for &buffer in &node.storage_buffer_reads {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::UNORDERED_ACCESS,
            );
        }

        for &buffer in &node.storage_buffer_modifies {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::UNORDERED_ACCESS,
            );
        }

        for &buffer in &node.copy_src_buffer_reads {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::COPY_SRC,
            );
        }

        for &buffer in &node.copy_dst_buffer_writes {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut buffer_node_barriers,
                buffer,
                RafxResourceState::COPY_DST,
            );
        }

        resource_barriers.insert(
            *node_id,
            RenderGraphNodeResourceBarriers {
                image_barriers: image_node_barriers,
                buffer_barriers: buffer_node_barriers,
            },
        );
    }

    // A special final node is added to every graph to handle transitioning any output
    // images/buffers to their final state. We handle setting up the required barriers for that
    // node here.
    let _builtin_final_node = graph.node(builtin_final_node);
    let mut final_image_node_barriers: FnvHashMap<PhysicalImageId, RenderGraphPassImageBarriers> =
        Default::default();
    let mut final_buffer_node_barriers: FnvHashMap<
        PhysicalBufferId,
        RenderGraphPassBufferBarriers,
    > = Default::default();

    for external_image in &graph.external_images {
        if let Some(output_usage) = external_image.output_usage {
            add_image_barrier_for_node(
                physical_resources,
                &mut final_image_node_barriers,
                output_usage,
                external_image.final_state,
            );
        }
    }

    for external_buffer in &graph.external_buffers {
        if let Some(output_usage) = external_buffer.output_usage {
            add_buffer_barrier_for_node(
                physical_resources,
                &mut final_buffer_node_barriers,
                output_usage,
                external_buffer.final_state,
            );
        }
    }

    resource_barriers.insert(
        builtin_final_node,
        RenderGraphNodeResourceBarriers {
            image_barriers: final_image_node_barriers,
            buffer_barriers: final_buffer_node_barriers,
        },
    );

    resource_barriers
}

// * At this point we know images/image views, format, samples, load/store ops. We also know what
//   needs to be flushed/invalidated
// * We want to determine layouts and the validates/flushes we actually need to insert. Essentially
//   we simulate executing the graph in sequence and keep up with what's been invalidated/flushed,
//   and what layouts images are in when the respective node is run.
#[profiling::function]
fn build_pass_barriers(
    graph: &RenderGraphBuilder,
    _node_execution_order: &[RenderGraphNodeId],
    _constraints: &DetermineConstraintsResult,
    physical_resources: &AssignPhysicalResourcesResult,
    node_barriers: &FnvHashMap<RenderGraphNodeId, RenderGraphNodeResourceBarriers>,
    passes: &mut [RenderGraphPass],
) {
    log::trace!("-- build_pass_barriers --");

    //
    // We will walk through all nodes keeping track of memory access as we go
    //
    struct ImageState {
        resource_state: RafxResourceState,
    }

    impl Default for ImageState {
        fn default() -> Self {
            ImageState {
                resource_state: RafxResourceState::UNDEFINED,
            }
        }
    }

    struct BufferState {
        resource_state: RafxResourceState,
    }

    impl Default for BufferState {
        fn default() -> Self {
            BufferState {
                //DX12TODO: This was UNDEFINED but DX12 seems to need it to be COPY_DST?
                resource_state: RafxResourceState::COPY_DST,
            }
        }
    }

    //TODO: Starting from UNDEFINED initial state is generally bad, we are reusing resources, we
    // could know what state it was in

    //TODO: to support subpass, probably need image states for each previous subpass
    // TODO: This is coarse-grained over the whole image. Ideally it would be per-layer and per-mip
    let mut image_states: Vec<ImageState> =
        Vec::with_capacity(physical_resources.image_specifications.len());
    image_states.resize_with(physical_resources.image_specifications.len(), || {
        Default::default()
    });

    let mut buffer_states: Vec<BufferState> =
        Vec::with_capacity(physical_resources.buffer_specifications.len());
    buffer_states.resize_with(physical_resources.buffer_specifications.len(), || {
        Default::default()
    });

    // Populate init state for external images/buffers
    for external_image in &graph.external_images {
        if let Some(input_usage) = external_image.input_usage {
            let physical_id = physical_resources.image_usage_to_physical[&input_usage];
            image_states[physical_id.0].resource_state = external_image.initial_state;
        }
    }

    for external_buffer in &graph.external_buffers {
        if let Some(input_usage) = external_buffer.input_usage {
            let physical_id = physical_resources.buffer_usage_to_physical[&input_usage];
            buffer_states[physical_id.0].resource_state = external_buffer.initial_state;
        }
    }

    for (pass_index, pass) in passes.iter_mut().enumerate() {
        log::trace!("pass {}", pass_index);

        // Initial layout for all attachments at the start of the renderpass
        let mut attachment_initial_state: Vec<Option<RafxResourceState>> = Default::default();
        if let RenderGraphPass::Render(pass) = pass {
            attachment_initial_state.resize_with(pass.attachments.len(), || None);
        }

        //let nodes: Vec<_> = pass.nodes().iter().copied().collect();
        //for (subpass_index, subpass_node_id) in nodes.iter().enumerate() {
        let subpass_node_id = pass.node();
        let node_barriers = &node_barriers[&subpass_node_id];

        struct ImageTransition {
            physical_image_id: PhysicalImageId,
            old_state: RafxResourceState,
            new_state: RafxResourceState,
        }

        struct BufferTransition {
            physical_buffer_id: PhysicalBufferId,
            old_state: RafxResourceState,
            new_state: RafxResourceState,
        }

        let mut image_transitions = Vec::default();
        // Look at all the images we read and determine what invalidates we need
        for (physical_image_id, image_barrier) in &node_barriers.image_barriers {
            log::trace!("    image {:?}", physical_image_id);
            let image_state = &mut image_states[physical_image_id.0];

            let resource_state_change = image_state.resource_state != image_barrier.resource_state;
            if resource_state_change {
                log::trace!(
                    "      state change! {:?} -> {:?}",
                    image_state.resource_state,
                    image_barrier.resource_state
                );

                if resource_state_change {
                    image_transitions.push(ImageTransition {
                        physical_image_id: *physical_image_id,
                        old_state: image_state.resource_state,
                        new_state: image_barrier.resource_state,
                    });
                }

                image_state.resource_state = image_barrier.resource_state;
            }

            // Set the initial layout for the attachment, but only if it's the first time we've seen it
            //TODO: This is bad and does not properly handle an image being used in multiple ways requiring
            // multiple layouts
            if let RenderGraphPass::Render(pass) = pass {
                for (attachment_index, attachment) in &mut pass.attachments.iter_mut().enumerate() {
                    //log::trace!("      attachment {:?}", attachment.image);
                    if attachment.image.unwrap() == *physical_image_id {
                        if attachment_initial_state[attachment_index].is_none() {
                            //log::trace!("        initial layout {:?}", image_barrier.layout);
                            attachment_initial_state[attachment_index] =
                                Some(image_state.resource_state.into());

                            // Use an image barrier before the pass to transition the layout,
                            // so we will already be in the correct layout before starting the
                            // pass.
                            attachment.initial_state = image_barrier.resource_state.into();
                        }

                        attachment.final_state = image_barrier.resource_state.into();
                        break;
                    }
                }
            }
        }

        // Look at all the buffers we read and determine what invalidates we need
        let mut buffer_transitions = Vec::default();
        for (physical_buffer_id, buffer_barrier) in &node_barriers.buffer_barriers {
            log::trace!("    buffer {:?}", physical_buffer_id);
            let buffer_state = &mut buffer_states[physical_buffer_id.0];

            let resource_state_change =
                buffer_state.resource_state != buffer_barrier.resource_state;
            if resource_state_change {
                log::trace!(
                    "      state change! {:?} -> {:?}",
                    buffer_state.resource_state,
                    buffer_barrier.resource_state
                );

                buffer_transitions.push(BufferTransition {
                    physical_buffer_id: *physical_buffer_id,
                    old_state: buffer_state.resource_state,
                    new_state: buffer_barrier.resource_state,
                });

                buffer_state.resource_state = buffer_barrier.resource_state;
            }
        }

        let image_barriers: Vec<_> = image_transitions
            .into_iter()
            .map(|image_transition| {
                assert_ne!(image_transition.new_state, RafxResourceState::UNDEFINED);
                PrepassImageBarrier {
                    image: image_transition.physical_image_id,
                    old_state: image_transition.old_state,
                    new_state: image_transition.new_state,
                }
            })
            .collect();

        let buffer_barriers: Vec<_> = buffer_transitions
            .into_iter()
            .map(|buffer_transition| {
                assert_ne!(buffer_transition.new_state, RafxResourceState::UNDEFINED);
                PrepassBufferBarrier {
                    buffer: buffer_transition.physical_buffer_id,
                    old_state: buffer_transition.old_state,
                    new_state: buffer_transition.new_state,
                }
            })
            .collect();

        if !image_barriers.is_empty() || !buffer_barriers.is_empty() {
            let barrier = PrepassBarrier {
                image_barriers,
                buffer_barriers,
            };

            pass.set_pre_pass_barrier(barrier);
        }
    }
}

#[profiling::function]
fn create_output_passes(
    graph: &RenderGraphBuilder,
    passes: Vec<RenderGraphPass>,
) -> Vec<RenderGraphOutputPass> {
    let mut renderpasses = Vec::with_capacity(passes.len());

    for pass in passes {
        let render_node = graph.node(pass.node());
        let debug_name = render_node.name;

        match pass {
            RenderGraphPass::Render(pass) => {
                let attachment_images = pass
                    .attachments
                    .iter()
                    .map(|attachment| attachment.image_view.unwrap())
                    .collect();

                let mut color_formats = vec![];
                let mut sample_count = None;
                for color_attachment in &pass.color_attachments {
                    if let Some(color_attachment) = color_attachment {
                        color_formats.push(pass.attachments[*color_attachment].format);

                        let expected_sample_count = pass.attachments[*color_attachment].samples;
                        if let Some(sample_count) = sample_count {
                            assert_eq!(sample_count, expected_sample_count, "Render node has color attachments with different sample counts, this is unsupported.");
                        } else {
                            sample_count = Some(expected_sample_count);
                        }
                    }
                }

                let mut depth_format = None;
                if let Some(depth_attachment) = pass.depth_attachment {
                    depth_format = Some(pass.attachments[depth_attachment].format);

                    let expected_sample_count = pass.attachments[depth_attachment].samples;
                    if let Some(sample_count) = sample_count {
                        assert_eq!(sample_count, expected_sample_count, "Render node has color attachment and depth attachment with different sample counts, this is unsupported.");
                    } else {
                        sample_count = Some(expected_sample_count);
                    }
                }

                let render_target_meta = GraphicsPipelineRenderTargetMeta::new(
                    color_formats,
                    depth_format,
                    sample_count.unwrap(),
                );

                let mut color_render_targets = Vec::with_capacity(MAX_COLOR_ATTACHMENTS);

                for (color_index, attachment_index) in pass.color_attachments.iter().enumerate() {
                    if let Some(attachment_index) = attachment_index {
                        let attachment = &pass.attachments[*attachment_index]; //.image.unwrap();
                        let attachment_usage = &graph.image_usages[attachment.usage.0];
                        let array_slice = attachment_usage.view_options.array_slice;
                        let mip_slice = attachment_usage.view_options.mip_slice;

                        let mut resolve_image = None;
                        let mut resolve_array_slice = None;
                        let mut resolve_mip_slice = None;
                        let mut resolve_store_op = RafxStoreOp::DontCare;
                        if let Some(resolve_attachment_index) =
                            pass.resolve_attachments[color_index]
                        {
                            let resolve_attachment = &pass.attachments[resolve_attachment_index]; //.image.unwrap();
                            let resolve_attachment_usage =
                                &graph.image_usages[resolve_attachment.usage.0];
                            resolve_image = Some(resolve_attachment.image.unwrap());
                            resolve_array_slice = resolve_attachment_usage.view_options.array_slice;
                            resolve_mip_slice = resolve_attachment_usage.view_options.mip_slice;
                            resolve_store_op = resolve_attachment.store_op;
                        }

                        color_render_targets.push(RenderGraphColorRenderTarget {
                            image: attachment.image.unwrap(),
                            load_op: attachment.load_op,
                            store_op: attachment.store_op,
                            clear_value: attachment
                                .clear_color
                                .clone()
                                .map(|x| x.to_color_clear_value())
                                .unwrap_or_default(),
                            array_slice,
                            mip_slice,
                            resolve_image,
                            resolve_store_op,
                            resolve_array_slice,
                            resolve_mip_slice,
                        });
                    }
                }

                let mut depth_stencil_render_target = None;
                if let Some(attachment_index) = pass.depth_attachment {
                    let attachment = &pass.attachments[attachment_index];
                    let array_slice = graph.image_usages[attachment.usage.0]
                        .view_options
                        .array_slice;
                    let mip_slice = graph.image_usages[attachment.usage.0]
                        .view_options
                        .mip_slice;
                    depth_stencil_render_target = Some(RenderGraphDepthStencilRenderTarget {
                        image: attachment.image.unwrap(),
                        depth_load_op: attachment.load_op,
                        stencil_load_op: attachment.stencil_load_op,
                        depth_store_op: attachment.store_op,
                        stencil_store_op: attachment.stencil_store_op,
                        clear_value: attachment
                            .clear_color
                            .clone()
                            .map(|x| x.to_depth_stencil_clear_value())
                            .unwrap_or_default(),
                        array_slice,
                        mip_slice,
                    });
                }

                let output_pass = RenderGraphOutputRenderPass {
                    node_id: pass.node_id,
                    attachment_images,
                    pre_pass_barrier: pass.pre_pass_barrier,
                    debug_name,
                    color_render_targets,
                    depth_stencil_render_target,
                    render_target_meta,
                };

                renderpasses.push(RenderGraphOutputPass::Render(output_pass));
            }
            RenderGraphPass::Callback(pass) => {
                let output_pass = RenderGraphOutputCallbackPass {
                    node: pass.node,
                    pre_pass_barrier: pass.pre_pass_barrier,
                    debug_name,
                };

                renderpasses.push(RenderGraphOutputPass::Callback(output_pass));
            }
        }
    }

    renderpasses
}

#[allow(dead_code)]
fn print_constraints(
    graph: &RenderGraphBuilder,
    constraint_results: &mut DetermineConstraintsResult,
) {
    log::trace!("Image constraints:");
    for (image_index, image_resource) in graph.image_resources.iter().enumerate() {
        log::trace!("  Image {:?} {:?}", image_index, image_resource.name);
        for (version_index, version) in image_resource.versions.iter().enumerate() {
            log::trace!("    Version {}", version_index);

            log::trace!(
                "      Writen as: {:?}",
                constraint_results.image_specification(version.create_usage)
            );

            for (usage_index, usage) in version.read_usages.iter().enumerate() {
                log::trace!(
                    "      Read Usage {}: {:?}",
                    usage_index,
                    constraint_results.image_specification(*usage)
                );
            }
        }
    }

    log::trace!("Buffer constraints:");
    for (buffer_index, buffer_resource) in graph.buffer_resources.iter().enumerate() {
        log::trace!("  Buffer {:?} {:?}", buffer_index, buffer_resource.name);
        for (version_index, version) in buffer_resource.versions.iter().enumerate() {
            log::trace!("    Version {}", version_index);

            log::trace!(
                "      Writen as: {:?}",
                constraint_results.buffer_specification(version.create_usage)
            );

            for (usage_index, usage) in version.read_usages.iter().enumerate() {
                log::trace!(
                    "      Read Usage {}: {:?}",
                    usage_index,
                    constraint_results.buffer_specification(*usage)
                );
            }
        }
    }
}

#[allow(dead_code)]
fn print_image_compatibility(
    graph: &RenderGraphBuilder,
    constraint_results: &DetermineConstraintsResult,
) {
    log::trace!("Image Compatibility Report:");
    for (image_index, image_resource) in graph.image_resources.iter().enumerate() {
        log::trace!("  Image {:?} {:?}", image_index, image_resource.name);
        for (version_index, version) in image_resource.versions.iter().enumerate() {
            let write_specification = constraint_results.image_specification(version.create_usage);

            log::trace!("    Version {}: {:?}", version_index, version);
            for (usage_index, usage) in version.read_usages.iter().enumerate() {
                let read_specification = constraint_results.image_specification(*usage);

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
    node_barriers: &FnvHashMap<RenderGraphNodeId, RenderGraphNodeResourceBarriers>
) {
    log::trace!("Barriers:");
    for (node_id, barriers) in node_barriers.iter() {
        log::trace!("  pass {:?}", node_id);
        log::trace!("    resource states");
        for (physical_id, barriers) in &barriers.image_barriers {
            log::trace!("      {:?}: {:?}", physical_id, barriers.resource_state);
        }

        for (physical_id, barriers) in &barriers.buffer_barriers {
            log::trace!("      {:?}: {:?}", physical_id, barriers.resource_state);
        }
    }
}

#[allow(dead_code)]
fn verify_unculled_image_usages_specifications_exist(
    graph: &RenderGraphBuilder,
    node_execution_order: &Vec<RenderGraphNodeId>,
    constraint_results: &DetermineConstraintsResult,
) {
    for (_image_index, image_resource) in graph.image_resources.iter().enumerate() {
        //log::trace!("  Image {:?} {:?}", image_index, image_resource.name);
        for (_version_index, version) in image_resource.versions.iter().enumerate() {
            // Check the write usage for this version
            if node_execution_order.contains(&version.creator_node)
                && constraint_results
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
                    RenderGraphImageUser::Input(_) => true,
                    RenderGraphImageUser::Output(_) => true,
                };

                if is_scheduled && constraint_results.images.get(usage).is_none() {
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
    assign_physical_resources_result: &AssignPhysicalResourcesResult,
    external_images: &FnvHashMap<PhysicalImageViewId, RenderGraphPlanExternalImage>,
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

    for (physical_id, external_image) in external_images {
        log::trace!(
            "External Image: {:?} {:?}",
            assign_physical_resources_result.image_views[physical_id.0].physical_image,
            external_image
        );
    }
}

#[allow(dead_code)]
fn print_final_buffers(
    external_buffers: &FnvHashMap<PhysicalBufferId, RenderGraphPlanExternalBuffer>,
    intermediate_buffers: &FnvHashMap<PhysicalBufferId, RenderGraphBufferSpecification>,
) {
    log::trace!("-- BUFFERS --");
    for (physical_id, intermediate_image_spec) in intermediate_buffers {
        log::trace!(
            "Intermediate Buffer: {:?} {:?}",
            physical_id,
            intermediate_image_spec
        );
    }
    for (physical_id, external_image) in external_buffers {
        log::trace!("External Buffer: {:?} {:?}", physical_id, external_image);
    }
}

#[allow(dead_code)]
fn print_final_resource_usages(
    graph: &RenderGraphBuilder,
    assign_physical_resources_result: &AssignPhysicalResourcesResult,
    constraint_results: &DetermineConstraintsResult,
    renderpasses: &Vec<RenderGraphOutputPass>,
) {
    fn print_image_resource_usage(
        graph: &RenderGraphBuilder,
        assign_physical_resources_result: &AssignPhysicalResourcesResult,
        constraint_results: &DetermineConstraintsResult,
        image: RenderGraphImageUsageId,
        prefix: &str,
    ) {
        let physical_image = assign_physical_resources_result.image_usage_to_physical[&image];
        let write_name = graph.image_resource(image).name;
        log::debug!(
            "    {}: {:?} Name: {:?} Constraints: {:?}",
            prefix,
            physical_image,
            write_name,
            constraint_results.images[&image]
        );
    }

    fn print_buffer_resource_usage(
        graph: &RenderGraphBuilder,
        assign_physical_resources_result: &AssignPhysicalResourcesResult,
        constraint_results: &DetermineConstraintsResult,
        buffer: RenderGraphBufferUsageId,
        prefix: &str,
    ) {
        let physical_buffer = assign_physical_resources_result.buffer_usage_to_physical[&buffer];
        let write_name = graph.buffer_resource(buffer).name;
        log::debug!(
            "    {}: {:?} Name: {:?} Constraints: {:?}",
            prefix,
            physical_buffer,
            write_name,
            constraint_results.buffers[&buffer]
        );
    }

    log::debug!("-- RESOURCE USAGE --");
    for (pass_index, pass) in renderpasses.iter().enumerate() {
        log::debug!("pass {}", pass_index);

        let node = graph.node(pass.node());
        log::debug!("  subpass {:?} {:?}", pass.node(), node.name);

        for (color_attachment_index, color_attachment) in node.color_attachments.iter().enumerate()
        {
            if let Some(color_attachment) = color_attachment {
                let read_or_write = color_attachment
                    .read_image
                    .or_else(|| color_attachment.write_image)
                    .unwrap();
                print_image_resource_usage(
                    graph,
                    assign_physical_resources_result,
                    constraint_results,
                    read_or_write,
                    &format!("Color Attachment {}", color_attachment_index),
                );
            }
        }

        for (resolve_attachment_index, resolve_attachment) in
            node.resolve_attachments.iter().enumerate()
        {
            if let Some(resolve_attachment) = resolve_attachment {
                print_image_resource_usage(
                    graph,
                    assign_physical_resources_result,
                    constraint_results,
                    resolve_attachment.write_image,
                    &format!("Resolve Attachment {}", resolve_attachment_index),
                );
            }
        }

        if let Some(depth_attachment) = &node.depth_attachment {
            let read_or_write = depth_attachment
                .read_image
                .or_else(|| depth_attachment.write_image)
                .unwrap();
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                read_or_write,
                "Depth Attachment",
            );
        }

        for &image in &node.sampled_images {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                image,
                "Sampled",
            );
        }

        for &image in &node.storage_image_creates {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                image,
                "Storage Image (Create)",
            );
        }

        for &image in &node.storage_image_reads {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                image,
                "Storage Image (Read)",
            );
        }

        for &image in &node.storage_image_modifies {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                image,
                "Storage Image (Modify)",
            );
        }

        for &image in &node.copy_src_image_reads {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                image,
                "Copy Src Image",
            );
        }

        for &image in &node.copy_dst_image_writes {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                image,
                "Copy Dst Image",
            );
        }

        for &buffer in &node.vertex_buffer_reads {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Vertex Buffer",
            );
        }

        for &buffer in &node.index_buffer_reads {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Index Buffer",
            );
        }

        for &buffer in &node.indirect_buffer_reads {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Indirect Buffer",
            );
        }

        for &buffer in &node.uniform_buffer_reads {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Uniform Buffer",
            );
        }

        for &buffer in &node.storage_buffer_creates {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Storage Buffer (Create)",
            );
        }

        for &buffer in &node.storage_buffer_reads {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Storage Buffer (Read)",
            );
        }

        for &buffer in &node.storage_buffer_modifies {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Storage Buffer (Modify)",
            );
        }

        for &buffer in &node.copy_src_buffer_reads {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Copy Src Buffer",
            );
        }

        for &buffer in &node.copy_dst_buffer_writes {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                buffer,
                "Copy Dst Buffer",
            );
        }
    }

    log::debug!("External Resources");

    for external_image in &graph.external_images {
        if let Some(input_usage) = external_image.input_usage {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                input_usage,
                "Read External Image",
            );
        }

        if let Some(output_usage) = external_image.output_usage {
            print_image_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                output_usage,
                "Write External Image",
            );
        }
    }

    for external_buffer in &graph.external_buffers {
        if let Some(input_usage) = external_buffer.input_usage {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                input_usage,
                "Read External Buffer",
            );
        }

        if let Some(output_usage) = external_buffer.output_usage {
            print_buffer_resource_usage(
                graph,
                assign_physical_resources_result,
                constraint_results,
                output_usage,
                "Write External Buffer",
            );
        }
    }
}

#[derive(Debug)]
pub struct RenderGraphPlanExternalImage {
    pub id: RenderGraphExternalImageId,
    pub resource: ResourceArc<ImageViewResource>,
}

#[derive(Debug)]
pub struct RenderGraphPlanExternalBuffer {
    pub id: RenderGraphExternalBufferId,
    pub resource: ResourceArc<BufferResource>,
}

/// The final output of a render graph, which will be consumed by PreparedRenderGraph. This just
/// includes the computed metadata and does not allocate resources.
pub struct RenderGraphPlan {
    pub(super) passes: Vec<RenderGraphOutputPass>,
    pub(super) external_images: FnvHashMap<PhysicalImageViewId, RenderGraphPlanExternalImage>,
    pub(super) external_buffers: FnvHashMap<PhysicalBufferId, RenderGraphPlanExternalBuffer>,
    pub(super) intermediate_images: FnvHashMap<PhysicalImageId, RenderGraphImageSpecification>,
    pub(super) intermediate_buffers: FnvHashMap<PhysicalBufferId, RenderGraphBufferSpecification>,
    pub(super) image_views: Vec<RenderGraphImageView>, // index by physical image view id
    pub(super) node_to_pass_index: FnvHashMap<RenderGraphNodeId, usize>,
    pub(super) _image_usage_to_physical: FnvHashMap<RenderGraphImageUsageId, PhysicalImageId>,
    pub(super) image_usage_to_view: FnvHashMap<RenderGraphImageUsageId, PhysicalImageViewId>,
    pub(super) buffer_usage_to_physical: FnvHashMap<RenderGraphBufferUsageId, PhysicalBufferId>,

    // callbacks
    pub(super) visit_node_callbacks:
        FnvHashMap<RenderGraphNodeId, RenderGraphNodeVisitNodeCallback>,
    pub(super) _render_phase_dependencies:
        FnvHashMap<RenderGraphNodeId, FnvHashSet<RenderPhaseIndex>>,
}

impl RenderGraphPlan {
    #[profiling::function]
    pub(super) fn new(
        mut graph: RenderGraphBuilder,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> RenderGraphPlan {
        log::trace!("-- Create render graph plan --");

        //
        // We add an extra node that always runs at the end. This lets us use the same codepath for
        // inserting barriers before nodes in the graph to insert barriers at the end of the graph
        // for "output" images/buffers
        //
        let builtin_final_node =
            graph.add_callback_node("BuiltinFinalNode", RenderGraphQueue::DefaultGraphics);

        //
        // Walk backwards through the DAG, starting from the output images, through all the upstream
        // dependencies of those images. We are doing a depth first search. Nodes that make no
        // direct or indirect contribution to an output image will not be included. As an
        // an implementation detail, we try to put renderpass merge candidates adjacent to each
        // other in this list
        //

        //TODO: Support to force a node to be executed/unculled
        let mut node_execution_order = determine_node_order(&graph);
        node_execution_order.push(builtin_final_node);

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
        let mut constraint_results =
            determine_constraints(&graph, &node_execution_order, swapchain_surface_info);

        // Look at all image versions and ensure a constraint exists for usages where the node was
        // not culled
        //RenderGraphPlan::verify_unculled_image_usages_specifications_exist(&graph, &node_execution_order, &constraint_results);

        // Print out the constraints assigned to images
        //print_image_constraints(&graph, &mut constraint_results);

        //
        // Add resolves to the graph - this will occur when a renderpass outputs a multisample image
        // to a renderpass that is expecting a non-multisampled image.
        //
        insert_resolves(&mut graph, &node_execution_order, &mut constraint_results);

        // Print the cases where we can't reuse images
        //print_image_compatibility(&graph, &constraint_results);

        //
        // Assign logical images to physical images. This should give us a minimal number of images
        // if we are not reusing or aliasing. (We reuse when we assign physical indexes)
        //
        let assign_virtual_images_result =
            assign_virtual_resources(&graph, &node_execution_order, &mut constraint_results);

        //
        // Combine nodes into passes where possible
        //
        let mut passes = build_physical_passes(
            &graph,
            &node_execution_order,
            &constraint_results,
            &assign_virtual_images_result,
        );

        //
        // Find virtual images with matching specification and non-overlapping lifetimes. Assign
        // the same physical index to them so that we reuse a single allocation
        //
        let assign_physical_resources_result = assign_physical_resources(
            &graph,
            &constraint_results,
            &assign_virtual_images_result,
            &mut passes,
        );

        //
        // Determine read/write barriers for each node based on the data the produce/consume
        //
        let node_barriers = build_node_barriers(
            &graph,
            &node_execution_order,
            &constraint_results,
            &assign_physical_resources_result, /*, &determine_image_layouts_result*/
            builtin_final_node,
        );

        print_node_barriers(&node_barriers);

        //TODO: Figure out in/out layouts for passes? Maybe insert some other fixes? Drop transient
        // images?

        //
        // Combine the node barriers to produce the dependencies for subpasses and determine/handle
        // image layout transitions
        //
        build_pass_barriers(
            &graph,
            &node_execution_order,
            &constraint_results,
            &assign_physical_resources_result,
            &node_barriers,
            &mut passes,
        );

        //TODO: Cull images that only exist within the lifetime of a single pass? (just passed among
        // subpasses)

        //TODO: Allocation of images
        // alias_images(
        //     &graph,
        //     &node_execution_order,
        //     &constraint_results,
        //     &assign_physical_resources_result,
        //     &node_barriers,
        //     &passes,
        // );

        //
        // Produce the final output data. This mainly includes a descriptor object that can be
        // passed into the resource system to create the renderpass but also includes other metadata
        // required to push them through the command queue
        //
        let output_passes = create_output_passes(&graph, passes);

        //
        // Separate the output images from the intermediate images (the rendergraph will be
        // responsible for allocating the intermediate images)
        //
        let mut external_images: FnvHashMap<PhysicalImageViewId, RenderGraphPlanExternalImage> =
            Default::default();
        let mut external_image_physical_ids = FnvHashSet::default();
        for external_image in &graph.external_images {
            let input_physical_view_id = external_image
                .input_usage
                .map(|usage| assign_physical_resources_result.image_usage_to_image_view[&usage]);

            let output_physical_view_id = external_image
                .output_usage
                .map(|usage| assign_physical_resources_result.image_usage_to_image_view[&usage]);

            if let Some(physical_view_id) = input_physical_view_id.or(output_physical_view_id) {
                // Verify that one of them was None, or that they were the same
                assert!(
                    input_physical_view_id.is_none()
                        || output_physical_view_id.is_none()
                        || input_physical_view_id == output_physical_view_id
                );

                external_images.insert(
                    physical_view_id,
                    RenderGraphPlanExternalImage {
                        id: external_image.external_image_id,
                        resource: external_image.image_resource.clone(),
                    },
                );

                external_image_physical_ids.insert(
                    assign_physical_resources_result.image_views[physical_view_id.0].physical_image,
                );
            }
        }

        let mut external_buffers: FnvHashMap<PhysicalBufferId, RenderGraphPlanExternalBuffer> =
            Default::default();
        //let mut external_buffer_physical_ids = FnvHashSet::default();
        for external_buffer in &graph.external_buffers {
            let input_physical_id = external_buffer
                .input_usage
                .map(|usage| assign_physical_resources_result.buffer_usage_to_physical[&usage]);

            let output_physical_id = external_buffer
                .output_usage
                .map(|usage| assign_physical_resources_result.buffer_usage_to_physical[&usage]);

            if let Some(physical_id) = input_physical_id.or(output_physical_id) {
                // Verify that one of them was None, or that they were the same
                assert!(
                    input_physical_id.is_none()
                        || output_physical_id.is_none()
                        || input_physical_id == output_physical_id
                );

                external_buffers.insert(
                    physical_id,
                    RenderGraphPlanExternalBuffer {
                        id: external_buffer.external_buffer_id,
                        resource: external_buffer.buffer_resource.clone(),
                    },
                );
            }
        }

        let mut intermediate_images: FnvHashMap<PhysicalImageId, RenderGraphImageSpecification> =
            Default::default();
        for (index, specification) in assign_physical_resources_result
            .image_specifications
            .iter()
            .enumerate()
        {
            let physical_image = PhysicalImageId(index);

            if external_image_physical_ids.contains(&physical_image) {
                continue;
            }

            intermediate_images.insert(physical_image, specification.clone());
        }

        let mut intermediate_buffers: FnvHashMap<PhysicalBufferId, RenderGraphBufferSpecification> =
            Default::default();
        for (index, specification) in assign_physical_resources_result
            .buffer_specifications
            .iter()
            .enumerate()
        {
            let physical_buffer = PhysicalBufferId(index);

            if external_buffers.contains_key(&physical_buffer) {
                continue;
            }

            intermediate_buffers.insert(physical_buffer, specification.clone());
        }

        print_final_images(
            &assign_physical_resources_result,
            &external_images,
            &intermediate_images,
        );
        print_final_buffers(&external_buffers, &intermediate_buffers);

        print_final_resource_usages(
            &graph,
            &assign_physical_resources_result,
            &constraint_results,
            &output_passes,
        );

        //
        // Create a lookup from node_id to pass. Nodes are culled and renderpasses may include
        // subpasses from multiple nodes.
        //
        let mut node_to_pass_index = FnvHashMap::default();
        for (pass_index, pass) in output_passes.iter().enumerate() {
            node_to_pass_index.insert(pass.node(), pass_index);
        }

        RenderGraphPlan {
            passes: output_passes,
            external_images,
            external_buffers,
            intermediate_images,
            intermediate_buffers,
            image_views: assign_physical_resources_result.image_views,
            node_to_pass_index,
            _image_usage_to_physical: assign_physical_resources_result.image_usage_to_physical,
            image_usage_to_view: assign_physical_resources_result.image_usage_to_image_view,
            buffer_usage_to_physical: assign_physical_resources_result.buffer_usage_to_physical,

            visit_node_callbacks: graph.visit_node_callbacks,
            _render_phase_dependencies: graph.render_phase_dependencies,
        }
    }
}
