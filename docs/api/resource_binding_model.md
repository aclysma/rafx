# Resource Binding Model

Rafx uses a resource binding model that is nearly identical to vulkan. Shader resources are bound to separate
descriptor sets, and multiple sets can be bound at once. Rafx limits the number of sets to 4 because this is the
maximum guaranteed to be supported by vulkan.

The most common way to use descriptor sets is to group resources by "frequency". For example, if the currently bound
pipeline will render meshes, some data like the view/projection matrix will rarely change, but other data like the
current mesh's material properties or a position/rotation/scale matrix might change on every draw call.

In this example, the once-per-frame data can be grouped in set 0 and the per-drawable-object data can be grouped in set 1.

## How this maps to platform APIs

The Rafx Shader Processor will translate your shader into a form appropriate for each backend. Internally it uses
`spirv-cross` so more information on how this translation process works can be found there.

When using the shader processor and rafx framework, the implementation details can generally be ignored, but if you
decide to provide a custom shader, the following details may be useful:

### Vulkan

For vulkan, this mappin is 1:1. When a GLSL shader has something like:

```c
layout (set = 0, binding = 0) uniform textureCube shadow_map_images[32];
layout (set = 0, binding = 1) uniform textureCube shadow_map_images_cube[16];
```

The first resource will be bound to set 0, binding 0. The second resource will be bound to set 0, binding 1.

### Metal

Metal offers "argument buffers" as a way to efficiently bind a group of resources in a single operation. However,
there are a few key differences in how MSL bindings work.

The rafx shader processor would generate MSL for the above something like this:

```c
struct spvDescriptorSetBuffer0
{
    array<depth2d<float>, 32> shadow_map_images [[id(0)]];
    array<depthcube<float>, 16> shadow_map_images_cube [[id(32)]];
};
```

Metal requires that an array of N resources takes up N "slots". However, when using the Rafx API, use binding 0 for
`shadow_map_images` and binding 1 for `shadow_map_images_cube`.
