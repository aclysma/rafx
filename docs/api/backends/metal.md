# Metal Backend Design Notes

## Minimum Requirements

`rafx-api` requires metal 2.0. iOS GPU family 3 or higher is recommended. (A9 or higher on mobile) 

For more info, see the [Metal Feature Set Tables](https://developer.apple.com/metal/Metal-Feature-Set-Tables.pdf)
provided by Apple.

## Status

Metal support is fairly complete but could probably be optimized. See Future Work.

## Platform Support

Metal is supported on macOS and iOS only. A9 or higher is recommended on mobile. 

## Implementation Notes

### Coordinate System

`rafx-api` uses the default metal coordinate system with no modifications.

### Validation

Set the environment variable `METAL_DEVICE_WRAPPER_TYPE=1`

### Debugging

Xcode can be used to debug/trace a frame. It can be used by rust by making an empty xcode project and changing the
debug settings to launch an external program.

### Shader Translation

`rafx-api` accepts metal source code or compiled shader libraries. You can compile this yourself, or use
`rafx-shader-processor`. The shader processor currently uses vulkan GLSL as input. `spirv-cross` is used to translate
the shader to metal shader language. There are a few things about this process to be aware of:
 * We translate vulkan descriptor sets into metal argument buffers
 * The argument buffer definition must be identical in the vertex/fragment shaders.   
 * The bindings in metal must be sequential, 0..n. The shader processor remaps the bindings in the GLSL to match. If you
   use `rafx-framework` or other higher-level rafx crates to load the shader, the setup and API will handle this for you
   automatically.
 * Immutable samplers in MSL are called const samplers. These are specified in the generated MSL.

### Subpasses/Tile Shading

Tile shading is not supported in rafx-api. It's unclear how metal's abstraction can be mapped to other backends.
`rafx-api` makes the internally created metal objects available to you, so you can always natively implement a
performance-critical part of your pipeline if needed.

## Future Work

* Descriptor set "default" values (i.e. bind a 1x1 texture)
* Inline constant data in argument buffers
* Use metal heaps for better performance
* A general optimization pass over the backend. (For some scenes with few draw calls, the vulkan backend is slightly
  faster, which is not expected!)
