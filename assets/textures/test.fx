{
//TODO
// - The material will map a shader/PSO to a phase
// - We should know what phases we will call per renderpass
// - Given this, we should know which renderpasses a material can be drawn in and generate PSOs
// - When a material loads it needs to find the descriptor sets for the PSOs and bind textures

    "descriptor_set_layouts": [
        // set 0
        {
            "descriptor_set_layout_bindings": [
                {
                    "binding": 0,
                    "descriptor_type": "UNIFORM_BUFFER",
                    "descriptor_count": 1,
                    "stage_flags": "VERTEX"
                },
                {
                    "binding": 1,
                    "descriptor_type": "SAMPLER",
                    "descriptor_count": 1,
                    "stage_flags": "FRAGMENT"
                },
            ]
        },

        // set 1
        {
            "descriptor_set_layout_bindings": [
                {
                    "binding": 0,
                    "descriptor_type": "UNIFORM_BUFFER",
                    "descriptor_count": 1,
                    "stage_flags": "VERTEX"
                },
                {
                    "binding": 1,
                    "descriptor_type": "SAMPLER",
                    "descriptor_count": 1,
                    "stage_flags": "FRAGMENT"
                },
            ]
        }
    ],
    "pipeline_layouts": [
        // PIPELINE LAYOUT
        // - Descriptor set layouts
        // - Push constant ranges
        //
        //
        // The PSO state required to define a pipeline layout
        //
        // FIXED FUNCTION STATE
        // - PipelineInputAssemblyState
        //   - topology i.e. triangle list
        // - PipelineVertexInputState
        //   - [vk::VertexInputBindingDescription]
        //   - [vk::VertexInputAttributeDescription]
        // - PipelineViewportState
        //   - [viewports]
        //   - [scissors]
        // - PipelineRasterizationState
        //   - backface culling (i.e. CW vs CCW)
        //   - fill
        // - PipelineMultisampleState
        // - PipelineColorBlendState
        //   - alpha blending
        // - PipelineDynamicState
        //   - opt-in to some of the above being dynamic
        //
        // RENDERPASS
        // - List of attachments
        //   - format
        //   - multisample
        //   - load/store ops
        //   - init/final layouts
        // - List of subpasses
        // - List of dependencies between subpasses
        //
        // 
    ],
    "passes", [
        //
    ]
    "pipelines": [
        // The PSO state required to define a pipeline instance
    ],
    "materials": [
        // Mappings of phases to pipelines
        // shaders, things to bind to the shader
    ],
    "": [

    ]
}