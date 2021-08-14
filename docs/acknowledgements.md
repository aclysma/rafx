# Acknowledgements

## References

Rafx benefits from many great ideas from other people!

* The job/phase rendering design is inspired by the 2015 GDC talk "[Destiny's Multithreaded Rendering Architecture](http://advances.realtimerendering.com/destiny/gdc_2015/Tatarchuk_GDC_2015__Destiny_Renderer_web.pdf)".
* The render graph is inspired by the 2017 GDC talk "[FrameGraph: Extensible Rendering Architecture in Frostbite](https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-Rendering-Architecture-in)"
    * see also "[Render Graphs and Vulkan - a deep dive](http://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/)" for an implementation of a render graph on vulkan
* The low-level API is somewhat similar to the API from "[The Forge](https://github.com/ConfettiFX/The-Forge)"

## Dependencies

Rafx also benefits from many excellent crates - in particular bindings to native graphics APIs. (This is not a complete
list of dependencies, just ones of particular note for graphics programming).

General:
* `raw-window-handle`: https://github.com/rust-windowing/raw-window-handle

Vulkan:
 * `ash` and `ash-window`: https://github.com/MaikKlein/ash
 * `gpu-allocator`: https://github.com/Traverse-Research/gpu-allocator
   
Metal:
 * `metal`: https://github.com/gfx-rs/metal-rs
 * `raw-window-metal`: https://github.com/norse-rs/raw-window-metal
 * `objc`: https://github.com/SSheldon/rust-objc
 * `dispatch`: https://github.com/SSheldon/rust-dispatch
 * `cocoa-foundation`: https://github.com/servo/core-foundation-rs
 * `block`: https://github.com/SSheldon/rust-block

Shader Processing
 * `spirv-cross`: https://github.com/KhronosGroup/SPIRV-Cross
     * bindings: https://github.com/grovesNL/spirv_cross
 * `shaderc`: https://github.com/google/shaderc
     * bindings: https://crates.io/crates/shaderc

Windowing:
 * `sdl2`: https://www.libsdl.org
     * bindings: https://crates.io/crates/sdl2
 * `winit`: https://github.com/rust-windowing/winit
 

## Other Projects

 * `gfx-hal`: https://github.com/gfx-rs/gfx
     * While rafx does not use gfx-hal, the rust ecosystem is in a much better place for having it, and many of the
       crates that rafx does depend on are maintained by gfx-hal contributors
 * MoltenVK: https://github.com/KhronosGroup/MoltenVK
     * rafx has a native backend for metal and does not require MoltenVK, however rafx benefits from much of the
       pathfinding they have done, much of which has been rolled into projects like `spirv-cross`

## Rust and the Rust Community

The rust ecosystem has many options for low- and high-level graphics programming, many of which are listed at
https://arewegameyet.rs 

Finally, the rust ecosystem is built and maintained by many brilliant and NICE people in the community. To everyone who
has or will play a part in making the community and ecosystem a better place for everyone else, THANK YOU!!
