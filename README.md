# renderer_prototype (Name TBD)

This is a vulkan renderer built on top of the [`atelier-assets`](https://github.com/amethyst/atelier-assets) asset 
pipeline. It's intended to be performant, flexible, workflow-friendly, and suitable for use in real-world projects in a 
team environment.

The asset pipeline is designed with user workflow in mind (including dedicated artists!), supporting workflow-friendly
features like hot reloading assets, including on remote devices. The architecture of the renderer is intended to support
advance use-cases such as streaming, LODs, visibility systems, and multi-threaded draw call submission. 

Extending and using this crate directly requires some understanding of vulkan. However, there are many tools to make
iteration much faster such as a **render graph** and **auto-generated shader bindings**.

Supported Platforms:
 * Windows
 * macOS (via MoltenVK)
 * iOS (via MoltenVK)
 * Linux

Android might work but I don't have hardware to test with.

References:
 * The job/phase rendering design is inspired by the 2015 GDC talk "[Destiny's Multithreaded Rendering Architecture](http://advances.realtimerendering.com/destiny/gdc_2015/Tatarchuk_GDC_2015__Destiny_Renderer_web.pdf)".
 * The render graph is inspired by the 2017 GDC talk "[FrameGraph: Extensible Rendering Architecture in Frostbite](https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-Rendering-Architecture-in)"
     * see also "[Render Graphs and Vulkan - a deep dive](http://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/)"  

[![Build Status](https://github.com/aclysma/renderer_prototype/workflows/CI/badge.svg)](https://github.com/aclysma/renderer_prototype/actions)

[![Video of Renderer in Use](docs/ios-screenshot.png)](https://www.youtube.com/watch?v=Ks_HQbejHE4 "Video of Renderer in Use")

[^ Video of this renderer running on iOS hardware](https://www.youtube.com/watch?v=Ks_HQbejHE4) 

![Screenshot demonstrating realtime shadows](docs/shadow-screenshot.png)

## Diagrams

 * [Diagram of key crate dependencies](docs/crate_dependencies.png)
 * [Pipelining](docs/pipelining.png)
 * [Diagram of rendering process](docs/render_process.png)

## Status

This project should still be considered a prototype, shared for informational purposes only. Please don't use it in
anything real yet!

The demo includes:
 * Render thread decoupled from main thread [(diagram)](docs/pipelining.png)
 * Asynchronous asset loading
 * Assets can be streamed to remote hardware (i.e. a phone)
 * Hot-reloading assets (needs more work, some asset types do not work reliably)
 * Render graph can be used for efficient and flexible definition of a render pipeline
 * Auto-generated shader bindings make working with descriptor sets convenient and less error prone.
 * Material System supporting multiple passes
 * Multi-view support (to produce shadow maps, for example)
 * Demo game state stored in ECS (NOTE: demo uses legion but the renderer is ECS-agnostic)
 * PBR Meshes
 * Sprites
 * Debug Draw
 * imgui
 * HDR Pipeline with Bloom

## Running the Demo

```
git clone https://github.com/aclysma/renderer_prototype.git
cd renderer_prototype
cargo update -p tokio --precise 0.2.13
cargo run --release
```

([Tokio >= 0.2.14 hangs](https://github.com/tokio-rs/tokio/issues/2390))

Running in release reduces logging and disables vulkan validation. The first time it will load more slowly because it
has to import the assets, including a GLTF mesh with large textures. **Using profile overrides to optimize upstream crates
is highly recommeneded. Asset processing is extremely slow in debug mode.** (i.e. 30s instead of 2s)

The demo uses SDL2 and in debug mode, vulkan validation. If you have trouble running the demo, please check that
dependencies for both SDL2 and vulkan are available.

## Features

 * `renderer-shell-vulkan`, `renderer-shell-vulkan-sdl2` - Basic helpers for vulkan
   * Friendly helpers for setting up the device and window
   * Some basic, unopinionated helpers for vulkan. Things like async image uploads, deferring destruction of resources, 
     and pooling/reusing resources
 * `renderer-base` - Shared helpers/data structures. Nothing exciting
 * `renderer-nodes` - Inspired by the 2015 GDC talk "Destiny's Multithreaded Rendering Architecture." (A low-budget
   version and jobs are not actually MT yet)
   * A job system with extract, prepare, and write phases
   * Rendering is pipelined with simulation thread, and the job structure is intended to be highly parallel
   * Handles multiple views and phases allowing advanced features like shadow maps
   * Flexible sorting mechanism for interleaving and batching write commands from multiple rendering features
 * `renderer-visibility` - Placeholder visibility system. Doesn't do anything yet (returns all things visible all the 
   time). See the GDC talk for more info on how this will work.
 * `renderer-resources` - Resource management for images, buffers, descriptor sets, etc.
   * Most things are hashed and reference counted
   * Provides a render graph
   * Nearly all vulkan assets are data-driven from serializable and hashable structures rather than hard-coded.
   * Buffers and images are asynchronously uploaded on dedicated transfer queue when available
 * `renderer-assets` - An asset loading and management system.
   * Assets can hot reload from files (but see [#14](renderer_prototype/issues/14))
   * Because atelier-assets pre-processes and stores cached assets as they change, custom processing/packing can be
     implemented while maintaining extremely fast load times. For example, texture compression could be implemented
     as an import step.  
   * Separate multi-thread friendly path for creating assets at runtime
   * Multi-pass material abstraction with bindable parameters

Notably, this project does not support multiple rendering backends. This is something I want to get to eventually! I
also would prefer to work with other rendering APIs (like metal, dx12) directly rather than through a complete generic
abstraction layer like gfx-hal.

## Roadmap

 * Better shadows
 * More rendering techniques like SSAO
 * Support for more rendering backends (mainly metal and dx12)

The demo shows a basic rendering pipeline with a GLTF importer, PBR, bloom, imgui, debug draw, sprites, and dynamic
light/shadows. It also demonstrates how to pipeline rendering on a separate thread from simulation.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

The demo/fonts directory contains several fonts under their own licenses:
 * [Feather](https://github.com/AT-UI/feather-font), MIT
 * [Material Design Icons](https://materialdesignicons.com), SIL OFL 1.1
 * [FontAwesome 4.7.0](https://fontawesome.com/v4.7.0/license/), available under SIL OFL 1.1
 * [`mplus-1p-regular.ttf`](http://mplus-fonts.osdn.jp), available under its own license.

The assets/blender contains some shaders from from https://freepbr.com, available under [its own license](assets/blender/pbr_texture_attribution.txt)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
