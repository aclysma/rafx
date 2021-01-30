# API Design

Rafx has three layers:
 * rafx-api: A hardware abstraction layer
 * rafx-framework: Mid-level helpers and abstractions to make writing GPU code more productive
 * rafx-assets: Integration with the `distill` asset pipeline

Rafx also provides tools, the main one being `rafx-shader-processor` which accepts GLSL shaders and produces assets
necessary to create and use that shader on various platforms.

**This document covers just `rafx-api`.**

## Code Structure

 * rafx-api/src
     * \[root\]: The high-level abstraction layer
     * types: Many simple API-agnostic types. Things like `RafxFormat` or `RafxBufferDef`
     * backends
        * metal: The metal backend
        * vulkan: The vulkan backend
     * extra: Some utilities that are helpful but optional when using rafx
     * internal_shared: Internal code that is shared between backends
   
## Safety

Rafx is an unsafe API. Functions that are unsafe but purely touch CPU are marked as unsafe, but functions that are
unsafe simply because they interact with the GPU are not marked unsafe. (Because if they were, every single API in rafx
would be marked unsafe.)

GPU resources must not be dropped if the GPU is using them. Resources must also be in the correct state when they are
used. [Details on Safety in Rafx](safety.md) 

## Rafx "Objects" are Enums

High level rafx "objects" like `RafxBuffer` or `RafxTexture` are just enums. The
rationale:
 * Enums are simple and easy to use
 * Allows the same code to work with a backend chosen at runtime
 * Avoids monomorphization, which can negatively affect code size and compile times
 * API-specific objects (like `RafxBufferVulkan`/`RafxBufferMetal`) are always accessible
 
```rust
pub enum RafxBuffer {
    Vk(RafxBufferVulkan),
    Metal(RafxBufferMetal),
}
```

While it might first appear that the enum is adding overhead to access the underlying type, this is unlikely:
 * Compiling with only a single backend would result in a single-variant enum. The branch would be optimized out and the
   access inlined.
 * Even if it's not optimized away by the compiler (say a platform supports more than one API), modern CPU branch
   prediction would likely have a 100% hit rate if the same branch is taken every time.
   
## The API Entry Point: `RafxApi`

The first step to using the API is to initialize a `RafxApi`.

```rust
use rafx::api::RafxApi;
let mut api = RafxApi::new(&sdl2_systems.window, &Default::default())?;
```
   
This object may not be dropped until all objects created from it are dropped. `RafxApi::destroy()` can optionally be
called to ensure the rafx API completely shuts down at that point. Otherwise, it will be called automatically when
`RafxApi` is dropped.

Additionally, the underlying `RafxApiMetal`, `RafxApiVulkan`, etc. can be obtained by calling `RafxApi::metal_api()` or
`RafxApi::vk_api()`. The API of these objects is less stable and less documented, but this provides an escape hatch into 
the native API if you need it. For example, Rafx does not provide abstraction for raytracing (yet) but you can use Rafx
to set up the vulkan API for you and only write extra vulkan code where necessary.

## Device Contexts

Once `RafxApi` is created, you can get the `RafxDeviceContext` by calling `RafxApi::device_context()`. The device
context is a cloneable, thread-friendly object. You can create as many as you like.

Most GPU objects are created by calling `RafxDeviceContext::create_*`

```rust
// Get the device context from the API. Clone it and pass it around as much as you like
let device_context = api.device_context();

// Use the device context to create a buffer
device_context.create_buffer(&RafxBufferDef {
     size: 512,
     memory_usage: RafxMemoryUsage::CpuToGpu,
     resource_type: RafxResourceType::VERTEX_BUFFER,
     ..Default::default()
 })
```

All RafxDeviceContexts (and other Rafx API objects) must be destroyed before dropping `RafxApi` or calling 
`RafxApi::destroy`

## Resource Types and States

Some resources have a `RafxResourceType` and/or `RafxResourceState`

### RafxResourceType

Rafx does not provide separate types for every possible kind of resource. A `RafxTexture` could be read-only, read-write,
a cube texture, or some combination of the above. Setting a resource type flag enables the resource to be used in
additional ways, but may require additional resources to be created (like for vulkan, a vk::ImageView). In general,
specify only the type flags that you need

### RafxResourceState

Some operations require resources to be placed in the correct state. For example, a `RafxRenderTarget` may only be
presented if it is in the PRESENT state. However, while it is being written to, it must be in the `RENDER_TARGET` state.

States are required partially because underlying GPU APIs require them. (Like in vulkan, `VkImageLayout`).

However, rafx may inject some synchronization or memory barriers for you when a resource changes states. 

## Descriptor Sets

Descriptor sets can be quite tricky to manage and there are many possible approaches. The API level of rafx only 
provides a bare minimum layer on top of descriptor sets.

To create a descriptor set, allocate a `RafxDescriptorSetArray`. There is some memory overhead to creating this wrapper,
so it is expected that they will be allocated in blocks.

**You should try to reuse these blocks. A rafx backend implementation is allowed to permanently leak any descriptor
set arrays that you drop.** (You should pool and reuse them!)

`rafx-framework` provides a system for pooling and reusing descriptor sets in an easy, reference-counted way with many
ergonomic improvements over the low-level API.

## More Information

 * [Rendering Concepts](rendering_concepts.md)
 * [Rafx Binding Model](resource_binding_model.md)
