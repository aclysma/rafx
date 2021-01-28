# Converting from GLSL to MSL

For the most part this conversion is automatically handled by `rafx-shader-processor`, which uses
[`spirv_cross`](https://github.com/KhronosGroup/SPIRV-Cross). However there are a few things worth
knowing as the shader languages do not match up perfectly 1:1.

## Entry Point Name

MSL has reserved names that cannot be used as entry point names. `spirv_cross` handles this by adding a 0 to the name.
For example, `main` becomes `main0`. This is hard-coded in the metal backend such that if you provide `main` as an
entry point, `main0` will be used instead.

## Immutable Samplers

In MSL, immutable samplers are known as constexpr samplers. Something like this:

```c
// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         mip_map_mode: Linear,
//         address_mode_u: Repeat,
//         address_mode_v: Repeat,
//         address_mode_w: Repeat,
//         max_anisotropy: 16.0,
//     )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;
```

Will be generated as a constant value 

```
fragment main0_out main0(...)
{
    constexpr sampler smp(filter::linear, mip_filter::linear, address::repeat, compare_func::never, max_anisotropy(16));
}
```

This does not require any handling in your code, it's just something to be aware of!

## Descriptor Sets and Argument Buffers

Rafx uses the vulkan model for handling descriptors. The closest analog to this in MSL is argument buffers. Metal
argument buffers are just normal buffers encoded in a special format. (This is handled for you in rafx-api).

The generated MSL code will include an argument buffer for each descriptor set used in GLSL.

Bindings within the set must have unique IDs. However, unlike GLSL, an array will span multiple IDs.

> **WARNING**: Because the array length of one field can affect the chosen ID for another field, all shader stages must
> see the same resources. It is best to put all resources in a .glsl file and #include them from the .vert and .frag
> files.

### Example

 * GLSL uses bindings 0, 3, and 4. Binding 1 and 2 are immutable samplers and don't need to be set
 * MSL expects bindings 0, 3..35, and 35..51. This is because the arrays contain 32 and 16 elements respectively

When working with the rafx API, always use the GLSL style (so binding 0, 3, and 4 in this example). However, if you use
the metal debugger in xcode or interact with the metal API directly, you will need to use the MSL-specific bindings
instead.

```c
layout (set = 0, binding = 0) uniform PerViewData {
    // ...
} per_view_data;
// @[immutable_samplers([...])]
layout (set = 0, binding = 1) uniform sampler smp;
// @[immutable_samplers([...])]
layout (set = 0, binding = 2) uniform sampler smp_depth;
layout (set = 0, binding = 3) uniform texture2D shadow_map_images[32];
layout (set = 0, binding = 4) uniform textureCube shadow_map_images_cube[16];
```

```c
struct spvDescriptorSetBuffer0
{
    constant PerViewData* per_view_data [[id(0)]];
    array<depth2d<float>, 32> shadow_map_images [[id(3)]];
    array<depthcube<float>, 16> shadow_map_images_cube [[id(35)]];
};
```