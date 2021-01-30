# Why Rafx?

Rafx is a multi-backend rendering framework targeted specifically at games and tools for games. It provides:
 * Low-level hardware abstraction (`rafx-api`)
 * A modern rendering framework (`rafx-framework`)
 * Integration with the `distill` asset pipeline (`rafx-assets`)
 * A shader authoring workflow `rafx-shader-processor` and other tools

## rafx-api

Rust already has an amazing selection of low-level rendering crates: `gfx-hal`, `wgpu`, `vulkano`, `glium`, lots of
raw bindings to platform APIs like `ash` and `metal`, and more. So it's fair to ask, why make another one?

Rafx intends to support multiple platform APIs, so as of this writing, `gfx-hal` or other APIs built on top of it like
`wgpu` are the only choices that meet this criteria.

### Compared with gfx-hal

`gfx-hal` is an unsafe API that closely follows the API design of vulkan. Like vulkan, it heavily favors flexibility
and performance over ease of use. The API exposes concepts and features that do not always exist in other platform APIs
or may difficult to emulate. When necessary, `gfx-hal` goes to great length to hide a platform API's lack of native
support for the exposed API.

`rafx-api` is also unsafe, but has a reduced API footprint that is easily supported across modern platform APIs.
This keeps backends simple to read and debug, while hopefully improving ease of use. In some cases, the provided API
will not be sufficient, so `rafx-api` fully exposes the underlying platform APIs and resources. This allows full, native
control and access to the very latest features in the underlying platform API.

### Compared with wgpu

`wgpu` is a fully safe API that closely follows the webgpu standard. It pursues safety at any cost because the API is
intended to be exposed in web browsers - where any form of undefined behavior is unacceptable. However this safety
combined with a less vulkan-centric API design makes it much easier to use than `gfx-hal` and it has become very popular
in the rust community. It is under the MPL license which is more restrictive than licenses like MIT or Apache 2.0.

`rafx-api` does not have these safety guarantees (or complexity/overhead required to support them). However,
`rafx-framework` provides higher-level tools and abstractions that mitigate this unsafety. For example, the render graph
automatically handles placing resources into the correct state. It can potentially do this in a more optimal way because
it has full knowledge of what will happen during the entire frame. `rafx-api` is available under the very permissive
Apache-2.0/MIT license.

## rafx-framework

While there are many low-level rendering solutions in Rust, fewer high-level solutions exist.

`rafx-framework` provides a rendering framework based on ideas that are in use in modern, shipping AAA games.

* The job/phase rendering design is inspired by the 2015 GDC talk "[Destiny's Multithreaded Rendering Architecture](http://advances.realtimerendering.com/destiny/gdc_2015/Tatarchuk_GDC_2015__Destiny_Renderer_web.pdf)".
* The render graph is inspired by the 2017 GDC talk "[FrameGraph: Extensible Rendering Architecture in Frostbite](https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-Rendering-Architecture-in)"
    * see also "[Render Graphs and Vulkan - a deep dive](http://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/)" for an implementation of a render graph on vulkan
    
`rafx-framework` provides reference counted GPU resources, descriptor set management, a render graph, a material
system, and flexible handling for vertex data at runtime.

Additionally, `rafx-framework` can use additional metadata extracted by `rafx-shader-processor` to implement time-saving
features like automatically binding [immutable samplers](shaders/shader_annotation.md#immutable_samplers) and 
[uniform buffers](shaders/shader_annotation.md#internal_buffer) to descriptor sets.

## rafx-assets

Rafx includes full integration with the `distill` asset pipeline. It supports many advanced features such as streaming
assets to remote devices and hot reloading of data.

See [`Distill`](https://github.com/amethyst/atelier-assets/tree/master) for more info.

## rafx-shader-processor

Rafx includes a shader processor the pre-cooks shaders for multiple platforms. This avoids needing to do just-in-time
shader translation/compilation on end-user devices. This permits many heavy non-rust dependencies to be excluded from
the game itself.

Additionally, the shader processor can [generate rust code](shaders/generated_rust_code.md) that provides a type-safe
interface for working with descriptor sets compatible with a given shader. 













