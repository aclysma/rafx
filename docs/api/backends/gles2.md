# OpenGL ES 2.0 Backend Design Notes

## Minimum Requirements

The OpenGL ES 2.0 backend is intended primarily for WebGL, but might also be useful with very old mobile devices. It
currently does not require any extensions.

## Status

WebGL and GLES2 are very old APIs and are very limited. Many features in `rafx-api` will not work properly. It is
probably best to limit usage to simple 2d rendering.

## Platform Support

While OpenGL is available across a wide range of hardware/platforms, initialization is very platform-dependent.
Currently initialization only supports windows, macOS, linux, and web browsers. Support for Android would
require implementing initialization with EGL.

Technically, when this backend is running on desktop, it is using desktop OpenGL, not ES. The backend has a few tweaks
to make this work transparently. More details below!

## Implementation Notes

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
* NVIDEA Nsight
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
 * OpenGL ES 2.0 only supports a single vertex array object (VAO) that is always bound. To emulate this, if we are on
   desktop GL, we create a single VAO and leave it bound.
 * OpenGL ES 2.0 does not support validation (it was introduced in 3.2). But it is often available on desktop and will
   be turned on if enabled and available.
 * OpenGL ES 2.0 supports very few texture formats. sRGB formats require an extension
 * The API for enumerating OpenGL extensions is different, this is transparently handled.

### Framebuffers

The backend creates a single framebuffer and changes it on every render pass. This could be replaced with a
hashing/caching mechanism similar to the vulkan backend if it becomes a performance problem.

### Render Targets

OpenGL can draw to textures, render targets, and the "default" framebuffer. For simplicity, rafx-api always renders to
textures, except for the final present to the "default" framebuffer.

Base OpenGL ES 2.0 does not support SRGB textures or depth textures. The backend does not implement these
extensions yet. The lack of depth textures limits what can be done with the render graph in `rafx-framework`.

### Formats

Notably sRGB is not supported in base OpenGL ES 2.0. The GL_EXT_sRGB adds support, but this is not implemented.

### Binding uniform values

OpenGL ES 2.0 does not support uniform block objects (UBO). This means uniform values must be set member-by-member.
`rafx-api`'s API only supports setting the entire uniform at once (like other APIs). To emulate this, reflection data
must be provided to the API. Reflection can be generated automatically offline by `rafx-shader-processor`. If you use
other rafx crates to load the shader, this is handled for you transparently.

### Texture Units

Legacy GLSL does not support separate texture/sampler objects. If you use other rafx crates to load the shader, this is
handled for you transparently.

### Passing data between vertex/fragment shaders

In legacy GLSL, to pass information between vertex/fragment shaders, the variables must have the same name.
`rafx-shader-processor` renames the in/out variables based on their location. (i.e. in a vertex shader, 
`layout(location = 0) out vec4 out_color;` becomes `out vec4 interface_var_0;`.

## Future Work

* Support EGL
* More control over enabling/disabling debugging
* Ability to opt-out of the final Y-flip
* A few missing features (like arrays of textures)
* Additional extension support such as 3d textures, depth textures, and sRGB formats
* Descriptor set "default" values (i.e. bind a 1x1 texture)