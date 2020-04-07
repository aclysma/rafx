# Render Process

The end user will create render objects. These can be high-level concepts like meshes and sprites.

The user must register the features and phases they will use:
 - A feature could be something like a Mesh, Sprite, etc. A *thing* that can be rendered.
 - A phase is a particular point in the rendering process. For example, "render opaque objects" or "apply emissive decals".

Every frame, the end user create multiple views. These can be thought of as cameras. They have their own
view*proj matrix and include a mask that enables and disables phases for that view. (For example, a view that generates
shadows can skip most phases.)

For each view, visibility returns a list of generic render object handles. A handle is defined like:

```rust
struct GenericRenderHandle {
    feature_index: u32,
    instance_index: u32 // (each individual instance of a thing has its own index within the feature)
}
```

We can represent this as a Vec<Vec<u32>> accessed like:

```
generic_handle_set[feature_index] = { all slab indexes for this feature }
```

We now allocate a single frame packet and allocate a view packet for each view. We populate the the frame/view packet
with per-frame and per-view nodes for each visible render object.

```
struct PerFrameNode {
    feature_index: u32,
    instance_index: u32,

    // Any data we want to have cache-coherent can be here as long as it stays compact
    bounding_sphere: BoundingSphere
}

struct PerViewNode {
    frame_node_index: u32,

    // Any data we want to have cache-coherent can be here as long as it stays compact
    distance_from_camera: f32
}
```

This will involve each view iterating through the list of visible objects within that view, allocating its own view nodes
and some method of allocating shared frame nodes for each view.

Now we can use callbacks for each feature, passing all the nodes relevant to that feature:
 - Begin Extract
 - Per-Frame Extract
 - Per-View Extract
 - Per-View Finalize
 - Per-Frame Finalize

This will cache dynamic data as needed into the frame packet. This ends the extraction phase and after this point, we do
not read from game state.

For the prepare step, pre-processing can occur. Callbacks will be called per-feature:
 - Begin Prepare
 - Per-Frame Prepare
 - Per-View Prepare
 - Pre-View Finalize
 - Per-Frame Finalize
 
This will produce submit node blocks for each phase. For example, if a feature has object A that is relevant in phase
index 0, and object B that is relevant in phase index 2, we will produce submit nodes for phase index 0
(containing {A, B}) and phase index 2 (containing {B})

Submit nodes may be sorted by feature, depth, etc.

During the submit step, the user would call a function:

```
submit_render_stage_for_view(view, stage);
```

This would iterate the appropriate submit node blocks, calling these callbacks.
 - Block Begin
 - Submit Node
 - Block End
