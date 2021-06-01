# Vulkan Backend Design Notes

## Minimum Requirements

The vulkan backend requires Vulkan 1.1. This could probably be relaxed to 1.0 with the `VK_KHR_Maintenance1` extension.
This extension is necessary to support negative viewport height, which allows vulkan to match the coordinate system
of DirectX and Metal.

The SDK must be installed (https://www.lunarg.com/vulkan-sdk/) to get validation. Additionally, macOS users *must* have
it installed for vulkan to work at all.

## Status

Vulkan was the first backend supported by Rafx, so it is one of the more mature backends. There are a few less-used
features that are not yet implemented, see Future Work.

## Platform Support

Detailed info here: https://vulkan.gpuinfo.org

Vulkan is well supported on a wide variety of desktop hardware and operating systems.
* [AMD](https://www.amd.com/en/technologies/vulkan) - R5 240, 500 series, etc. and up (~circa 2013)
* [Intel](https://www.intel.com/content/www/us/en/support/articles/000005524/graphics.html) - Skylake/HD 500 or later (~circa 2015)
* [NVIDIA](https://developer.nvidia.com/vulkan-driver) - Kepler/600 series and up (~circa 2012)

It is also supported on iOS (via MoltenVK) and some android devices.

Caveats:
* Keep in mind, old GPUs are more likely to have driver bugs. Mobile GPUs are also more likely to have bugs.
* Linux support can be problematic with certain driver/window manager combinations.
* Windows/Linux/Android are natively supported. macOS/iOS is supported via MoltenVK. (You may want to static link it
  on iOS).

## Implementation Notes

### Coordinate System

Rafx-api will flip the render target Y axis via a negative viewport height. This makes the coordinate system match
DirectX and Metal.

### Validation

By default, rafx-api will try to enable validation in debug mode. This requires installing the vulkan SDK. It is
highly recommended that you install the SDK and run with validation on from time to time.

### Debugging

These tools can be used to debug/trace a frame
 * Renderdoc
 * NVIDIA Nsight
 * Xcode

### Shader Translation

`rafx-api` accepts pre-compiled SPV. You can compile this yourself, or use `rafx-shader-processor`. The shader processor
currently uses vulkan GLSL as input, so there are no special considerations - the shader is used exactly as it is
provided.

### Linking Vulkan

Vulkan can be loaded dynamically or statically. Dynamic linking should be preferred and is the default. Static linking
is mostly useful for use with MoltenVK on iOS.

### Queues

Vulkan exposes a finite set of queues belonging to several queue families. The queue families provide different
capabilities. (Graphics/Compute/Transfer are the ones you likely want to use). See vulkan documentation for information
about queue families and what each family permits.

Rafx has two allocation strategies:
* ShareFirstQueueInFamily: Allocate one shared queue for each family
    * When you create a RafxQueue for a given RafxQueueType, the same queue will be returned. Creating a queue will
      never fail.
* Pool(n): Create a pool of N queues for the given family
    * Queues are allocated from the pool when a RafxQueue is created, and returned when it is destroyed. Allocation will
      fail if no queues are in the pool.

The default is ShareFirstQueueInFamily. Most commercial games only use one queue of each type. (Using more queues
generally does not increase performance - it merely splits available performance across the queues that are in use.)

### Framebuffers and Renderpasses

Vulkan has framebuffer and renderpass concepts that are not exposed in `rafx-api`. They are created automatically as
needed and hashed/reused. We retain the 200 most recent of each.

### Subpasses

Subpasses are not supported.
 * Historically they have not been very useful on desktop platforms. DirectX 12 added support for them only recently
 * They can be a significant win for mobile platforms (see Tile-Based Deferred Rendering) and metal has had similar
   support for a while. Unfortunately the two APIs are very different in design and it's unclear how they can be
   abstracted in a reasonable way.

### Descriptors

`rafx-api` internally maintains a pool of descriptors. The size is configured by RafxDescriptorHeapPoolConfig. When the
pool is exhausted, a new pool is created. This is invisible to end-users.

Keep in mind that `rafx-api` does not permit returning descriptors to the pool. Most users should use the higher-level
logic in `rafx-framework` to ensure that dropped descriptor sets are queued for later reuse.

## Future Work

 * Use `VK_KHR_descriptor_update_template` for more efficient descriptor set updates
 * Descriptor set "default" values (i.e. bind a 1x1 texture)
 * Push Constants
     * While these can be convenient, they are not necessarily a performance win vs. uniforms.
 * Dynamic uniforms
 * Better debugging tools, particularly around crash/GPU hang handling