# Why Rafx?

Rafx is a multi-backend rendering framework targeted specifically at games and tools for games. It provides:
 * Low-level hardware abstraction (`rafx-api`)
 * A modern rendering framework (`rafx-framework`)
 * Integration with the `distill` asset pipeline (`rafx-assets`)
 * A shader authoring workflow `rafx-shader-processor` and other tools

While there are many crates similar in scope to `rafx-api` there are fewer high-level options in the rust ecosystem.
The full stack of Rafx crates aims to be similar in scope to Ogre3D or Horde3D, but modernized and following
industry practices where possible.

## rafx-api

Rust already has an amazing selection of low-level rendering crates: `wgpu`, `vulkano`, `glium`, lots of
raw bindings to platform APIs like `ash` and `metal`, and more. So it's fair to ask, why make another one?

Rafx intends to support multiple platform APIs, and as of late 2020 when `rafx-api` was created, `gfx-hal` and `wgpu`
were the only choices that met this criteria. (As of late 2021, `gfx-hal` is deprecated and `wgpu` is the only choice
I'm aware of.)

### Compared with gfx-hal

`gfx-hal` was deprecated in mid-2021. [This issue](https://github.com/gfx-rs/gfx/discussions/3768) describes why, and
some of the issues mentioned there were factors in choosing not to use it. It was unsafe API that closely followed the 
API design of vulkan. Because it so closely followed vulkan, it had concepts and features that did not map cleanly to
other APIs we want to support. When necessary, `gfx-hal` went to great length to hide this, adding complexity.

`rafx-api` is also unsafe, but has a reduced API footprint that is more easily supported across modern platform APIs.
This keeps backends simple to read and debug, while hopefully improving ease of use. In some cases, the provided API
will not be sufficient, so `rafx-api` fully exposes the underlying platform APIs and resources. This allows full, native
control and access to the very latest features in the underlying platform API.

### Compared with wgpu

`wgpu` is a fully safe API that closely follows the webgpu standard. To ensure safety, GPU resources are tracked at
runtime so that they are not dropped while in use. Additionally, resources are automatically placed in the correct 
state for the GPU to use them. This safety combined with a less vulkan-centric API design made it much easier to use
than `gfx-hal`. It has since become very popular in the rust community.

`rafx-api` does not have these safety guarantees (or complexity/overhead required to support them). Instead,
`rafx-framework` provides higher-level tools and abstractions that mitigate this unsafety. For example, the render graph
automatically handles placing resources into the correct state. It can potentially do this in a more optimal way because
it has a holistic view of what will happen during the entire frame.

In mid-2020, `wgpu` was relicensed from MPL to the less restrictive MIT/Apache-2.0 license. Additionally, the structure
of the project was drastically simplified. In my opinion, these are great changes that make `wgpu` an easy
recommendation for many projects. Even so, I think `rafx-api` is sufficiently different from `wgpu` in engineering
tradeoffs to make it worth considering for certain kinds of projects.

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

See [`Distill`](https://github.com/amethyst/distill) for more info.

## rafx-shader-processor

Rafx includes a shader processor the pre-cooks shaders for multiple platforms. This avoids needing to do just-in-time
shader translation/compilation on end-user devices. This permits many heavy non-rust dependencies to be excluded from
the game itself.

Additionally, the shader processor can [generate rust code](shaders/generated_rust_code.md) that provides a type-safe
interface for working with descriptor sets compatible with a given shader.
