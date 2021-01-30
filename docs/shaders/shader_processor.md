# Rafx Shader Processor

The shader processor reads GLSL and produces several outputs, including MSL source code, rust source code, compiled
vulkan SPV, and a custom "package" format that can be used to create a shader at runtime in a cross-platform way.

`rafx-shader-processor` uses `spirv_cross` to read and translate between shader languages.

![Diagram showing input and outputs of the shader processor](../shader-processor.png)

## Usage

```
USAGE:
    rafx-shader-processor [FLAGS] [OPTIONS]

FLAGS:
    -h, --help                Prints help information
        --optimize-shaders    
        --trace               
    -V, --version             Prints version information

OPTIONS:
        --cooked-shader-file <cooked-shader-file>                
        --cooked-shaders-path <cooked-shaders-path>              
        --glsl-file <glsl-file>                                  
        --glsl-path <glsl-path>...                               
        --metal-generated-src-file <metal-generated-src-file>    
        --metal-generated-src-path <metal-generated-src-path>    
        --rs-file <rs-file>                                      
        --rs-path <rs-path>                                      
        --shader-kind <shader-kind>                              
        --spv-file <spv-file>                                    
        --spv-path <spv-path>      
```

### Inputs

 * `--glsl-file`/`--glsl-path`: A single GLSL file or a directory containing GLSL files.
 * `--optimize-shaders`: Produce optimized shaders (also strips debug information)
 * `--trace`: Increased logging
 * `--shader-kind`: Specify the stage the shader is intended for (i.e. vertex, frag, compute...). This is generally
   automatically detected and not necessary to specify.

### Outputs

 * `--cooked-shader-file`/`--cooked-shader-path`: Produce a binary (bincode format) that contains everything needed to
   create the shader at runtime at the specified file or path. `rafx-assets` includes a `distill` importer for this 
   format. 
 * `--metal-generated-src-file`/`--metal-generated-src-path`: Produce MSL source code at the specified file or path. This can either be loaded at
   runtime, compiled, or just used for debugging/reference.
 * `--rs-file`/`--rs-path`: Produce rust code for `@[exported]` elements in the shader at the specified file or path
 * `--spv-file`/`--spv-path`: Produce SPIR-V for the shader at the specified file or path.

When the "file" variants are used, `rafx-shader-processor` reads a single file and writes single files. With the "path"
variant is used, `rafx-shader-processor` reads all shaders matching a glob and writes a file for each input at the
provided paths.


### Example

`cargo run --package rafx-shader-processor -- --glsl-path glsl/*.vert glsl/*.frag glsl/*.comp --rs-path src --cooked-shaders-path ../../assets/shaders`

 * Read *.vert, *.frag, *.comp files from glsl/
 * Write rust code to src/
 * Write cooked shaders to ../../assets/shaders

## Supported Input Formats

`rafx-shader-processor` currently supports just GLSL. Internally, the shader processor uses `spirv_cross`, so support
for other languages like HLSL might not be too difficult to add in the future.

There are also some projects like [`rust-gpu`](https://github.com/EmbarkStudios/rust-gpu) to write shaders
in rust. While this is an exciting area of development, rafx will prioritize production-ready workflows.

## Supported Output Formats

`spirv_cross` can can read source code written in one language (like HLSL and GLSL) and output source code for a
different language (like MSL).

This translation process is mostly automatic and 1:1, but there are a few key places where additional information is
need to do the translation. Some of this is automatically generated by the shader processor, and some of it must be
provided via custom annotation in the shader.

## Shader Annotation and Code Generation

GLSL does not support a native form of annotation in the language. However, `rafx-shader-processor` looks for markup
in comments.

See also: [Shader Annotation](shader_annotation.md) and [Generated Rust Code](generated_rust_code.md)

### Example 1: Automatically Creating and Binding an Immutable Sampler

This will automatically set up a sampler bound to this descriptor set. No changes in your code required! (**Requires
using `DescriptorSetAllocatorManager` in `rafx-framework`!**):

```c
// @[immutable_samplers([
//     (
//         mag_filter: Nearest,
//         min_filter: Nearest,
//         mip_map_mode: Linear,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//     )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;
```

### Example 2: Generating Rust Code that Matches the Shader

The `@[export]` annotation will cause the shader processor to generate rust code for this descriptor set with accessors
to set the texture like this.

```c
// @[export]
layout (set = 0, binding = 0) uniform PerViewData {
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
    ShadowMap2DData shadow_map_2d_data[32];
    ShadowMapCubeData shadow_map_cube_data[16];
} per_view_data;
```

```rust
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PerViewDataStd140 {
    pub ambient_light: [f32; 4],                             // +0 (size: 16)
    pub point_light_count: u32,                              // +16 (size: 4)
    pub directional_light_count: u32,                        // +20 (size: 4)
    pub spot_light_count: u32,                               // +24 (size: 4)
    pub _padding0: [u8; 4],                                  // +28 (size: 4)
    pub point_lights: [PointLightStd140; 16],                // +32 (size: 1024)
    pub directional_lights: [DirectionalLightStd140; 16],    // +1056 (size: 1024)
    pub spot_lights: [SpotLightStd140; 16],                  // +2080 (size: 1536)
    pub shadow_map_2d_data: [ShadowMap2DDataStd140; 32],     // +3616 (size: 2560)
    pub shadow_map_cube_data: [ShadowMapCubeDataStd140; 16], // +6176 (size: 256)
} // 6432 bytes
```

In addition to struct, `rafx-shader-processor` will also generate code for creating/setting descriptor sets that contain
this field. (See [Generated Rust Code](generated_rust_code.md) for more details)