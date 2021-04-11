# Adding Features

Features represent "things" that can be drawn. For example, the could be separate features for meshes, sprites, cloth,
debug draw, imgui, etc.

## Declare the Feature

You may either implement `RenderFeature` or use this macro

```rust
rafx::declare_render_feature!(Debug3DRenderFeature, DEBUG_3D_FEATURE_INDEX);
```

## Register the Feature

```rust
// Create the registry
let render_registry = rafx::nodes::RenderRegistryBuilder::default()
    .register_feature::<SpriteRenderFeature>()
    .register_feature::<MeshRenderFeature>();
```

Features that have been registered are assign a unique index. For example, to get the feature index of 
`MeshRenderFeature` call `MeshRenderFeature::feature_index()`.

## Implement `RenderNodeSet` and add it to the frame packet

See the demo for examples implementing `RenderNodeSet`. (Maybe there should be a macro to provide a default impl).

Render nodes will be updated during the extract phase and will "belong" to the render thread until the next extract
phase. 

```rust
// Create a frame packet builder. It needs to know about all the render nodes and views.
// (Setting views not shown here)
let frame_packet_builder = {
    let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
    sprite_render_nodes.update();
    let mut mesh_render_nodes = resources.get_mut::<MeshRenderNodeSet>().unwrap();
    mesh_render_nodes.update();
    let mut all_render_nodes = AllRenderNodes::default();
    all_render_nodes.add_render_nodes(&*sprite_render_nodes);
    all_render_nodes.add_render_nodes(&*mesh_render_nodes);

    FramePacketBuilder::new(&all_render_nodes)
};
```

## Implement `ExtractJob` and add it to an extract jobs set

See the demo for examples implementing `ExtractJob`.

```rust
// Create extract jobs for features we will render
let mut extract_job_set = ExtractJobSet::new();
extract_job_set.add_job(create_sprite_extract_job());
extract_job_set.add_job(create_sprite_extract_job());

// Kick off the extract
let frame_packet = frame_packet_builder.build();
extract_job_set.extract(&extract_context, &frame_packet, &extract_views)
```
