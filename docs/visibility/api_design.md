# API Design

`rafx-visibility` is a low-level crate used by `rafx-framework` for providing a `VisibilityWorld`. A `VisibilityWorld` is like a physics world -- it's a retained API that maintains a shadow state of the application's game world. The API encapsulates the game code from changes to the visibility's code algorithms or data structures. As new features are added to `rafx-visibility` like occlusion culling or hierarchical cull structures for large game scenes, those features will become available transparently to the applications relying on `rafx-visibility`.

## High-level Overview

### Public API

The public API to `rafx-visibility` is contained in `src/*` and `src/geometry/*`.

The main entry point is the `VisibilityWorldArc`. The application creates `VisibilityWorldArc` and registers new `Objects`, `Models`, `Zones`, `Volumes`, and `View Frustums` with it to mirror their game's scene. Additionally, the application can use a channel to send asynchronous commands to the `VisibilityWorldArc` in a thread-safe manner that can be processed during the next update of the `VisibilityWorld`.

The `VisibilityWorld` uses handles to avoid bleeding internal design into the application.

- A `ViewFrustumHandle` represents a view in the application, like the main camera, a mini-map, or a light source needed for shadow casting. It has a specific `Projection`. The supported `Projection` types are `Orthogonal` or `Perspective`.
- An `VisibilityObjectHandle` represents a game entity in the application. It is associated with a `Model` for culling and placed into a `Zone` with `View Frustums`.
- A `ZoneHandle` is used to group `Objects` and `View Frustums` together. `View Frustums` are only able to "see" `Objects` in the same `Zone`. `Objects` and `View Frustums` may not be in multiple zones simultaneously. This is similar to a layer in a physics API. In a future release, the intent is for `Zones` to be the way to connect easily segmented levels with `Portals`. [1]
- A `ModelHandle` represents the visibility bounds for a game entity in the application. Internally, it's represented by one of several possible bounding structures. In a physics API, this is like defining the collider for an entity in the physics world.
- A `VolumeHandle` is like a `Model`, but instead of being reduced down to the bounding structure, it maintains a well-defined geometric shape like a cone, sphere, capsule, or other supported 3-dimensional shape. Unlike a `Model`, a `Volume` is not associated with an `VisibilityObjectHandle`. It is placed into a `Zone` and positioned independently. `Volumes` are included in the list of visibility results from a `View Frustum` when the `Volume` intersects with that `View Frustum`. `Volumes` are currently not implemented. In the future, `Volumes` will be the primary way for the application to determine the visibility of light sources in a view for the purpose of culling unseen lights. [2]

The public API also includes a `VisibleBounds` struct for processing an arbitrary `PolygonSoup` provided by the application into the `VisibilityWorld`'s internal bounding structures. `VisibilityBounds` may be computed offline and provided to the `VisibilityWorld` during loading for initializing `Models`. The reason for preferring an internal structure is that the `VisibilityWorld` will need to simplify application models into simpler `OccluderBounds` in the future when occlusion culling is added.

### Visibility Queries

The `VisibilityWorldArc` defines the `query_visibility` function. This function is thread-safe.

```rust
pub fn query_visibility(
    &self,
    view_frustum: ViewFrustumHandle,
    result: &mut VisibilityQuery,
) -> Result<(), QueryError> 
```

The result of a `VisibilityQuery` is a `Vec` of `VisibilityResult<T>` where `T` is either an `VisibilityObjectHandle` or a `VolumeHandle`. Each `VisibilityResult` contains the following information:

```rust
pub struct VisibilityResult<T> {
    pub handle: T,
    pub id: u64,
    pub bounding_sphere: BoundingSphere,
    pub distance_from_view_frustum: f32,
}
```

- `handle` is the `VisibilityObjectHandle` or `VolumeHandle` visible to the `ViewFrustumHandle` during the query.
- The `id` is an application-provided value set & retained on each `VisibilityObjectHandle` or `VolumeHandle` with the `set_object_id` and `set_volume_id` functions. The `id` is intended to be used by the application as a key back to their game entity, e.g. a `ptr`, or an `Entity ID` in an `ECS`, or a key in some type of `Map`.
- The `bounding_sphere` and `distance_from_view_frustum` are provided to support applications maintaining a level-of-detail budget. [3]

## Internals

- `src/internal/*` defines the data structures and algorithm for thread-safe frustum culling.
- `src/frustum_culling/*` contains an SIMD algorithm for culling bounded spheres quickly and associated data structure -- the `PackedBoundingSphere` and `PackedBoundingSphereChunk`. [4] 


[1] https://en.wikipedia.org/wiki/Portal_rendering

[2] Lengyel, Eric. (2019). Foundations of Game Engine Development: Vol 2. Rendering. 

[3] http://advances.realtimerendering.com/destiny/gdc_2015/Tatarchuk_GDC_2015__Destiny_Renderer_web.pdf

[4] https://www.ea.com/frostbite/news/culling-the-battlefield-data-oriented-design-in-practice