# Visibility Resource

![Overview](../images/visibility_resource.png)

A `VisibilityResource` is a ref-counted wrapper around 2 `Zones`.  One of the zones is designated "static" and the other 
designated "dynamic". When registering objects with the resource, the application may pick if it is "static" or "dynamic" 
-- this will assign it to the corresponding `Zone` and return a `VisibilityObjectArc`. Note that the "static" or "dynamic" 
assignment is not a meaningful distinction by the underlying `VisibilityWorld` -- it is ok to move an object that was 
registered as "static". The reason for separating "static" and "dynamic" assignments is to support a future capability 
for running the "static" visibility calculation earlier in the frame and combining it with the "dynamic" visibility later 
in the frame. [1] There are also occasions where only static or dynamic objects will be rendered (i.e. caching shadow maps)
An application view is registered with the `VisibilityResource` and returned as a `ViewFrustumArc`.

The `VisibilityObjectArc` is a ref-counted wrapper around an `VisibilityObjectHandle`. This struct contains functions for setting 
the position, `id`, and other fields. The set functions are implemented under the hood using async commands over a channel 
to the `VisibilityWorld`. Each `VisibilityObjectArc` contains a list of features registered with that handle. Particular 
features may be shown or hidden on an entity in the world by adding or removing the feature from the `VisibilityObjectArc` 
associated with that entiy.

The `ViewFrustumArc` is a ref-counted wrapper around 1 or 2 `ViewFrustumHandle` representing a view of the "static" `Zone` 
and a view of the "dynamic" `Zone` in the `VisibilityResource`. This struct contains functions for setting the location, 
`id`, projection, and querying for visibility. The set functions are implemented under the hood using async commands over 
a channel to the `VisibilityWorld`. Each `RenderView` requires a `ViewFrustumArc` so that visibility can be calculated 
for that view.

`ObjectId` is a helper to transmute between a struct `T: 'static + Copy + Hash + Eq + PartialEq` with the same size as a 
`u64` and the `u64` required for the `id` in the `VisibilityWorld`. 

```rust
let entity = world.push((transform_component.clone(), mesh_component));
let mut entry = world.entry(entity).unwrap();
entry.add_component(VisibilityComponent {
    visibility_object_handle: {
        let handle = visibility_resource.register_static_object(
            ObjectId::from(entity),
            CullModel::VisibleBounds(load_visible_bounds(&floor_mesh_asset)),
        );
        handle.set_transform(
            transform_component.translation,
            transform_component.rotation,
            transform_component.scale,
        );
        handle.add_render_object(&floor_mesh_render_object);
        handle
    },
});
```

When a `RenderView` is needed in the current frame, the associated `ViewFrustum` is queried for visibility. The visible 
`VisibilityObjectHandle`s are mapped to their `VisibilityObjectArc`s and the associated `RenderObject`s are added to the `FramePacket` 
of the relevant `RenderFeature` -- if the `RenderView` is registered for the `RenderFeature` and any `RenderPhase` required 
by it. 

[1] http://advances.realtimerendering.com/destiny/gdc_2015/Tatarchuk_GDC_2015__Destiny_Renderer_web.pdf
