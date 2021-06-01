
# Documentation

## Start Here

 * [Why Rafx?](why_rafx.md)
 * [API Design in Rust Pseudocode](api/api_design_in_rust_psuedocode.rs)
 * [API Triangle Example](../rafx/examples/api_triangle/api_triangle.rs)

## Topics

* [Why Rafx?](why_rafx.md)
* [Rendering Concepts](api/rendering_concepts.md) (A gentle introduction to GPU rendering - not specific to rafx)
* rafx-api
    * [API Design](api/api_design.md)
    * [API Design in Rust Pseudocode](api/api_design_in_rust_psuedocode.rs)  
    * [Safety](api/safety.md)
    * [Windowing and Swapchain Handling](api/windowing_and_swapchains.md)
    * [Resource Binding Model](api/resource_binding_model.md)
    * [Validation and Debugging](api/validation_and_debugging.md)
    * [API Triangle Example](../rafx/examples/api_triangle/api_triangle.rs)
    * Backend Implementation Details:
        * [Vulkan](api/backends/vulkan.md)
        * [Metal](api/backends/metal.md)
        * [GL ES 2.0](api/backends/gles2.md)
        * [GL ES 3.0](api/backends/gles3.md)
* rafx-visibility
    * [API Design](visibility/api_design.md)
* rafx-framework
    * [Framework Architecture](framework/framework_architecture.md)
    * [Adding Features](framework/adding_features.md)
    * [Adding Phases](framework/adding_render_phases.md)
    * [Render Graph](framework/render_graph.md)
    * [Render Graph Triangle Example](../rafx/examples/framework_triangle/framework_triangle.rs)
    * [Visibility Region](framework/visibility_region.md)
* rafx-assets
    * `distill` Architecture and Features
    * [Asset Triangle Example](../rafx/examples/asset_triangle/asset_triangle.rs)
* rafx-renderer
    * [Renderer Architecture](renderer/renderer_architecture.md)
    * [Renderer Triangle Example](../rafx/examples/renderer_triangle/renderer_triangle.rs)
* Shader Authoring with `rafx-shader-processor`
    * [Shader Processor](shaders/shader_processor.md)
    * [Custom Shader Markup](shaders/shader_annotation.md)
        * [@[export]](shaders/shader_annotation.md#export)
        * [@[immutable_samplers]](shaders/shader_annotation.md#immutable_samplers)
        * [@[internal_buffer]](shaders/shader_annotation.md#internal_buffer)
        * [@[semantic]](shaders/shader_annotation.md#semantic)
        * [@[slot_name]](shaders/shader_annotation.md#slot_name)
    * [Conversion from GLSL to MSL](shaders/glsl_to_msl.md)
    * [Using the Generated Rust Code](shaders/generated_rust_code.md)
    * [Recommended Practices](shaders/recommended_practices.md)
* Other Tools
    * Asset Packing
    * Profiling
* Examples
    * [API Triangle Example](../rafx/examples/api_triangle/api_triangle.rs)
    * [Render Graph Triangle Example](../rafx/examples/framework_triangle/framework_triangle.rs)
    * [Asset Triangle Example](../rafx/examples/asset_triangle/asset_triangle.rs)
* [Building for iOS](building_for_ios.md)
* [Acknowledgements and Other Resources](acknowledgements.md)

## Diagrams

* [Key crate dependencies](images/crate_dependencies.png)
* [Pipelining](images/pipelining.png)
* [Shader Processor](images/shader_processor.png)
* [Visibility region](images/visibility_region.png)
* [Extract, prepare, write](images/extract_prepare_write.png)

## Other Resources

More complete list here: [Acknowledgements and Other Resources](acknowledgements.md)

* The job/phase rendering design is inspired by the 2015 GDC talk "[Destiny's Multithreaded Rendering Architecture](http://advances.realtimerendering.com/destiny/gdc_2015/Tatarchuk_GDC_2015__Destiny_Renderer_web.pdf)".
* The render graph is inspired by the 2017 GDC talk "[FrameGraph: Extensible Rendering Architecture in Frostbite](https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-Rendering-Architecture-in)"
  * see also "[Render Graphs and Vulkan - a deep dive](http://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/)" for an implementation of a render graph on vulkan
* The low-level API is somewhat similar to the API from "[The Forge](https://github.com/ConfettiFX/The-Forge)"
