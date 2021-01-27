# Safety

Rafx is an **unsafe** API. Interacting with a GPU is a fundamentally unsafe thing to do. It is really quite easy
to accidentally kernel panic or freeze a system if a shader does something bad.

Safe APIs in rust assure safety by adding runtime checking. They must track when/how resources are used to assure
that the resource is not deleted while the GPU is using it, and that resource states are transitioned with appropriate
memory barriers. These barriers may not always be optimal because the API cannot know what you will do with that
resource in the future.

Rafx API does not do resource tracking for you. You must handle **lifetimes** and **resource state transitions**.
However, Rafx Framework provides tools to address both issues. Using these tools, near-native performance can be
obtained without the full complexity of using a native API directly.

## Resource Lifetimes

When resources (like buffers, textures) are used by a GPU, most modern APIs assume that the resource will not be
deleted until BOTH the CPU and GPU will no longer reference it. The GPU may try to reference resources due to command
buffers or descriptor sets referencing those resources.

Most applications that use the GPU follow a standard pattern of rotating through 3 images:
 * The frame that is on the screen
 * A frame that's finished rendering and will be placed on the screen at the next vsync
 * An incomplete frame that is being drawn

(If an application is running at a lower frame rate, there may not be a completed image ready to swap on the next vsync.)

The API layer of rafx does not automatically hold resources for these extra frames. However, the framework layer
provides reference counting mechanisms that delay destroying resources until enough frames pass that the resource
is known not to be in use.

## Resource Transitions and Barriers

Working with a GPU requires special memory usage considerations:
 * Unlike the CPU, most GPUs have many layers of caching that are not coherent.
 * GPUs "pipeline" work - without synchronization it's possible that two renderpasses will try to read and write to a
   resource at the same time.
 * GPUs may store an image in a form and/or compressed in such a way that it is only compatible with certain operations.
   (For example, "sampled" by a shader or drawn by a renderpass).

For the most part, **immutable** resources (vertex buffers and textures loaded for disk) avoid these problems. For
example, a compute shader may **write** vertex data into a buffer for a renderpass to **read** later.

However, resources that change will often need to be transitioned between states between read/write operations. Use
`RafxCommandBuffer::cmd_resource_barrier` to transition a resource from one state to another.

In addition to potentially changing the form the resource is stored in on the GPU, resource transitions will insert
memory and pipeline barriers to solve the previously mentioned memory hazards.

### Render Graph

Handling these transitions can be difficult, especially when certain features may be enabled/disabled at runtime 
(anti-aliasing, bloom quality, etc.). Rafx Framework provides a render graph implementation to help manage this. The
render graph allows you to define what resources you will use and how/when you will use them.

In addition to managing barriers for you, the render graph will create/reuse runtime resources like buffers and textures
used as render targets. Using a render graph will also allow more sophisticated usage of memory like aliasing images
when the render graph knows the images will never be used concurrently.

### Examples

### Manual Resource Transition

The API Triangle example shows a resource barrier from `PRESENT` -> `RENDER_TARGET` and then `RENDER_TARGET` -> 
`PRESENT`.

```rust
// Acquire a swapchain image

cmd_buffer.cmd_resource_barrier(
    &[],
    &[],
    &[RafxRenderTargetBarrier::state_transition(
        &render_target,
        RafxResourceState::PRESENT,
        RafxResourceState::RENDER_TARGET,
    )],
)?;

// Draw on the render_target

cmd_buffer.cmd_resource_barrier(
    &[],
    &[],
    &[RafxRenderTargetBarrier::state_transition(
        &render_target,
        RafxResourceState::RENDER_TARGET,
        RafxResourceState::PRESENT,
    )],
)?;

// Present the swapchain image
```

### Using the Render Graph to Automate Resource Handling

The Render Graph Triangle example shows a simple render graph with one step.

```rust
// Create a renderpass with a single color attachment
let node = graph_builder.add_node("opaque", RenderGraphQueue::DefaultGraphics);
let color_attachment = graph_builder.create_color_attachment(
   node,
   0,
   Some(RafxColorClearValue([0.0, 0.0, 0.0, 0.0])),
   RenderGraphImageConstraint {
      samples: Some(RafxSampleCount::SampleCount4),
      format: Some(swapchain_helper.format()),
      ..Default::default()
   },
   Default::default(),
);

// ... potentially more passes, see the full example and demo for more details

graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
    // The render graph creates a 4xMSAA image for you and sets up the renderpass. You can just draw!
    args.command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;
    args.command_buffer.cmd_bind_vertex_buffers(
       0,
       &[RafxVertexBufferBinding {
          buffer: &vertex_buffer.get_raw().buffer,
          offset: 0,
       }],
    )?;
    args.command_buffer.cmd_draw(3, 0)?;
}

// This tells the graph that the final result should end up on the swapchain image
graph_builder.set_output_image(
   color_attachment,
   swapchain_image_view,
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