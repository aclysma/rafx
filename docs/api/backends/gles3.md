# OpenGL ES 3.0 Backend Design Notes

## Minimum Requirements

The OpenGL ES 3.0 backend can be used with WebGL 2, mobile devices that do not support metal/vulkan, or as a desktop
falback if vulkan is not available (windows/linux only). It currently does not require or use any extensions.

## Status

**This backend is still under construction.** It was forked from the OpenGL ES 2.0 backend but is missing some features
that are available under OpenGL ES 3.0.

While OpenGL ES 3.0 and WebGL 2 are significant steps forward from OpenGL ES 2.0 and WebGL 1, they still do not offer
full feature parity with more modern graphics APIs like DirectX 12, Metal, and Vulkan. Some features in `rafx-api`
may not work properly. While it may be possible to implement 3D techniques with this backend, it will likely require
compromises that will prevent taking full advantage of more modern APIs.

It may be possible to mitigate most of the limitations with OpenGL ES 3.1/3.2 support or extensions, but we would need
to decide on a way to make these features available to people who want them, while also giving clear feedback when
someone tries to use them without opting-in to the additional functionality at the cost of reduced compatibility.

## Platform Support

While OpenGL is available across a wide range of hardware/platforms, initialization is platform-dependent.
Currently initialization only supports windows, linux, and web browsers. Support for Android would
require implementing initialization with EGL. macOS does not work as its OpenGL implementation does not support the
GL ES 3.0 shader dialect.

Technically, when this backend is running on desktop, it is using desktop OpenGL, not ES. The backend has a few tweaks
to make this work transparently. More details below!

## Implementation Notes

### Limitations of OpenGL ES 3.0

An OpenGL context may not be used by multiple threads simultaneously. So when this backend is in use, the same
restriction applies to `rafx-api`.

Some features that are not available in GL ES 3.0 (some of these can be addressed with extensions):
* No compute shaders (introduced in GL ES 3.1)
* GLSL does not support dynamically indexing arrays of textures

Some features in this backend are missing or could be improved using support newly-introduced in 3.0:
* More texture formats
* 3D textures
* Sampling from and rendering to specific mip levels
* Use sampler objects to reduce API calls
* Use proper OpenGL fences instead of emulating them
* MSAA support
* Instanced drawing

### Coordinate System

The OpenGL coordinate system is quite different from other GPU APIs like DirectX and Metal. We resolve this by patching
the vertex shader during shader translation via spirv-cross. More details here:
 * https://github.com/KhronosGroup/SPIRV-Cross#clip-space-conventions

This essentially inserts the following at the end of a vertex shader.

```
gl_Position.z = 2.0 * gl_Position.z - gl_Position.w;
gl_Position.y = -gl_Position.y;
```

This allows the OpenGL backend to accept the same UV coordinates and projection matrices as the other backends.

Because this inverts the Y axis, we insert a final full-screen blit of the image that "undoes" this Y flip.
Additionally, tools like renderdoc will show rendered images as being upside down. (There is a button for flipping it
right side up in renderdoc.)

### Validation

OpenGL has logging that can be enabled on desktop OpenGL systems (but is not technically part of the base standard.)
This was not added to OpenGL ES until version 3.2. Some feedback is also available when running in a web browser.

This functionality is not properly exposed in the public API yet.

### Debugging

These tools can be used to debug/trace a frame
* Renderdoc
* NVIDIA Nsight
* Xcode (Haven't confirmed myself)

### Shader Translation

`rafx-api` accepts OpenGL source code. It may be possible to add support for glShaderBinary, but it is currently not
implemented.

`rafx-api` assumes that render targets are upside down and inserts a final Y-flip when presenting. See
"Coordinate System" section for more details

Additionally, samplers and shader in/out variables are renamed:
 * Values passed between vertex/fragment shaders are renamed to interface_var_0 for location 0, etc.
 * Textures are translated to individual samplers. `rafx-api` will set sampler state on all of these samplers, even
   if in other APIs they would share the same sampler state.

### Loading OpenGL

`rafx-api` includes a fork of the [raw-gl-context](https://github.com/glowcoil/raw-gl-context) crate, with
modifications. (Most of the changes have been PR'd to the original project, but it does not seem to be actively
maintained.)

Currently windows, macOS, and x11 are supported. Support for more platforms, particularly EGL, would be nice to add
(and would make a great PR!)

### OpenGL ES vs. "Desktop" OpenGL

There are a few subtle differences between OpenGL ES and OpenGL running on a desktop:
 * OpenGL ES 3.0 does not support validation (it was introduced in 3.2). But it is often available on desktop and will
   be turned on if enabled and available.
 * sRGB conversion is always enabled in OpenGL ES 3.0. On desktop, it can be enabled/disabled, and is disabled by
   default. This backend enables it immediately so that it functions similarly to ES 3.0.
 * The API for enumerating OpenGL extensions is different, this is transparently handled.

### Framebuffers

The backend creates a single framebuffer and changes it on every render pass. This could be replaced with a
hashing/caching mechanism similar to the vulkan backend if it becomes a performance problem.

### Render Targets

OpenGL can draw to textures, render targets, and the "default" framebuffer. For simplicity, rafx-api always renders to
textures, except for the final present to the "default" framebuffer.

### Texture Units

Legacy GLSL does not support separate texture/sampler objects. If you use other rafx crates to load the shader, this is
handled for you transparently.

### Passing data between vertex/fragment shaders

In legacy GLSL, to pass information between vertex/fragment shaders, the variables must have the same name.
`rafx-shader-processor` renames the in/out variables based on their location. (i.e. in a vertex shader, 
`layout(location = 0) out vec4 out_color;` becomes `out vec4 interface_var_0;`.

## Synchronization

Semaphores are no-ops. Fences are simuated in a very coarse-grained way by injecting a glFlush as needed during present
and setting a user-space boolean to indicate that the flush call was made

Multithreaded usage is not allowed at all in this backend.

## Future Work

* Add more OpenGL ES 3.0 features
* Support EGL
* More control over enabling/disabling debugging
* Ability to opt-out of the final Y-flip
* Additional extension support
* Descriptor set "default" values (i.e. bind a 1x1 texture)
