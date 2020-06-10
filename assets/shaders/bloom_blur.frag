#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (set = 0, binding = 0) uniform texture2D tex;
layout (set = 0, binding = 1) uniform sampler smp;
layout (set = 0, binding = 2) uniform Config {
    bool horizontal;
} config;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_blur;

void main()
{
    if (config.horizontal) {
        out_blur = texture(sampler2D(tex, smp), inUV);
    } else {
        out_blur = texture(sampler2D(tex, smp), inUV);
    }
}
