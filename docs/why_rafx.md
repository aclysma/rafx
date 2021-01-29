# Why Rafx?

Rafx is a multi-backend rendering framework targeted specifically at games and tools for games. It provides:
 * Low-level hardware abstraction (`rafx-api`)
 * A modern rendering framework (`rafx-framework`)
 * Integration with the `distill` asset pipeline (`rafx-assets`)
 * A shader authoring workflow `rafx-shader-processor` and other tools

## rafx-api

Rust already has an amazing selection of low-level rendering crates: `gfx-hal`, `wgpu`, `vulkano`, `glium`, lots of
raw bindings to platform APIs like `ash` and `metal`, and more. So it's fair to ask, why make another one?

Up to this point, graphics libraries in Rust have been very general. They can mostly be categorized as:
 * Very unsafe, fully general low-level APIs (i.e. `ash`, `gfx-hal`). These generally target the extreme end of maximum
   performance and flexibility, making them difficult to use correctly.
 * Completely safe, fully general, low-level APIs (i.e. `glium`, `vulkano`, `wgpu`). These APIs are intended to be 
   "safe at any cost." These safety guarantees have runtime cost.

I think the tradeoffs for these libraries are well-chosen for their intended purposes. However, I think the ideal game
development graphics abstraction for day-to-day use is is somewhere between these extremes.
   
Additionally, existing APIs tend to follow a native platform API (usually OpenGL or Vulkan) very closely, exposing 
every concept that exists in that API. In some cases when multiple platform APIs are supported, complex
solutions are employed to hide the platform's lack of native support for the exposed API.

`rafx-api` aims to be an unsafe API abstraction layer with a reduced API footprint that is easily supported across
modern platform APIs. This keeps backends simpler to understand and debug, while hopefully improving ease of use.

The API is roughly based on "[The Forge](https://github.com/ConfettiFX/The-Forge)", but it is a from-scratch, pure rust
implementation with changes in both API design and implementation.

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
[uniform buffers](shaders/shader_annotation.md#internal_buffer).

## rafx-assets

Rafx includes full integration with the `distill` asset pipeline. It supports many advanced features such as streaming
assets to remote devices and hot reloading of data.

See [`Distill`](https://github.com/amethyst/atelier-assets/tree/master) for more info.

## rafx-shader-processor

Rafx includes a shader processor the pre-cooks shaders for multiple platforms. This avoids needing to do just-in-time
shader translation/compilation on end-user devices. This permits many heavy non-rust dependencies to be excluded from
the game itself.
