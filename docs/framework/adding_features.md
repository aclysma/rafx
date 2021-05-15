# Adding Features

Features represent "things" that can be drawn. For example, the could be separate features for meshes, sprites, cloth, 
debug draw, imgui, etc.

## Declare the Feature

You may either implement `RenderFeature` or use this macro

```rust
use rafx::render_feature_mod_prelude::*;
rafx::declare_render_feature!(Debug3DRenderFeature, DEBUG_3D_FEATURE_INDEX);
```

When using `rafx-renderer`, you should implement `RenderFeaturePlugin`.

## Register the Feature

```rust
// Create the registry
let render_registry = rafx::nodes::RenderRegistryBuilder::default()
    .register_feature::<SpriteRenderFeature>()
    .register_feature::<MeshRenderFeature>();
```

Features that have been registered are assign a unique index. For example, to get the feature index of `MeshRenderFeature` 
call `MeshRenderFeature::feature_index()`.

If using a `RenderFeaturePlugin`, the call to `register_feature` should go into `configure_render_registry`. This will be called
after the `RenderFeaturePlugin` is registered with the `Renderer`. A `RenderFeaturePlugin` can be registered with the `Renderer`
by calling `add_render_feature` on the `RendererBuilder`.

## Define the Frame Packet and Submit Packet

Each `RenderFeature` defines two data structures of packed arrays -- the `RenderFeatureFramePacket` and the `RenderFeatureSubmitPacket`. 
- `RenderFeatureFramePacket` contains the data extracted from the game world.
- `RenderFeatureSubmitPacket` contains data prepared for the GPU and a list of sortable submit nodes for the `RenderFeatureWriteJob`.

See the demo for examples of features implementing the `FramePacket` and `SubmitPacket`.

## Implement the `RenderFeatureExtractJob`, `RenderFeaturePrepareJob`, and `RenderFeatureWriteJob`

- `RenderFeatureExtractJob` queries data from the game world and copies it into the `RenderFeatureFramePacket`. 
- `RenderFeaturePrepareJob` processes extracted data into GPU-friendly data and creates the list of sortable submit nodes in the 
`RenderFeatureSubmitPacket`.
- `RenderFeatureWriteJob` defines the GPU commands for rendering each submit node.

See the demo for examples of features implementing the `RenderFeature` jobs.
