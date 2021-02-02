# Recommended Practices

 * **IMPORTANT**: Place all resources defined in GLSL for the vertex and fragment stages together in a single .glsl file
   and #include it from both stages. This ensures the shader processor sees the same resources in each stage and
   generates consistent resource binding IDs in each stage.
 * Use descriptor sets 0-3. A compliant vulkan device only needs to support up to 4 bound descriptor sets
 * Bind descriptor sets based on frequency. For example, resources that are bound once per frame should be grouped
   together in one set, and resource that change every draw call should be grouped in another set.
 * If you dislike the name or location of type in the generated rust code, you can use the rust `type` keyword to
   "re-export" the type from a different location with the name of your choice.
 * Use `cargo install rafx-shader-processor --version x.y.z` to install the version of `rafx-shader-processor` that
   matches the version of `rafx` you are building with. Don't forget to update this if you update the dependent version!
   
