# Adding Render Phases

Render phases represent when to issue draw calls within a frame, usually in a render pass. For example, there
could be render phases for shadow maps, standard 3d drawing, or drawing 2d UI on top of the scene.

Phases combine the draw calls from multiple features into a single pass by sorting them according to a function.

## Declare the Render Phase

You may either implement `RenderPhase` or use this macro to reduce boilerplate code.

The sort function will be applied to all submit nodes in the same phase. Most common sorting schemes are:
 * Front-to-back
 * Back-to-front
 * Batched by feature

```rust
rafx::declare_render_phase!(
    ShadowMapRenderPhase,
    SHADOW_MAP_RENDER_PHASE_INDEX,
    shadow_map_render_phase_sort_submit_nodes
);

fn shadow_map_render_phase_sort_submit_nodes(submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sort by feature
    log::trace!(
        "Sort phase {}",
        ShadowMapRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));
}
```

## Register the Render Phase

```rust
let render_registry = rafx::nodes::RenderRegistryBuilder::default()
    .register_render_phase::<OpaqueRenderPhase>("Opaque")
    .register_render_phase::<ShadowMapRenderPhase>("ShadowMap")
```

Render phases that have been registered are assign a unique index. For example, to get the render phase index of
`OpaqueRenderPhase` call `OpaqueRenderPhase::render_phase_index()`.

## Running a Render Phase

Generally this would happen from within a renderpass, possibly defined by the render graph.

```rust
graph_callbacks.set_renderpass_callback(node, move |args, user_context| {
    let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
    user_context
        .prepared_render_data
        .write_view_phase::<OpaqueRenderPhase>(&main_view, &mut write_context)
});
```