#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (set = 0, binding = 0) uniform texture2D tex;
layout (set = 0, binding = 1) uniform sampler smp;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_sdr;
layout (location = 1) out vec4 out_bloom;

void main()
{
    vec3 color = texture(sampler2D(tex, smp), inUV).rgb;

    if (dot(color, color) > 1.0f) {
        out_bloom = vec4(color, 1.0);
    } else {
        out_bloom = vec4(0.0, 0.0, 0.0, 1.0);
    }

    out_sdr = vec4(color, 1.0);
}
