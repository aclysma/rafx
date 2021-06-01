# OpenGL ES 2.0 Backend Design Notes

## Minimum Requirements

The OpenGL ES 2.0 backend is intended primarily for WebGL, but might also be useful with very old mobile devices. It
currently does not require or use any extensions.

## Status

Most of what is possible to do in OpenGL ES 2.0 without extensions has been implemented. This backend is relatively
less mature than the vulkan and metal backends.

OpenGL ES 2.0 and WebGL are old APIs and are limited in functionality and performance. Many features in `rafx-api` will not work
properly. It is probably best to limit usage to simple 2d rendering, unless you are willing to require extensions. In
general this backend is intended for broadest compatibility possible with minimal functionality to do basic 2d drawing.

## Platform Support

While OpenGL is available across a wide range of hardware/platforms, initialization is platform-dependent.
Currently initialization only supports windows, macOS, linux, and web browsers. Support for Android would
require implementing initialization with EGL.

Technically, when this backend is running on desktop, it is using desktop OpenGL, not ES. The backend has a few tweaks
to make this work transparently. More details below!

## Implementation Notes

### Limitations of OpenGL ES 2.0

An OpenGL context may not be used by multiple threads simultaneously. So when this backend is in use, the same
restriction applies to `rafx-api`.

Some features that are not available in GL ES 2.0 (some of these can be addressed with extensions):
* Only Uint16 index buffers are supported (requires OES_element_index_uint extension)
* Only 16-bit depth buffers (requires GL_OES_depth24/GL_OES_depth32 extensions)
* No native instanced drawing (might be possible to emulate, or use extensions EXT_draw_instanced/EXT_instanced_arrays)
* No compute shaders (introduced in GL ES 3.1)
* Cubemap sampling may not be seamless (introduced in GL ES 3.0)
* No MSAA on textures (requires GL_EXT_multisampled_render_to_texture)
* No 3D textures (requires GL_OES_texture_3D extension)
* No depth textures (requires GL_OES_depth_texture, GL_OES_depth_texture_cube_map, and GL_EXT_shadow_samplers to sample from them)
* No sRGB formats (requires GL_EXT_sRGB, 56% coverage)
* Poor support for sampling of specific mip levels
* No support for rendering to a specific mip level (requires OES_fbo_render_mipmap extension)
* Some texture sampling methods only work with power-of-two texture sizes
* GLSL does not support bool, uint, or textureSize()
* GLSL does not support dynamically indexing arrays of textures
* Extensions are required for all but the most basic texture formats

Some of these (like GL_OES_depth_texture) are widely supported, but some like GL_EXT_sRGB are surprisingly not.

When using WebGL, even if some widely supported extensions are available on hardware, they may not be exposed in
certain browsers. Additionally, there are some WebGL-only extensions (like WEBGL_depth_texture) that are similar but
not quite the same as their comparable extension in native OpenGL ES.

Also note that while generally in OpenGL ES an extension is "always on", in WebGL it must be explicitly enabled.

There are also limitations in the API that can be performance liabilities:
* No support for unified buffer objects, meaning at best, 1 API call per 16 bytes of uniform data. This adds up fast!
* No support for sampler objects. This results in several API calls to bind a single texture.
* Drawing indexed primitives with a vertex buffer offset is not natively supported, but emulated by rebinding the
  vertex buffer.

While this backend could be improved to support some of these features via extensions, this project is more likely
to focus on more recent backends, and only recommend the use of this backend for very simple workloads that require
very broad compatibility.

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

Notably sRGB is not supported in base OpenGL ES 2.0. The GL_EXT_sRGB adds support, but this is not implemented. As an
alternative, conversion from/to linear color space can be manually performed in a fragment shader.

### Binding uniform values

OpenGL ES 2.0 does not support uniform block objects (UBO). This means uniform values must be set member-by-member.
`rafx-api`'s API only supports setting the entire uniform at once (like other APIs). To emulate this, reflection data
must be provided to the API. Reflection can be generated automatically offline by `rafx-shader-processor`. If you use
other rafx crates to load the shader, this is handled for you transparently.

Keep in mind that the lack of UBO support may result in many API calls to bind a single uniform struct.

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

* Support EGL
* More control over enabling/disabling debugging
* Ability to opt-out of the final Y-flip
* A few missing features (like arrays of textures)
* Additional extension support such as 3d textures, depth textures, and sRGB formats
* Descriptor set "default" values (i.e. bind a 1x1 texture)
