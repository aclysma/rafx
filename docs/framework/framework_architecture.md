# Framework Architecture

`rafx-framework` provides an architecture similar to what is describe in the 2015 GDC talk 
"[Destiny's Multithreaded Rendering Architecture](http://advances.realtimerendering.com/destiny/gdc_2015/Tatarchuk_GDC_2015__Destiny_Renderer_web.pdf)".

A video of this talk is available here! https://www.youtube.com/watch?v=0nTDFLMLX9k

## Pipelining

One of the most important goals in this architecture is to move as much rendering work as possible off the critical 
path of generating a frame. Rendering work should happen on a separate thread from simulation.

![Diagram showing pipelineing](../images/pipelining.png)

This diagram shows a "pipelined" frame. As soon as simulation for frame N ends, we can hand it off to a rendering thread
and immediately start simulation for frame N+1. A similar pattern could be followed for other systems.

## Features

`rafx-framework` provides a mechanism for you to register and implement new rendering features. For example, you might
register separate features for drawing meshes, sprites, cloth, water, etc. The framework helps you combine the output
of your features to render the scene.

Features must be registered.

## Phases

Features need to submit draw calls at the correct time when rendering a scene. This is usually associated with a
particular render pass.

For example, a mesh may need to be drawn BOTH for creating shadows and for the main view. A feature may emit a "submit node" 
for a particular mesh in both phases.

Phases define a sorting order. This allows front-to-back, back-to-front, or batched-by-feature ordering of draw
calls within the phase.

Phases must be registered.

## Views

When rendering a scene, it's often necessary to draw the scene from different viewpoints.

For example, when creating shadow maps, we must render a depth buffer from the viewpoint of all light sources that cast
shadows. In this case, we would create a view for each shadow-casting light that will run just the shadow mapping phase.

## Render Resources

The framework provides a "resource" table (similar to ECS resources, but simplified) that allows storing shared data
between the draw scheduling logic and extract/prepare jobs. In general, it is better to pass/share data "normally"
with plain rust code. However, there are cases where the flexibility is useful either for code modularity or when rust's
borrow checking cannot verify that your code is safe.

# Rendering a Frame

Each frame will go through these steps:

## Simulation

Process all game logic as normal. Game logic may be stored in whatever way you like. (ECS of your choice, or no ECS at 
all).

## Extract Jobs

Copy all data from game state necessary to render the scene. Extract jobs implement the `RenderFeatureExtractJob` trait.

**Game state may not change during this time. This will likely block simulating the next frame until it completes.**
However, extract jobs may run concurrently. (A similar pattern could be followed for other systems like audio.)

Generally only **dynamic** data needs to be copied. Static data is safely shareable between threads 
(via `Arc<T>` or other mechanisms).

After the data has been extracted, the prepare and write jobs can be run on separate threads from the simulation.

## Prepare Jobs

Process all the data collected during the extract job and produce `SubmitNode`s. Prepare jobs implement the
`RenderFeaturePrepareJob` trait.

`SubmitNode`s are like a handle for something that can be rendered later. The `SubmitNode`s are associated with a particular
`RenderView` and `RenderPhase`.

This job might make holistic decision for the feature, like choosing the 10 most important pieces of cloth to render at
high LOD. Prepare jobs for different features may run concurrently.

## Submit Node Sorting

The `SubmitNode`s emitted by all prepare jobs are sorted by the `RenderPhase` for each `RenderView`. When sort order does 
not matter, they can be sorted by render feature to reduce pipeline changes. When sort order does matter (like when working 
with transparency), features can be sorted as needed, most likely by depth.

## Write Jobs

Record the draw calls needed for each `SubmitNode` into command buffers. Write jobs implement the `RenderFeatureWriteJob` trait.

# Draw Scheduling

When the frame is renderered, we must set up render passes and run the phases intended for that pass. In the destiny GDC
talk, this was called a "script". It sounds like the manually handled this process. It's certainly not a bad way to go
as it's simple and flexible. In particular, the flexibility is important to ensure that low-CPU/high-GPU units of work
are submitted first so that both the CPU and GPU are fully utilized at all times.

However, many other games have started to use render graphs to solve this. `rafx-framework` provides a render graph
implementation. Phases can be kicked off from within render graph nodes.

```rust
// Example of registering a callback on a render graph node that triggers a render phase
graph_callbacks.set_renderpass_callback(node, move |args, user_context| {
    let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
    user_context
        .prepared_render_data
        .write_view_phase::<OpaqueRenderPhase>(&main_view, &mut write_context)
});
```