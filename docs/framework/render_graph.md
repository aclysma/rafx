# Render Graph

`rafx-framework` provides a render graph implementation that handles several concerns:
 * Creating and using render passes
 * Allocating dynamic resources used within the frame
 * Transitioning resources between states as needed during the frame
 * Inserting GPU synchronization to handle potential memory hazards

Render graphs must be created every frame. This ensures that when a scene or intended render settings change, they take
effect immediately. This allows parts of the graph to only execute when there are things in the scene that require it.
Nodes that produce outputs that are not part of the dependency chain for final output for the frame are discarded.

See these external resources for more info:
 * [FrameGraph: Extensible Rendering Architecture in Frostbite](https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-Rendering-Architecture-in) - Conceptual explanation of render graphs
 * [Render Graphs and Vulkan - a deep dive](http://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/) - Render graph implementation in raw vulkan

## Example Usage

This feature is probably best explained by example. See the 
[Render Graph Triangle example](../../rafx/examples/render_graph_triangle/render_graph_triangle.rs) or demo code.

## Graph Nodes

A node usually represents a render pass or a compute pass. The name is for debug purposes only.

Nodes have inputs and outputs. The graph must be a directed acyclic graph. The nodes will be executed in an order chosen
by considering input/output dependencies. Producing a cyclic graph will 

```rust
// We create an empty graph and set of callbacks that we will populate
let mut graph_builder = RenderGraphBuilder::default();
let mut graph_callbacks = RenderGraphNodeCallbacks::<()>::default();

// The string name is for logging/debugging purposes only
let node = graph_builder.add_node("opaque", RenderGraphQueue::DefaultGraphics);
```

## Adding Render Pass Attachments

Input/Output dependencies for nodes can be added by calling additional functions on the graph. These functions for the
most begin with `create_`, `read_`, and `modify_`. This corresponds to write, read, and read/write behavior.

```rust
// Create a simple render pass, cleared to black with 4xMSAA
let color_attachment = graph_builder.create_color_attachment(
    node,
    0, // color attachment index
    Some(RafxColorClearValue([0.0, 0.0, 0.0, 0.0])),
    RenderGraphImageConstraint {
        samples: Some(RafxSampleCount::SampleCount4),
        format: Some(swapchain_helper.format()),
        ..Default::default()
    },
    Default::default(),
);

// Adding a name improves logging/debugging
graph_builder.set_image_name(color_attachment, "color");
```

## Adding a callback

```rust
graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
    let command_buffer = args.command_buffer;
    // ... render code here
}
```

## Marking the Attachment as an Output Image

Generally when rendering a frame, you ultimately want to write the rendered scene into the swapchain image. To
accomplish this with the render graph, we assign the color attachment created in the pass to output to the swapchain
image.

In this case, after the render graph is analyzed, the resulting plan will:
 * Create a temporary 4xMSAA image 
 * Render to this image, but resolve to the swapchain image rather than storing it

```rust
graph_builder.set_output_image(
    color_attachment,
    swapchain_image,
    RenderGraphImageSpecification {
        samples: RafxSampleCount::SampleCount1,
        format: swapchain_helper.format(),
        resource_type: RafxResourceType::TEXTURE,
        extents: RenderGraphImageExtents::MatchSurface,
        layer_count: 1,
        mip_count: 1,
    },
    Default::default(),
    RafxResourceState::PRESENT,
);
```

## Executing the Graph

Use the `RenderGraphExecutor` to allocate resources and issue callbacks. This will produce command buffers that may be
submitted.

```rust
let executor = RenderGraphExecutor::new(
    &device_context,
    &resource_context,
    graph_builder,
    &swapchain_surface_info,
    graph_callbacks,
)?;

let command_buffers = executor.execute_graph(&(), &graphics_queue)?;
```
