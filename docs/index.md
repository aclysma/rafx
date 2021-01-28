
# Topics

* API
    * [Rendering Concepts](api/rendering_concepts.md)
    * [API Design](api/api_design.md)
    * [Safety](api/safety.md)
    * [Windowing and Swapchain Handling](api/windowing_and_swapchains.md)
    * [Resource Binding Model](api/resource_binding_model.md)
    * [Validation and Debugging](api/validation_and_debugging.md)
    * [API Triangle Example](../rafx/examples/api_triangle/api_triangle.rs)
* Framework
    * [Framework Architecture](framework/framework_architecture.md)
    * Adding Features
    * Adding Phases
    * Render Graph
    * [Render Graph Triangle Example](../rafx/examples/render_graph_triangle/render_graph_triangle.rs)
* Assets
    * `distill` Architecture and Features
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
    * [Render Graph Triangle Example](../rafx/examples/render_graph_triangle/render_graph_triangle.rs)
* Backend Implementation Details
    * Metal
    * Vulkan
* [Acknowledgements and Other Resources](acknowledgements.md)