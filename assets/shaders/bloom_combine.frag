#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (set = 0, binding = 0) uniform texture2D in_color;
layout (set = 0, binding = 1) uniform texture2D in_blur;
layout (set = 0, binding = 2) uniform sampler smp;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_sdr;

void main()
{
    vec4 color = texture(sampler2D(in_color, smp), inUV) + texture(sampler2D(in_blur, smp), inUV);

    // tonemapping
    vec3 mapped = color.rgb / (color.rgb + vec3(1.0));

    out_sdr = vec4(mapped, color.a);
}
