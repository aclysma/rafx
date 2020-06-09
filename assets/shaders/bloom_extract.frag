#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (set = 0, binding = 0) uniform texture2D tex;
layout (set = 0, binding = 1) uniform sampler smp;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_sdr;
layout (location = 1) out vec4 out_bloom;

void main()
{
    vec4 color = texture(sampler2D(tex, smp), inUV);

    // tonemapping
    vec3 mapped = color.rgb / (color.rgb + vec3(1.0));

    if (dot(mapped, mapped) > 1.0f) {
        out_bloom = vec4(mapped, 1.0);
    } else {
        out_bloom = vec4(0.0, 0.0, 0.0, 1.0);
    }

    out_sdr = vec4(mapped, color.a);
}

//TODO: Add a bloom pass - need to take resolved image and write it out to two buffers, then sample for the blur,
// then sample the original + blurred